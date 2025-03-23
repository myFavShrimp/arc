use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use log::info;
use mlua::{Lua, LuaOptions, StdLib};
use modules::Modules;
use state::{State, TasksResultResetError, TasksResultStateSetError};
use system::{
    executor::{ExecutionTargetSetError, Executor},
    System,
};

use crate::{
    error::MutexLockError,
    memory::{
        target_groups::TargetGroupsMemory, target_systems::TargetSystemsMemory, tasks::TasksMemory,
    },
};

pub mod modules;
pub mod state;
pub mod system;

pub struct Engine {
    lua: Lua,
    state: State,
    is_dry_run: bool,
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to create engine")]
pub enum EngineBuilderCreationError {
    Lua(#[from] mlua::Error),
}

static ENTRY_POINT_SCRIPT: &str = "arc.lua";

#[derive(thiserror::Error, Debug)]
#[error("Failed to run scripts")]
pub enum EngineExecutionError {
    Io(#[from] std::io::Error),
    Lua(#[from] mlua::Error),
    ExecutionTargetSet(#[from] ExecutionTargetSetError),
    FilteredGroupDoesNotExistError(#[from] FilteredGroupDoesNotExistError),
    Lock(#[from] MutexLockError),
    TasksResultResetError(#[from] TasksResultResetError),
    TasksResultSet(#[from] TasksResultStateSetError),
}

#[derive(Debug, thiserror::Error)]
#[error("The filtered group {0:?} does not exist")]
pub struct FilteredGroupDoesNotExistError(Vec<String>);

impl Engine {
    pub fn new(
        root_directory: PathBuf,
        is_dry_run: bool,
    ) -> Result<Self, EngineBuilderCreationError> {
        let mut lua = Lua::new_with(StdLib::ALL_SAFE, LuaOptions::new().catch_rust_panics(true))?;

        let target_systems_memory = Arc::new(Mutex::new(TargetSystemsMemory::default()));
        let target_groups_memory = Arc::new(Mutex::new(TargetGroupsMemory::default()));
        let tasks_memory = Arc::new(Mutex::new(TasksMemory::default()));

        Modules::new(
            target_systems_memory.clone(),
            target_groups_memory.clone(),
            tasks_memory.clone(),
            root_directory,
        )
        .mount_to_globals(&mut lua)?;

        Ok(Self {
            lua,
            state: State::new(target_systems_memory, target_groups_memory, tasks_memory),
            is_dry_run,
        })
    }

    pub fn execute(
        &self,
        tags: Vec<String>,
        groups: Vec<String>,
    ) -> Result<(), EngineExecutionError> {
        let entry_point_script_path = PathBuf::from(ENTRY_POINT_SCRIPT);
        let entry_point_script = std::fs::read_to_string(&entry_point_script_path)?;

        self.lua
            .load(entry_point_script)
            .set_name(entry_point_script_path.to_string_lossy())
            .exec()?;

        let systems = self.state.systems_for_selected_groups(&groups)?;
        let tasks = self
            .state
            .tasks_for_selected_groups_and_tags(&groups, &tags)?;

        let mut tasks_to_execute = tasks.values().cloned().collect::<Vec<_>>();
        // TODO: this is not correct in all cases
        tasks_to_execute.sort_unstable_by(|a, b| a.partial_cmp(b).expect("ordering is not none"));

        let missing_selected_groups = self.state.missing_selected_groups(&groups)?;
        if !missing_selected_groups.is_empty() {
            Err(FilteredGroupDoesNotExistError(
                missing_selected_groups.clone(),
            ))?
        }

        let selected_groups = self.state.selected_groups(&groups)?;

        if self.is_dry_run {
            info!("Starting dry run ...");
        }

        for (system_name, system_config) in systems {
            let system_groups = selected_groups
                .iter()
                .filter(|(_, config)| config.members.contains(&system_name))
                .map(|(name, _)| name)
                .collect::<Vec<&String>>();
            let system_tasks = tasks_to_execute
                .iter()
                .filter(|task| {
                    system_groups.is_empty()
                        || task.groups.is_empty()
                        || task
                            .groups
                            .iter()
                            .any(|group| system_groups.contains(&group))
                })
                .collect::<Vec<_>>();

            info!("Processing target {:?}", system_name);

            if system_tasks.is_empty() {
                info!("No tasks to execute for target {:?}", system_name);
                continue;
            }

            self.state.reset_task_results()?;

            let system = System {
                name: system_config.name.clone(),
                address: system_config.address,
                port: system_config.port,
                user: system_config.user.clone(),
                execution_delegator: Executor::new_for_system(&system_config, self.is_dry_run)?,
            };

            for task_config in system_tasks {
                info!("Executing `{}` for {}", task_config.name, system_name);

                let result = task_config.handler.call::<mlua::Value>(system.clone())?;
                self.state.set_task_result(&task_config.name, result)?;
            }
        }

        Ok(())
    }
}

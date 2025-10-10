use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use delegator::{
    executor::{ExecutionTargetSetError, Executor},
    operator::{FileSystemOperator, OperationTargetSetError},
};
use mlua::{Lua, LuaOptions, StdLib};
use modules::{Modules, MountToGlobals};
use objects::system::System;
use recipe::{Recipe, RecipeCreationError};
use state::{State, TasksResultResetError, TasksResultStateSetError};

use crate::{
    engine::objects::system::SystemKind,
    error::MutexLockError,
    logger::{Logger, SharedLogger},
    memory::{
        target_groups::TargetGroupsMemory,
        target_systems::{TargetSystemKind, TargetSystemsMemory},
        tasks::TasksMemory,
    },
};

pub mod delegator;
pub mod modules;
pub mod objects;
mod readonly;
mod recipe;
pub mod state;

pub struct Engine {
    lua: Lua,
    state: State,
    is_dry_run: bool,
    logger: SharedLogger,
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
    OperationTargetSet(#[from] OperationTargetSetError),
    FilteredGroupDoesNotExistError(#[from] FilteredGroupDoesNotExistError),
    Lock(#[from] MutexLockError),
    TasksResultResetError(#[from] TasksResultResetError),
    TasksResultSet(#[from] TasksResultStateSetError),
    RecipeCreation(#[from] RecipeCreationError),
}

#[derive(Debug, thiserror::Error)]
#[error("The filtered group {0:?} does not exist")]
pub struct FilteredGroupDoesNotExistError(Vec<String>);

impl Engine {
    pub fn new(logger: Logger, is_dry_run: bool) -> Result<Self, EngineBuilderCreationError> {
        let logger = Arc::new(Mutex::new(logger));
        let mut lua = Lua::new_with(StdLib::ALL_SAFE, LuaOptions::new().catch_rust_panics(true))?;

        let target_systems_memory = Arc::new(Mutex::new(TargetSystemsMemory::default()));
        let target_groups_memory = Arc::new(Mutex::new(TargetGroupsMemory::default()));
        #[allow(clippy::arc_with_non_send_sync)]
        let tasks_memory = Arc::new(Mutex::new(TasksMemory::default()));

        Modules::new(
            target_systems_memory.clone(),
            target_groups_memory.clone(),
            tasks_memory.clone(),
            logger.clone(),
        )
        .mount_to_globals(&mut lua)?;

        Ok(Self {
            lua,
            state: State::new(target_systems_memory, target_groups_memory, tasks_memory),
            is_dry_run,
            logger,
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

        let tasks_to_execute = tasks.values().cloned().collect::<Vec<_>>();
        let recipe = Recipe::from_tasks(&tasks_to_execute)?;

        let missing_selected_groups = self.state.missing_selected_groups(&groups)?;
        if !missing_selected_groups.is_empty() {
            Err(FilteredGroupDoesNotExistError(
                missing_selected_groups.clone(),
            ))?
        }

        let selected_groups = self.state.selected_groups(&groups)?;

        if self.is_dry_run {
            let logger = self.logger.lock().unwrap();
            logger.info("Starting dry run ...");
            drop(logger);
        }

        for (system_name, system_config) in systems {
            let system_groups = selected_groups
                .iter()
                .filter(|(_, config)| config.members.contains(&system_name))
                .map(|(name, _)| name)
                .collect::<Vec<&String>>();
            let system_tasks = recipe
                .tasks
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

            let mut logger = self.logger.lock().unwrap();
            logger.current_system(&system_name);
            drop(logger);

            if system_tasks.is_empty() {
                continue;
            }

            self.state.reset_task_results()?;

            let system = System {
                name: system_config.name.clone(),
                kind: match &system_config.kind {
                    TargetSystemKind::Remote(remote_target_system) => {
                        SystemKind::Remote(objects::system::RemoteSystem {
                            address: remote_target_system.address,
                            port: remote_target_system.port,
                            user: remote_target_system.user.clone(),
                            executor: Executor::new_for_system(&system_config, self.is_dry_run)?,
                            file_system_operator: FileSystemOperator::new_for_system(
                                &system_config,
                                self.is_dry_run,
                            )?,
                        })
                    }
                    TargetSystemKind::Local => {
                        SystemKind::Local(Executor::new_local(), FileSystemOperator::new_local())
                    }
                },
            };

            for task_config in system_tasks {
                let result = task_config.handler.call::<mlua::Value>(system.clone())?;
                self.state.set_task_result(&task_config.name, result)?;
            }

            let mut logger = self.logger.lock().unwrap();
            logger.reset_system();
        }

        Ok(())
    }
}

use std::path::PathBuf;

use file_system::FileSystem;
use log::info;
use mlua::{Lua, LuaOptions, StdLib};
use system::{ExecutionTargetSetError, System};
use targets::TargetsValidationError;
use tasks::TasksValidationError;
use {
    targets::TargetsAcquisitionError,
    tasks::{TasksAcquisitionError, TasksResultResetError, TasksResultSetError},
};

use templates::{TemplateRenderError, Templates};
use {targets::Targets, tasks::Tasks};

pub mod file_system;
pub mod system;
pub mod targets;
pub mod tasks;
pub mod templates;

pub struct Engine {
    lua: Lua,
    targets: Targets,
    tasks: Tasks,
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to create engine")]
pub enum EngineBuilderCreationError {
    Lua(#[from] mlua::Error),
    Templates(#[from] TemplateRenderError),
}

static ENTRY_POINT_SCRIPT: &str = "arc.lua";

#[derive(thiserror::Error, Debug)]
#[error("Failed to run scripts")]
pub enum EngineExecutionError {
    Io(#[from] std::io::Error),
    Lua(#[from] mlua::Error),
    ExecutionTargetSet(#[from] ExecutionTargetSetError),
    TargetsAcquisition(#[from] TargetsAcquisitionError),
    TasksAcquisition(#[from] TasksAcquisitionError),
    TasksResultReset(#[from] TasksResultResetError),
    TasksResultSet(#[from] TasksResultSetError),
    TargetsValidation(#[from] TargetsValidationError),
    TasksValidation(#[from] TasksValidationError),
    Templates(#[from] TemplateRenderError),
    FilteredGroupDoesNotExistError(#[from] FilteredGroupDoesNotExistError),
}

#[derive(Debug, thiserror::Error)]
#[error("The filtered group {0:?} does not exist")]
pub struct FilteredGroupDoesNotExistError(Vec<String>);

impl Engine {
    pub fn new(root_directory: PathBuf) -> Result<Self, EngineBuilderCreationError> {
        let lua = Lua::new_with(StdLib::ALL_SAFE, LuaOptions::new().catch_rust_panics(true))?;

        let targets = Targets::default();
        let tasks = Tasks::default();
        let templates = Templates::new();
        let file_system = FileSystem::new(root_directory);

        let globals = lua.globals();
        globals.set("targets", targets.clone())?;
        globals.set("tasks", tasks.clone())?;
        globals.set("template", templates)?;
        globals.set("fs", file_system)?;

        Ok(Self {
            lua,
            targets,
            tasks,
        })
    }

    pub fn execute(
        &self,
        tags: Vec<String>,
        mut groups: Vec<String>,
    ) -> Result<(), EngineExecutionError> {
        let entry_point_script_path = PathBuf::from(ENTRY_POINT_SCRIPT);
        let entry_point_script = std::fs::read_to_string(&entry_point_script_path)?;

        self.lua
            .load(entry_point_script)
            .set_name(entry_point_script_path.to_string_lossy())
            .exec()?;

        self.targets.validate()?;
        let (mut systems, group_configs) = self.targets.targets()?;
        self.tasks.validate(&group_configs)?;

        let mut filtered_group_configs = group_configs.clone();
        filtered_group_configs.retain(|name, _| groups.is_empty() || groups.contains(name));
        systems.retain(|name, _| {
            filtered_group_configs
                .iter()
                .any(|(_, group)| group.members.contains(name))
                || !group_configs
                    .iter()
                    .any(|(_, group)| group.members.contains(name))
        });

        groups.retain(|name| !filtered_group_configs.contains_key(name));
        if !groups.is_empty() {
            Err(FilteredGroupDoesNotExistError(groups))?
        }

        let tasks = self.tasks.filtered_tasks_in_execution_order(&tags)?;

        for (system_name, system_config) in systems {
            info!("Processing target {:?}", system_name);

            if tasks.is_empty() {
                info!("No tasks to execute for target {:?}", system_name);
                continue;
            }

            self.tasks.reset_results()?;

            let system = System::connect(&system_config)?;

            for task_config in &tasks {
                info!("Executing `{}` for {}", task_config.name, system_name);

                let result = task_config.handler.call::<mlua::Value>(system.clone())?;
                self.tasks.set_task_result(&task_config.name, result)?;
            }
        }

        Ok(())
    }
}

use std::path::PathBuf;

use log::info;
use mlua::{Lua, LuaOptions, StdLib};
use system::{ExecutionTargetSetError, System};
use targets::TargetsValidationError;
use {
    targets::TargetsAcquisitionError,
    tasks::{TasksAcquisitionError, TasksResultResetError, TasksResultSetError},
};

use {targets::Targets, tasks::Task, tasks::Tasks};

pub mod system;
pub mod targets;
pub mod tasks;

pub struct Engine {
    lua: Lua,
    targets: Targets,
    tasks: Tasks,
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
    TargetsAcquisition(#[from] TargetsAcquisitionError),
    TasksAcquisition(#[from] TasksAcquisitionError),
    TasksResultReset(#[from] TasksResultResetError),
    TasksResultSet(#[from] TasksResultSetError),
    TargetsValidation(#[from] TargetsValidationError),
}

impl Engine {
    pub fn new() -> Result<Self, EngineBuilderCreationError> {
        let lua = Lua::new_with(StdLib::ALL_SAFE, LuaOptions::new().catch_rust_panics(true))?;

        let targets = Targets::default();
        let tasks = Tasks::default();

        let globals = lua.globals();
        globals.set("targets", targets.clone())?;
        globals.set("tasks", tasks.clone())?;

        Ok(Self {
            lua,
            targets,
            tasks,
        })
    }

    pub fn execute(&self, tags: Vec<String>) -> Result<(), EngineExecutionError> {
        let entry_point_script_path = PathBuf::from(ENTRY_POINT_SCRIPT);

        let entry_point_script = std::fs::read_to_string(&entry_point_script_path)?;

        self.lua
            .load(entry_point_script)
            .set_name(entry_point_script_path.to_string_lossy())
            .exec()?;

        self.targets.validate()?;
        let targets = self.targets.targets()?;

        for (system_name, system_config) in &targets.0 {
            info!("Processing target {:?}", system_name);

            let mut tasks = self.tasks.tasks_in_execution_order()?;

            if !tags.is_empty() {
                tasks = tasks
                    .into_iter()
                    .filter(|config| {
                        config
                            .tags
                            .iter()
                            .any(|config_tag| tags.contains(config_tag))
                    })
                    .collect::<Vec<Task>>();
            }

            if tasks.is_empty() {
                info!("No tasks to execute for target {:?}", system_name);
                return Ok(());
            }

            self.tasks.reset_results()?;

            let system = System::connect(system_config)?;

            for task_config in tasks {
                info!("Executing `{}` for {}", task_config.name, system_name);

                let result = task_config.handler.call::<mlua::Value>(system.clone())?;
                self.tasks.set_task_result(task_config.name, result)?;
            }
        }

        Ok(())
    }
}

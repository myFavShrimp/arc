use std::{path::PathBuf, sync::Arc};

use log::info;
use mlua::{Lua, LuaOptions, StdLib};
use modules::{
    operations::ExecutionTargetSetError,
    targets::TargetsAcquisitionError,
    tasks::{TasksAcquisitionError, TasksResultResetError, TasksResultSetError},
    ModuleRegistrationError,
};
use system::System;

use crate::{
    engine::modules::targets::TargetsModule, engine::modules::tasks::TaskConfig,
    engine::modules::tasks::TasksModule, operations::OperationsExecutionModule,
};

pub mod modules;
pub mod system;

pub struct Engine {
    lua: Lua,
    modules: modules::Modules,
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to create engine")]
pub enum EngineBuilderCreationError {
    Lua(#[from] mlua::Error),
    ModuleRegistration(#[from] ModuleRegistrationError),
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
}

impl Engine {
    pub fn new() -> Result<Self, EngineBuilderCreationError> {
        let lua = Lua::new_with(StdLib::ALL_SAFE, LuaOptions::new().catch_rust_panics(true))?;

        let modules = modules::Modules {
            targets: TargetsModule::default(),
            tasks: TasksModule::default(),
            operations: Arc::new(OperationsExecutionModule::default()),
        };
        modules.register_in_lua(&lua)?;

        Ok(Self { lua, modules })
    }

    pub fn execute(&self, tags: Vec<String>) -> Result<(), EngineExecutionError> {
        let entry_point_script_path = PathBuf::from(ENTRY_POINT_SCRIPT);

        let entry_point_script = std::fs::read_to_string(&entry_point_script_path)?;

        self.lua
            .load(entry_point_script)
            .set_name(entry_point_script_path.to_string_lossy())
            .exec()?;

        let targets = self.modules.targets.targets()?;

        for (system_name, system_config) in &targets.systems {
            info!("Processing target {:?}", system_name);

            let mut tasks = self.modules.tasks.tasks()?.tasks_in_execution_order();

            if !tags.is_empty() {
                tasks = tasks
                    .into_iter()
                    .filter(|config| {
                        config
                            .tags
                            .iter()
                            .any(|config_tag| tags.contains(config_tag))
                    })
                    .collect::<Vec<TaskConfig>>();
            }

            if tasks.is_empty() {
                info!("No tasks to execute for target {:?}", system_name);
                return Ok(());
            }

            self.modules.tasks.reset_results()?;
            // self.modules
            //     .operations
            //     .set_execution_target(system_config)?;

            let system = System::connect(system_config)?;

            for task_config in tasks {
                info!("Executing `{}` for {}", task_config.name, system_name);

                task_config.handler.call::<mlua::Value>(system.clone())?;
                // self.modules
                //     .tasks
                //     .set_task_result(task_config.name, result)?;
            }
        }

        Ok(())
    }
}

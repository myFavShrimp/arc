use std::{path::PathBuf, sync::Arc};

use log::info;
use mlua::{Lua, LuaOptions, LuaSerdeExt, StdLib};
use modules::{
    ExecutionTargetSetError, InventoryAcquisitionError, ModuleRegistrationError,
    TasksAcquisitionError, TasksResultResetError, TasksResultSetError,
};

use crate::{
    inventory::InventoryRegistrationModule, operations::OperationsExecutionModule,
    tasks::TaskRegistrationModule,
};

pub mod modules;

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

static ENTRY_POINT_SCRIPT: &str = "init.lua";

#[derive(thiserror::Error, Debug)]
#[error("Failed to run scripts")]
pub enum EngineExecutionError {
    Io(#[from] std::io::Error),
    Lua(#[from] mlua::Error),
    ExecutionTargetSet(#[from] ExecutionTargetSetError),
    InventoryAcquisition(#[from] InventoryAcquisitionError),
    TasksAcquisition(#[from] TasksAcquisitionError),
    TasksResultReset(#[from] TasksResultResetError),
    TasksResultSet(#[from] TasksResultSetError),
}

impl Engine {
    pub fn new() -> Result<Self, EngineBuilderCreationError> {
        let lua = Lua::new_with(StdLib::ALL_SAFE, LuaOptions::new().catch_rust_panics(true))?;

        let modules = modules::Modules {
            inventory: Arc::new(InventoryRegistrationModule::default()),
            tasks: Arc::new(TaskRegistrationModule::default()),
            operations: Arc::new(OperationsExecutionModule::default()),
        };
        modules.register_in_lua(&lua)?;

        Ok(Self { lua, modules })
    }

    pub fn execute(&self) -> Result<(), EngineExecutionError> {
        let inventory_entry_point_script_path = PathBuf::from(ENTRY_POINT_SCRIPT);

        let inventory_entry_point_script =
            std::fs::read_to_string(&inventory_entry_point_script_path)?;

        self.lua
            .load(inventory_entry_point_script)
            .set_name(inventory_entry_point_script_path.to_string_lossy())
            .exec()?;

        let inventory = self.modules.inventory.inventory()?;

        for (system_name, system_config) in &inventory.systems {
            info!("Processing system {}", system_name);

            self.modules.tasks.reset_results()?;
            self.modules
                .operations
                .set_execution_target(system_config)?;

            for task_config in dbg!(self.modules.tasks.tasks()?.tasks_in_execution_order()) {
                info!("Executing `{}` for {}", task_config.name, system_name);

                let result = task_config
                    .handler
                    .call::<mlua::Value>(self.lua.to_value(system_config)?)?;
                self.modules
                    .tasks
                    .set_task_result(task_config.name, result)?;
            }
        }

        Ok(())
    }
}

use std::{path::PathBuf, sync::Arc};

use mlua::{Lua, LuaSerdeExt};
use operations::OperationsModule;
use targets::{GroupConfig, SystemConfig, TargetsModule};
use tasks::TasksModule;

use crate::{engine::modules::tasks::TaskConfig, error::ErrorReport};

pub mod operations;
pub mod targets;
pub mod tasks;

pub struct Modules {
    pub targets: TargetsModule,
    pub tasks: Arc<dyn TasksModule>,
    pub operations: Arc<dyn OperationsModule>,
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to provide functions to lua venv")]
pub struct ModuleRegistrationError(#[from] mlua::Error);

impl Modules {
    pub fn register_in_lua(&self, lua: &Lua) -> Result<(), ModuleRegistrationError> {
        let globals = lua.globals();
        let targets_table = lua.create_table()?;

        {
            let targets = self.targets.clone();
            targets_table.set(
                "add_system",
                lua.create_function(
                    move |lua, (system_name, system): (mlua::String, mlua::Table)| {
                        let system_name = system_name.to_str()?;
                        let system: SystemConfig = lua.from_value(mlua::Value::Table(system))?;

                        targets
                            .add_system(system_name.to_string(), system)
                            .map_err(|e| {
                                mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report())
                            })?;

                        Ok(mlua::Value::Nil)
                    },
                )?,
            )?;
        }

        {
            let targets = self.targets.clone();
            targets_table.set(
                "add_group",
                lua.create_function(
                    move |lua, (group_name, group): (mlua::String, mlua::Table)| {
                        let group_name = group_name.to_str()?;
                        let group: GroupConfig = lua.from_value(mlua::Value::Table(group))?;

                        targets
                            .add_group(group_name.to_string(), group)
                            .map_err(|e| {
                                mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report())
                            })?;

                        Ok(mlua::Value::Nil)
                    },
                )?,
            )?;
        }

        globals.set("targets", targets_table)?;
        let tasks_table = lua.create_table()?;

        {
            let tasks = self.tasks.clone();
            tasks_table.set(
                "add",
                lua.create_function(
                    move |_lua, (task_name, config): (mlua::String, mlua::Value)| {
                        let task_name = task_name.to_str()?;
                        let config = TaskConfig::try_from((task_name.to_string(), config))
                            .map_err(|e| {
                                mlua::Error::runtime(ErrorReport::boxed_from(e).report())
                            })?;

                        tasks.add_task(task_name.to_string(), config).map_err(|e| {
                            mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report())
                        })?;

                        Ok(mlua::Value::Nil)
                    },
                )?,
            )?;
        }

        {
            let tasks = self.tasks.clone();
            tasks_table.set(
                "get_result",
                lua.create_function(move |_lua, (task_name,): (mlua::String,)| {
                    let task_name = task_name.to_str()?;

                    let task_result = tasks.task_result(&task_name.to_string()).map_err(|e| {
                        mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report())
                    })?;

                    Ok(task_result)
                })?,
            )?;
        }

        globals.set("tasks", tasks_table)?;
        let operations_table = lua.create_table()?;

        {
            let operations = self.operations.clone();
            operations_table.set(
                "run_command",
                lua.create_function(move |lua, (command,): (mlua::String,)| {
                    let command = command.to_str()?;

                    let command_result =
                        operations.run_command(command.to_string()).map_err(|e| {
                            mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report())
                        })?;

                    Ok(lua.to_value(&command_result)?)
                })?,
            )?;
        }

        {
            let operations = self.operations.clone();
            operations_table.set(
                "copy_file",
                lua.create_function(move |lua, (src, dest): (mlua::String, mlua::String)| {
                    let src = src.to_str()?;
                    let dest = dest.to_str()?;

                    let copy_result = operations
                        .copy_file(
                            PathBuf::from(src.to_string()),
                            PathBuf::from(dest.to_string()),
                        )
                        .map_err(|e| {
                            mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report())
                        })?;

                    Ok(lua.to_value(&copy_result)?)
                })?,
            )?;
        }

        globals.set("operation", operations_table)?;

        Ok(())
    }
}

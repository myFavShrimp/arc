use std::{path::PathBuf, sync::Arc};

use mlua::{Lua, LuaSerdeExt};
use serde::Serialize;

use crate::{
    error::{ErrorReport, MutexLockError},
    inventory::{GroupConfig, Inventory, SystemConfig},
    ssh::{ConnectionError, SshError},
    tasks::{TaskConfig, Tasks},
};

pub struct Modules {
    pub inventory: Arc<dyn InventoryModule>,
    pub tasks: Arc<dyn TasksModule>,
    pub operations: Arc<dyn OperationsModule>,
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to provide functions to lua venv")]
pub struct ModuleRegistrationError(#[from] mlua::Error);

impl Modules {
    pub fn register_in_lua(&self, lua: &Lua) -> Result<(), ModuleRegistrationError> {
        let globals = lua.globals();
        let inventory_table = lua.create_table()?;

        {
            let inventory = self.inventory.clone();
            inventory_table.set(
                "add_system",
                lua.create_function(
                    move |lua, (system_name, system): (mlua::String, mlua::Table)| {
                        let system_name = system_name.to_str()?;
                        let system: SystemConfig = lua.from_value(mlua::Value::Table(system))?;

                        inventory
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
            let inventory = self.inventory.clone();
            inventory_table.set(
                "add_group",
                lua.create_function(
                    move |lua, (group_name, group): (mlua::String, mlua::Table)| {
                        let group_name = group_name.to_str()?;
                        let group: GroupConfig = lua.from_value(mlua::Value::Table(group))?;

                        inventory
                            .add_group(group_name.to_string(), group)
                            .map_err(|e| {
                                mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report())
                            })?;

                        Ok(mlua::Value::Nil)
                    },
                )?,
            )?;
        }

        globals.set("inventory", inventory_table)?;
        let tasks_table = lua.create_table()?;

        {
            let tasks = self.tasks.clone();
            tasks_table.set(
                "add",
                lua.create_function(
                    move |_lua, (task_name, config): (mlua::String, mlua::Value)| {
                        let task_name = task_name.to_str()?;
                        let config = TaskConfig::try_from(config).map_err(|e| {
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

#[derive(thiserror::Error, Debug)]
#[error("Failed to add system")]
pub enum SystemAdditionError {
    Lock(#[from] MutexLockError),
    DuplicateSystem(#[from] DuplicateSystemError),
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to add group")]
pub enum GroupAdditionError {
    Lock(#[from] MutexLockError),
    MissingGroupMembers(#[from] UnregisteredGroupMembersError),
    DuplicateGroup(#[from] DuplicateGroupError),
}

#[derive(Debug, thiserror::Error)]
#[error("Unregistered group members: {0:?}")]
pub struct UnregisteredGroupMembersError(pub Vec<String>);

#[derive(Debug, thiserror::Error)]
#[error("Failed to retrieve inventory configuration")]
pub enum InventoryAcquisitionError {
    Lock(#[from] MutexLockError),
}

#[derive(Debug, thiserror::Error)]
#[error("Duplicate system: {0:?}")]
pub struct DuplicateSystemError(pub String);

#[derive(Debug, thiserror::Error)]
#[error("Duplicate group: {0:?}")]
pub struct DuplicateGroupError(pub String);

pub trait InventoryModule {
    fn add_system(&self, name: String, config: SystemConfig) -> Result<(), SystemAdditionError>;
    fn add_group(&self, name: String, config: GroupConfig) -> Result<(), GroupAdditionError>;
    fn inventory(&self) -> Result<Inventory, InventoryAcquisitionError>;
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to add task `{task}`")]
pub struct TaskAdditionError {
    pub task: String,
    #[source]
    pub kind: TaskAdditionErrorKind,
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum TaskAdditionErrorKind {
    Lock(#[from] MutexLockError),
    UnregisteredDependencies(#[from] UnregisteredDependenciesError),
    DuplicateTask(#[from] DuplicateTaskError),
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to retrieve tasks configuration")]
pub enum TasksAcquisitionError {
    Lock(#[from] MutexLockError),
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to retrieve task's result")]
pub enum TasksResultRetrievalError {
    Lock(#[from] MutexLockError),
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to reset tasks results")]
pub enum TasksResultResetError {
    Lock(#[from] MutexLockError),
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to set task's result")]
pub enum TasksResultSetError {
    Lock(#[from] MutexLockError),
}

#[derive(Debug, thiserror::Error)]
#[error("Unregistered task dependencies: {0:?}")]
pub struct UnregisteredDependenciesError(pub Vec<String>);

#[derive(Debug, thiserror::Error)]
#[error("Duplicate task: {0:?}")]
pub struct DuplicateTaskError(pub String);

pub trait TasksModule {
    fn tasks(&self) -> Result<Tasks, TasksAcquisitionError>;
    fn add_task(&self, name: String, config: TaskConfig) -> Result<(), TaskAdditionError>;
    fn reset_results(&self) -> Result<(), TasksResultResetError>;
    fn task_result(&self, name: &str) -> Result<Option<mlua::Value>, TasksResultRetrievalError>;
    fn set_task_result(&self, name: String, value: mlua::Value) -> Result<(), TasksResultSetError>;
}

#[derive(Debug, Serialize)]
pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

#[derive(Debug, Serialize)]
pub struct FileCopyResult {
    pub src: PathBuf,
    pub dest: PathBuf,
    pub size: usize,
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to set execution target")]
pub enum ExecutionTargetSetError {
    Connection(#[from] ConnectionError),
    Lock(#[from] MutexLockError),
}

#[derive(thiserror::Error, Debug)]
#[error("Missing execution target")]
pub struct UninitializedSshClientError;

#[derive(thiserror::Error, Debug)]
#[error("Failed to execute tasks")]
pub enum TaskError {
    Ssh(#[from] SshError),
    Lock(#[from] MutexLockError),
    UninitializedSshClient(#[from] UninitializedSshClientError),
}

pub trait OperationsModule {
    fn set_execution_target(&self, system: &SystemConfig) -> Result<(), ExecutionTargetSetError>;
    fn copy_file(&self, src: PathBuf, dest: PathBuf) -> Result<FileCopyResult, TaskError>;
    fn run_command(&self, cmd: String) -> Result<CommandResult, TaskError>;
}

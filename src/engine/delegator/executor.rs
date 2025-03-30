use std::path::PathBuf;

use mlua::IntoLua;
use serde::Serialize;

use super::{
    local::{HostError, LocalClient},
    ssh::{ConnectionError, SshClient, SshError},
};
use crate::{error::MutexLockError, memory::target_systems::TargetSystem};

#[derive(Clone)]
pub enum Executor {
    Ssh(SshClient),
    Dry,
    Host(LocalClient),
}

impl Executor {
    pub fn new_for_system(
        config: &TargetSystem,
        is_dry_run: bool,
    ) -> Result<Self, ExecutionTargetSetError> {
        Ok(match is_dry_run {
            true => Self::Dry,
            false => Self::Ssh(SshClient::connect(config)?),
        })
    }

    pub fn new_local() -> Self {
        Self::Host(LocalClient)
    }
}

#[derive(Debug, Serialize, Default)]
pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

impl IntoLua for CommandResult {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        let result_table = lua.create_table()?;

        result_table.set("stdout", self.stdout)?;
        result_table.set("stderr", self.stderr)?;
        result_table.set("exit_code", self.exit_code)?;

        result_table.set_readonly(true);

        Ok(mlua::Value::Table(result_table))
    }
}

#[derive(Debug, Serialize, Default)]
pub struct FileReadResult {
    pub path: PathBuf,
    pub content: String,
}

impl IntoLua for FileReadResult {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        let result_table = lua.create_table()?;

        result_table.set("path", self.path)?;
        result_table.set("content", self.content)?;

        result_table.set_readonly(true);

        Ok(mlua::Value::Table(result_table))
    }
}
#[derive(Debug, Serialize, Default)]
pub struct FileWriteResult {
    pub path: PathBuf,
    pub bytes_written: usize,
}

impl IntoLua for FileWriteResult {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        let result_table = lua.create_table()?;

        result_table.set("path", self.path)?;
        result_table.set("bytes_written", self.bytes_written)?;

        result_table.set_readonly(true);

        Ok(mlua::Value::Table(result_table))
    }
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
    Local(#[from] HostError),
    Lock(#[from] MutexLockError),
    UninitializedSshClientError(#[from] UninitializedSshClientError),
}

impl Executor {
    pub fn run_command(&self, cmd: String) -> Result<CommandResult, TaskError> {
        Ok(match self {
            Executor::Ssh(ssh_client) => ssh_client.execute_command(&cmd)?,
            Executor::Dry => {
                // info!("Running command {:?}", cmd);

                CommandResult::default()
            }
            Executor::Host(local_client) => local_client.execute_command(&cmd)?,
        })
    }
}

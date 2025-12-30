use mlua::IntoLua;
use serde::Serialize;

use super::{
    host::{HostClient, HostError},
    ssh::{ConnectionError, SshClient, SshError},
};
use crate::{
    engine::{
        delegator::local::{LocalError, with_local_dir},
        readonly::set_readonly,
    },
    error::{ErrorReport, MutexLockError},
    memory::target_systems::{TargetSystem, TargetSystemKind},
};

#[derive(Clone)]
pub enum Executor {
    Ssh(SshClient),
    Host(HostClient),
    Local(HostClient),
}

impl Executor {
    pub fn new_for_system(config: &TargetSystem) -> Result<Self, ExecutionTargetSetError> {
        Ok(match &config.kind {
            TargetSystemKind::Remote(remote_target_system) => {
                Self::Ssh(SshClient::connect(remote_target_system)?)
            }
            TargetSystemKind::Local => Self::new_local(),
        })
    }

    pub fn new_local() -> Self {
        Self::Local(HostClient)
    }

    pub fn new_host() -> Self {
        Self::Host(HostClient)
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

        let result_table = set_readonly(lua, result_table)
            .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))?;

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
    Host(#[from] HostError),
    Local(#[from] LocalError),
    Lock(#[from] MutexLockError),
    UninitializedSshClientError(#[from] UninitializedSshClientError),
}

impl Executor {
    pub fn run_command(&self, cmd: String) -> Result<CommandResult, TaskError> {
        Ok(match self {
            Executor::Ssh(ssh_client) => ssh_client.execute_command(&cmd)?,
            Executor::Host(local_client) => local_client.execute_command(&cmd)?,
            Executor::Local(local_client) => with_local_dir(|| local_client.execute_command(&cmd))?,
        })
    }
}

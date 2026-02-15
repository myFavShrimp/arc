use mlua::IntoLua;
use serde::Serialize;

use super::{
    host::{CommandError, HostClient},
    local::with_local_dir,
    ssh::{ConnectionError, SshClient, SshError},
};
use crate::{
    engine::readonly::set_readonly,
    error::ErrorReport,
    memory::target_systems::{TargetSystem, TargetSystemKind},
    progress::ProgressContext,
};

#[derive(Clone)]
pub struct Executor {
    kind: ExecutorKind,
    progress: ProgressContext,
}

#[derive(Clone)]
enum ExecutorKind {
    Ssh(SshClient),
    Host(HostClient),
    Local(HostClient),
}

impl Executor {
    pub fn new_for_system(
        config: &TargetSystem,
        progress: ProgressContext,
    ) -> Result<Self, ExecutionTargetSetError> {
        Ok(match &config.kind {
            TargetSystemKind::Remote(remote_target_system) => Self {
                kind: ExecutorKind::Ssh(SshClient::connect(remote_target_system)?),
                progress,
            },
            TargetSystemKind::Local => Self::new_local(progress),
        })
    }

    pub fn new_local(progress: ProgressContext) -> Self {
        Self {
            kind: ExecutorKind::Local(HostClient),
            progress,
        }
    }

    pub fn new_host(progress: ProgressContext) -> Self {
        Self {
            kind: ExecutorKind::Host(HostClient),
            progress,
        }
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

        let result_table = set_readonly(lua, result_table).map_err(|error| {
            mlua::Error::RuntimeError(ErrorReport::boxed_from(error).build_report())
        })?;

        Ok(mlua::Value::Table(result_table))
    }
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to set execution target")]
pub enum ExecutionTargetSetError {
    Connection(#[from] ConnectionError),
}

#[derive(thiserror::Error, Debug)]
#[error("Missing execution target")]
pub struct UninitializedSshClientError;

#[derive(thiserror::Error, Debug)]
#[error("Failed to execute tasks")]
pub enum TaskError {
    Ssh(#[from] SshError),
    Host(#[from] CommandError),
    Progress(#[from] crate::progress::CommandProgressCreationError),
    UninitializedSshClientError(#[from] UninitializedSshClientError),
}

impl Executor {
    pub fn run_command(&self, cmd: String) -> Result<CommandResult, TaskError> {
        let progress = self.progress.command(&cmd)?;

        let result = match &self.kind {
            ExecutorKind::Ssh(ssh_client) => ssh_client.execute_command(&cmd, &progress)?,
            ExecutorKind::Host(local_client) => local_client.execute_command(&cmd, &progress)?,
            ExecutorKind::Local(local_client) => {
                with_local_dir(|| local_client.execute_command(&cmd, &progress))?
            }
        };

        progress.finish();

        Ok(result)
    }
}

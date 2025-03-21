use std::{net::IpAddr, path::PathBuf};

use mlua::{IntoLua, UserData};
use serde::Serialize;
use ssh_executor::SshExecutor;

use crate::{
    error::{ErrorReport, MutexLockError},
    ssh::{ConnectionError, SshClient, SshError},
};

use super::targets::systems::SystemConfig;

mod ssh_executor;

#[derive(Clone)]
pub struct System {
    pub address: IpAddr,
    pub port: u16,
    pub user: String,
    execution_delegator: ExecutionDelegator,
}

impl System {
    pub fn connect(config: &SystemConfig) -> Result<Self, ExecutionTargetSetError> {
        let ssh_client = SshClient::connect(config)?;
        let ssh_executor = SshExecutor::new(ssh_client);

        Ok(Self {
            address: config.address.clone(),
            port: config.port.clone(),
            user: config.user.clone(),
            execution_delegator: ExecutionDelegator { ssh: ssh_executor },
        })
    }
}

#[derive(Clone)]
pub struct ExecutionDelegator {
    ssh: SshExecutor,
}

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
pub struct FileCopyResult {
    pub src: PathBuf,
    pub dest: PathBuf,
    pub size: usize,
}

impl IntoLua for FileCopyResult {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        let result_table = lua.create_table()?;

        result_table.set("src", self.src)?;
        result_table.set("dest", self.dest)?;
        result_table.set("size", self.size)?;

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
    Lock(#[from] MutexLockError),
    UninitializedSshClient(#[from] UninitializedSshClientError),
}

impl Executor for ExecutionDelegator {
    fn copy_file(&self, src: PathBuf, dest: PathBuf) -> Result<FileCopyResult, TaskError> {
        self.ssh.copy_file(src, dest)
    }

    fn run_command(&self, cmd: String) -> Result<CommandResult, TaskError> {
        self.ssh.run_command(cmd)
    }
}

pub trait Executor {
    fn copy_file(&self, src: PathBuf, dest: PathBuf) -> Result<FileCopyResult, TaskError>;
    fn run_command(&self, cmd: String) -> Result<CommandResult, TaskError>;
}

impl UserData for System {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("address", |_, this| Ok(this.address.to_string()));
        fields.add_field_method_get("port", |_, this| Ok(this.port));
        fields.add_field_method_get("user", |_, this| Ok(this.user.clone()));
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("run_command", |_, this, cmd: String| {
            Ok(this
                .execution_delegator
                .run_command(cmd)
                .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))?)
        });

        methods.add_method("copy_file", |_, this, (src, dest): (PathBuf, PathBuf)| {
            Ok(this
                .execution_delegator
                .copy_file(src, dest)
                .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))?)
        });
    }
}

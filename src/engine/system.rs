use std::{net::IpAddr, path::PathBuf};

use mlua::{IntoLua, UserData};
use ssh_executor::SshExecutor;

use crate::{error::ErrorReport, ssh::SshClient};

use super::modules::{
    operations::{CommandResult, FileCopyResult, TaskError},
    targets::SystemConfig,
};

mod ssh_executor;

#[derive(Clone)]
pub struct System {
    pub address: IpAddr,
    pub port: u16,
    pub user: String,
    execution_delegator: ExecutionDelegator,
}

impl System {
    pub fn connect(
        config: &SystemConfig,
    ) -> Result<Self, crate::engine::modules::operations::ExecutionTargetSetError> {
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

// TODO: Remove and add automatic executor selection for system?
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
            this.execution_delegator
                .run_command(cmd)
                .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))?;
            Ok(())
        });

        methods.add_method("copy_file", |_, this, (src, dest): (PathBuf, PathBuf)| {
            this.execution_delegator
                .copy_file(src, dest)
                .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))?;
            Ok(())
        });
    }
}

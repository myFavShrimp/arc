use std::{net::IpAddr, path::PathBuf};

use mlua::IntoLua;
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

impl IntoLua for System {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        let table = lua.create_table()?;

        table.set("address", self.address.to_string())?;
        table.set("port", self.port)?;
        table.set("user", self.user)?;

        {
            let execution_delegator = self.execution_delegator.clone();
            table.set(
                "run_command",
                lua.create_function(move |_lua, (self_table, cmd): (SystemConfig, String)| {
                    execution_delegator.run_command(cmd).map_err(|e| {
                        mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report())
                    })?;
                    Ok(())
                })?,
            )?;
        }

        table.set_readonly(true);

        return Ok(mlua::Value::Table(table));
    }
}

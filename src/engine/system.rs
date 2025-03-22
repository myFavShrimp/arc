use std::{net::IpAddr, path::PathBuf};

use executor::{ExecutionTargetSetError, Executor};
use mlua::UserData;

use crate::{error::ErrorReport, ssh::SshClient};

use super::targets::systems::SystemConfig;

pub mod executor;

#[derive(Clone)]
pub struct System {
    pub name: String,
    pub address: IpAddr,
    pub port: u16,
    pub user: String,
    execution_delegator: Executor,
}

impl System {
    pub fn connect(config: &SystemConfig) -> Result<Self, ExecutionTargetSetError> {
        let ssh_client = SshClient::connect(config)?;
        let ssh_executor = Executor::new(ssh_client);

        Ok(Self {
            name: config.name.clone(),
            address: config.address,
            port: config.port,
            user: config.user.clone(),
            execution_delegator: ssh_executor,
        })
    }
}

impl UserData for System {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("name", |_, this| Ok(this.name.clone()));
        fields.add_field_method_get("address", |_, this| Ok(this.address.to_string()));
        fields.add_field_method_get("port", |_, this| Ok(this.port));
        fields.add_field_method_get("user", |_, this| Ok(this.user.clone()));
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("run_command", |_, this, cmd: String| {
            this.execution_delegator
                .run_command(cmd)
                .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
        });

        methods.add_method("read_file", |_, this, (path,): (PathBuf,)| {
            this.execution_delegator
                .read_file(path)
                .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
        });

        methods.add_method(
            "write_file",
            |_, this, (path, content): (PathBuf, String)| {
                this.execution_delegator
                    .write_file(path, content)
                    .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
            },
        );

        methods.add_method("rename_file", |_, this, (from, to): (PathBuf, PathBuf)| {
            this.execution_delegator
                .rename_file(from, to)
                .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
        });

        methods.add_method("remove_file", |_, this, (path,): (PathBuf,)| {
            this.execution_delegator
                .remove_file(path)
                .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
        });

        methods.add_method("remove_directory", |_, this, (path,): (PathBuf,)| {
            this.execution_delegator
                .remove_directory(path)
                .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
        });

        methods.add_method("create_directory", |_, this, (path,): (PathBuf,)| {
            this.execution_delegator
                .create_directory(path)
                .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
        });

        methods.add_method(
            "set_permissions",
            |_, this, (path, mode): (PathBuf, u32)| {
                this.execution_delegator
                    .set_permissions(path, mode)
                    .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
            },
        );

        methods.add_method("metadata", |_, this, (path,): (PathBuf,)| {
            this.execution_delegator
                .metadata(path)
                .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
        });
    }
}

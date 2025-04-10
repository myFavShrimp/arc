use std::{net::IpAddr, path::PathBuf};

use mlua::UserData;

use crate::{
    engine::delegator::{
        executor::Executor,
        operator::{FileSystemOperator, MetadataType},
    },
    error::ErrorReport,
};

use super::{directory::Directory, file::File};

#[derive(Clone)]
pub struct System {
    pub name: String,
    pub address: IpAddr,
    pub port: u16,
    pub user: String,
    pub executor: Executor,
    pub file_system_operator: FileSystemOperator,
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
            this.executor
                .run_command(cmd)
                .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
        });

        methods.add_method("file", |_, this, path: PathBuf| {
            this.file_system_operator
                .file(&path)
                .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
        });

        methods.add_method("directory", |_, this, path: PathBuf| {
            this.file_system_operator
                .directory(&path)
                .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
        });
    }
}

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
            let metadata = this
                .file_system_operator
                .metadata(&path)
                .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))?;

            match metadata {
                None => Ok(File {
                    path,
                    file_system_operator: this.file_system_operator.clone(),
                }),
                Some(metadata) => match metadata.r#type {
                    MetadataType::File => Ok(File {
                        path,
                        file_system_operator: this.file_system_operator.clone(),
                    }),
                    MetadataType::Directory => Err(mlua::Error::runtime(format!(
                        "{:?} is a directory - expected a file",
                        path,
                    ))),
                    MetadataType::Unknown => {
                        Err(mlua::Error::runtime(format!("{:?} is not a file", path)))
                    }
                },
            }
        });

        methods.add_method("directory", |_, this, path: PathBuf| {
            let metadata = this
                .file_system_operator
                .metadata(&path)
                .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))?;

            match metadata {
                None => Ok(Directory {
                    path,
                    file_system_operator: this.file_system_operator.clone(),
                }),
                Some(metadata) => match metadata.r#type {
                    MetadataType::File => Err(mlua::Error::runtime(format!(
                        "{:?} is a file - expected a directory",
                        path,
                    ))),
                    MetadataType::Directory => Ok(Directory {
                        path,
                        file_system_operator: this.file_system_operator.clone(),
                    }),
                    MetadataType::Unknown => Err(mlua::Error::runtime(format!(
                        "{:?} is not a directory",
                        path,
                    ))),
                },
            }
        });

        // methods.add_method("read_file", |_, this, (path,): (PathBuf,)| {
        //     this.executor
        //         .read_file(path)
        //         .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
        // });

        // methods.add_method(
        //     "write_file",
        //     |_, this, (path, content): (PathBuf, String)| {
        //         this.executor
        //             .write_file(path, content)
        //             .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
        //     },
        // );

        // methods.add_method("rename_file", |_, this, (from, to): (PathBuf, PathBuf)| {
        //     this.executor
        //         .rename_file(from, to)
        //         .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
        // });

        // methods.add_method("remove_file", |_, this, (path,): (PathBuf,)| {
        //     this.executor
        //         .remove_file(path)
        //         .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
        // });

        // methods.add_method("remove_directory", |_, this, (path,): (PathBuf,)| {
        //     this.executor
        //         .remove_directory(path)
        //         .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
        // });

        // methods.add_method("create_directory", |_, this, (path,): (PathBuf,)| {
        //     this.executor
        //         .create_directory(path)
        //         .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
        // });

        // methods.add_method(
        //     "set_permissions",
        //     |_, this, (path, mode): (PathBuf, u32)| {
        //         this.executor
        //             .set_permissions(path, mode)
        //             // .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
        //             .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
        //     },
        // );

        // methods.add_method("metadata", |_, this, (path,): (PathBuf,)| {
        //     this.executor
        //         .metadata(path)
        //         .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
        // });
    }
}

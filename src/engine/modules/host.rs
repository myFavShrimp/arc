use std::path::PathBuf;

use mlua::UserData;

use crate::{
    engine::{
        delegator::{
            executor::Executor,
            operator::{FileSystemOperator, MetadataType},
        },
        objects::{directory::Directory, file::File},
    },
    error::ErrorReport,
};

#[derive(Clone)]
pub struct Host {
    pub executor: Executor,
    pub file_system_operator: FileSystemOperator,
}

impl Host {
    pub fn new() -> Self {
        Self {
            executor: Executor::new_local(),
            file_system_operator: FileSystemOperator::new_local(),
        }
    }
}

impl UserData for Host {
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
    }
}

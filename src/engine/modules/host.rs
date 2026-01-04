use std::path::PathBuf;

use mlua::UserData;

use crate::{
    engine::delegator::{executor::Executor, operator::FileSystemOperator},
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
            executor: Executor::new_host(),
            file_system_operator: FileSystemOperator::new_host(),
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

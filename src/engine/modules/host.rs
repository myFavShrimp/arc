use std::{panic::resume_unwind, path::PathBuf};

use mlua::UserData;

use crate::{
    engine::{
        delegator::{
            error::{FfiError, FfiPanicError},
            executor::Executor,
            operator::FileSystemOperator,
        },
        modules::MountToGlobals,
    },
    error::ErrorReport,
    progress::ProgressContext,
};

#[derive(Clone)]
pub struct Host {
    pub executor: Executor,
    pub file_system_operator: FileSystemOperator,
}

impl Host {
    pub fn new(progress: ProgressContext) -> Self {
        Self {
            executor: Executor::new_host(progress.clone()),
            file_system_operator: FileSystemOperator::new_host(progress),
        }
    }
}

impl UserData for Host {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("run_command", |_, this, command: String| {
            let result = this
                .executor
                .run_command(command)
                .unwrap_or_else(|error| resume_unwind(Box::new(FfiPanicError(Box::new(error)))));

            Ok(result)
        });

        methods.add_method("file", |_, this, path: PathBuf| {
            this.file_system_operator.file(&path).map_err(|error| {
                mlua::Error::RuntimeError(
                    ErrorReport::boxed_from(error.enforce_ffi_boundary()).build_report(),
                )
            })
        });

        methods.add_method("directory", |_, this, path: PathBuf| {
            this.file_system_operator.directory(&path).map_err(|error| {
                mlua::Error::RuntimeError(
                    ErrorReport::boxed_from(error.enforce_ffi_boundary()).build_report(),
                )
            })
        });
    }
}

impl MountToGlobals for Host {
    fn mount_to_globals(self, lua: &mut mlua::Lua) -> Result<(), mlua::Error> {
        let globals = lua.globals();
        globals.set("host", self)?;

        Ok(())
    }
}

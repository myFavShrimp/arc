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
            let result = this
                .executor
                .run_command(cmd)
                .unwrap_or_else(|e| resume_unwind(Box::new(FfiPanicError(Box::new(e)))));

            Ok(result)
        });

        methods.add_method("file", |_, this, path: PathBuf| {
            this.file_system_operator.file(&path).map_err(|e| {
                mlua::Error::RuntimeError(
                    ErrorReport::boxed_from(e.enforce_ffi_boundary()).report(),
                )
            })
        });

        methods.add_method("directory", |_, this, path: PathBuf| {
            this.file_system_operator.directory(&path).map_err(|e| {
                mlua::Error::RuntimeError(
                    ErrorReport::boxed_from(e.enforce_ffi_boundary()).report(),
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

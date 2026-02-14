use std::{net::IpAddr, panic::resume_unwind, path::PathBuf};

use mlua::UserData;

use crate::engine::delegator::error::FfiPanicError;
use crate::engine::delegator::{error::FfiError, executor::Executor, operator::FileSystemOperator};
use crate::error::ErrorReport;

#[derive(Clone)]
pub struct System {
    pub name: String,
    pub kind: SystemKind,
}

#[derive(Clone)]
pub enum SystemKind {
    Remote(RemoteSystem),
    Local(Executor, FileSystemOperator),
}

impl SystemKind {
    fn address(&self) -> Option<String> {
        match self {
            SystemKind::Remote(remote_system) => Some(remote_system.address.to_string()),
            SystemKind::Local(..) => None,
        }
    }
    fn port(&self) -> Option<u16> {
        match self {
            SystemKind::Remote(remote_system) => Some(remote_system.port),
            SystemKind::Local(..) => None,
        }
    }
    fn user(&self) -> Option<String> {
        match self {
            SystemKind::Remote(remote_system) => Some(remote_system.user.clone()),
            SystemKind::Local(..) => None,
        }
    }
    fn r#type(&self) -> String {
        match self {
            SystemKind::Remote(..) => String::from("remote"),
            SystemKind::Local(..) => String::from("local"),
        }
    }

    fn file_system_operator(&self) -> &FileSystemOperator {
        match self {
            SystemKind::Remote(remote_system) => &remote_system.file_system_operator,
            SystemKind::Local(_executor, file_system_operator) => file_system_operator,
        }
    }

    fn executor(&self) -> &Executor {
        match self {
            SystemKind::Remote(remote_system) => &remote_system.executor,
            SystemKind::Local(executor, _file_system_operator) => executor,
        }
    }
}

#[derive(Clone)]
pub struct RemoteSystem {
    pub address: IpAddr,
    pub port: u16,
    pub user: String,
    pub executor: Executor,
    pub file_system_operator: FileSystemOperator,
}

impl UserData for System {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("name", |_, this| Ok(this.name.clone()));
        fields.add_field_method_get("type", |_, this| Ok(this.kind.r#type()));

        fields.add_field_method_get("address", |_, this| Ok(this.kind.address()));
        fields.add_field_method_get("port", |_, this| Ok(this.kind.port()));
        fields.add_field_method_get("user", |_, this| Ok(this.kind.user()));
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("run_command", |_, this, cmd: String| {
            let result = this
                .kind
                .executor()
                .run_command(cmd)
                .unwrap_or_else(|e| resume_unwind(Box::new(FfiPanicError(Box::new(e)))));

            Ok(result)
        });

        methods.add_method("file", |_, this, path: PathBuf| {
            this.kind.file_system_operator().file(&path).map_err(|e| {
                mlua::Error::RuntimeError(
                    ErrorReport::boxed_from(e.enforce_ffi_boundary()).report(),
                )
            })
        });

        methods.add_method("directory", |_, this, path: PathBuf| {
            this.kind
                .file_system_operator()
                .directory(&path)
                .map_err(|e| {
                    mlua::Error::RuntimeError(
                        ErrorReport::boxed_from(e.enforce_ffi_boundary()).report(),
                    )
                })
        });
    }
}

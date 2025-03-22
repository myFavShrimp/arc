use std::{net::IpAddr, path::PathBuf};

use mlua::{IntoLua, UserData};
use serde::Serialize;
use ssh_executor::SshExecutor;

use crate::{
    error::{ErrorReport, MutexLockError},
    ssh::{self, ConnectionError, SshClient, SshError},
};

use super::targets::systems::SystemConfig;

mod ssh_executor;

#[derive(Clone)]
pub struct System {
    pub address: IpAddr,
    pub port: u16,
    pub user: String,
    execution_delegator: ExecutionDelegator,
}

impl System {
    pub fn connect(config: &SystemConfig) -> Result<Self, ExecutionTargetSetError> {
        let ssh_client = SshClient::connect(config)?;
        let ssh_executor = SshExecutor::new(ssh_client);

        Ok(Self {
            address: config.address,
            port: config.port,
            user: config.user.clone(),
            execution_delegator: ExecutionDelegator { ssh: ssh_executor },
        })
    }
}

#[derive(Clone)]
pub struct ExecutionDelegator {
    ssh: SshExecutor,
}

#[derive(Debug, Serialize)]
pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

impl IntoLua for CommandResult {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        let result_table = lua.create_table()?;

        result_table.set("stdout", self.stdout)?;
        result_table.set("stderr", self.stderr)?;
        result_table.set("exit_code", self.exit_code)?;

        result_table.set_readonly(true);

        Ok(mlua::Value::Table(result_table))
    }
}

#[derive(Debug, Serialize)]
pub struct FileReadResult {
    pub path: String,
    pub content: String,
}

impl IntoLua for FileReadResult {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        let result_table = lua.create_table()?;

        result_table.set("path", self.path)?;
        result_table.set("content", self.content)?;

        result_table.set_readonly(true);

        Ok(mlua::Value::Table(result_table))
    }
}
#[derive(Debug, Serialize)]
pub struct FileWriteResult {
    pub path: String,
    pub bytes_written: usize,
}

impl IntoLua for FileWriteResult {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        let result_table = lua.create_table()?;

        result_table.set("path", self.path)?;
        result_table.set("bytes_written", self.bytes_written)?;

        result_table.set_readonly(true);

        Ok(mlua::Value::Table(result_table))
    }
}

pub struct MetadataResult {
    pub path: String,
    pub size: Option<u64>,
    pub permissions: Option<u32>,
    pub r#type: MetadataType,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub accessed: Option<u64>,
    pub modified: Option<u64>,
}

pub enum MetadataType {
    File,
    Directory,
    Unknown,
}

impl ToString for MetadataType {
    fn to_string(&self) -> String {
        match self {
            MetadataType::File => "file".to_string(),
            MetadataType::Directory => "directory".to_string(),
            MetadataType::Unknown => "unknown".to_string(),
        }
    }
}

impl IntoLua for MetadataResult {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        let result_table = lua.create_table()?;

        result_table.set("path", self.path)?;
        result_table.set("size", self.size)?;
        result_table.set("permissions", self.permissions)?;
        result_table.set("type", self.r#type.to_string())?;
        result_table.set("uid", self.uid)?;
        result_table.set("gid", self.gid)?;
        result_table.set("accessed", self.accessed)?;
        result_table.set("modified", self.modified)?;

        result_table.set_readonly(true);

        Ok(mlua::Value::Table(result_table))
    }
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to set execution target")]
pub enum ExecutionTargetSetError {
    Connection(#[from] ConnectionError),
    Lock(#[from] MutexLockError),
}

#[derive(thiserror::Error, Debug)]
#[error("Missing execution target")]
pub struct UninitializedSshClientError;

#[derive(thiserror::Error, Debug)]
#[error("Failed to execute tasks")]
pub enum TaskError {
    Ssh(#[from] SshError),
    Lock(#[from] MutexLockError),
    UninitializedSshClient(#[from] UninitializedSshClientError),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum FileReadError {
    Ssh(#[from] ssh::FileError<ssh::FileReadErrorKind>),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum FileWriteError {
    Ssh(#[from] ssh::FileError<ssh::FileWriteErrorKind>),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum RenameError {
    Ssh(#[from] ssh::RenameError),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum RemoveFileError {
    Ssh(#[from] ssh::RemoveFileError),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum RemoveDirectoryError {
    Ssh(#[from] ssh::RemoveDirectoryError),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum CreateDirectoryError {
    Ssh(#[from] ssh::CreateDirectoryError),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum SetPermissionsError {
    Ssh(#[from] ssh::SetPermissionsError),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum MetadataError {
    Ssh(#[from] ssh::MetadataError),
}

impl Executor for ExecutionDelegator {
    fn read_file(&self, path: PathBuf) -> Result<FileReadResult, FileReadError> {
        self.ssh.read_file(path)
    }

    fn write_file(
        &self,
        path: PathBuf,
        content: String,
    ) -> Result<FileWriteResult, FileWriteError> {
        self.ssh.write_file(path, content)
    }

    fn rename_file(&self, from: PathBuf, to: PathBuf) -> Result<(), RenameError> {
        self.ssh.rename_file(from, to)
    }

    fn remove_file(&self, path: PathBuf) -> Result<(), RemoveFileError> {
        self.ssh.remove_file(path)
    }

    fn remove_directory(&self, path: PathBuf) -> Result<(), RemoveDirectoryError> {
        self.ssh.remove_directory(path)
    }

    fn create_directory(&self, path: PathBuf) -> Result<(), CreateDirectoryError> {
        self.ssh.create_directory(path)
    }

    fn set_permissions(&self, path: PathBuf, mode: u32) -> Result<(), SetPermissionsError> {
        self.ssh.set_permissions(path, mode)
    }

    fn metadata(&self, path: PathBuf) -> Result<Option<MetadataResult>, MetadataError> {
        self.ssh.metadata(path)
    }

    fn run_command(&self, cmd: String) -> Result<CommandResult, TaskError> {
        self.ssh.run_command(cmd)
    }
}

pub trait Executor {
    fn read_file(&self, path: PathBuf) -> Result<FileReadResult, FileReadError>;
    fn write_file(&self, path: PathBuf, content: String)
        -> Result<FileWriteResult, FileWriteError>;
    fn rename_file(&self, from: PathBuf, to: PathBuf) -> Result<(), RenameError>;
    fn remove_file(&self, path: PathBuf) -> Result<(), RemoveFileError>;
    fn remove_directory(&self, path: PathBuf) -> Result<(), RemoveDirectoryError>;
    fn create_directory(&self, path: PathBuf) -> Result<(), CreateDirectoryError>;
    fn set_permissions(&self, path: PathBuf, mode: u32) -> Result<(), SetPermissionsError>;
    fn metadata(&self, path: PathBuf) -> Result<Option<MetadataResult>, MetadataError>;
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

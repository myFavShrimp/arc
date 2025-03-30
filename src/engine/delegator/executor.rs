use std::{fmt::Display, path::PathBuf};

use mlua::IntoLua;
use serde::Serialize;

use super::ssh::{self, ConnectionError, SshClient, SshError};
use crate::{error::MutexLockError, memory::target_systems::TargetSystem};

#[derive(Clone)]
pub enum Executor {
    Ssh(SshClient),
    Dry,
}

impl Executor {
    pub fn new_for_system(
        config: &TargetSystem,
        is_dry_run: bool,
    ) -> Result<Self, ExecutionTargetSetError> {
        Ok(match is_dry_run {
            true => Self::Dry,
            false => Self::Ssh(SshClient::connect(config)?),
        })
    }
}

#[derive(Debug, Serialize, Default)]
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

#[derive(Debug, Serialize, Default)]
pub struct FileReadResult {
    pub path: PathBuf,
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
#[derive(Debug, Serialize, Default)]
pub struct FileWriteResult {
    pub path: PathBuf,
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

#[derive(Default)]
pub struct MetadataResult {
    pub path: PathBuf,
    pub size: Option<u64>,
    pub permissions: Option<u32>,
    pub r#type: MetadataType,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub accessed: Option<u64>,
    pub modified: Option<u64>,
}

#[derive(Default)]
pub enum MetadataType {
    File,
    Directory,
    #[default]
    Unknown,
}

impl Display for MetadataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            MetadataType::File => "file",
            MetadataType::Directory => "directory",
            MetadataType::Unknown => "unknown",
        })
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

impl Executor {
    pub fn read_file(&self, path: PathBuf) -> Result<FileReadResult, FileReadError> {
        Ok(match self {
            Executor::Ssh(ssh_client) => ssh_client.read_file(path)?,
            Executor::Dry => {
                // info!("CReading file {:?}", path);

                FileReadResult {
                    path,
                    ..Default::default()
                }
            }
        })
    }

    pub fn write_file(
        &self,
        path: PathBuf,
        content: String,
    ) -> Result<FileWriteResult, FileWriteError> {
        Ok(match self {
            Executor::Ssh(ssh_client) => ssh_client.write_file(path, &content)?,
            Executor::Dry => {
                // info!("Writing file {:?} with content {:?}", path, content);

                FileWriteResult {
                    path,
                    ..Default::default()
                }
            }
        })
    }

    pub fn rename_file(&self, from: PathBuf, to: PathBuf) -> Result<(), RenameError> {
        match self {
            Executor::Ssh(ssh_client) => ssh_client.rename_file(from, to)?,
            Executor::Dry => {
                // info!("Renaming file {:?} {:?}", from, to);
            }
        };
        Ok(())
    }

    pub fn remove_file(&self, path: PathBuf) -> Result<(), RemoveFileError> {
        match self {
            Executor::Ssh(ssh_client) => ssh_client.remove_file(path)?,
            Executor::Dry => {
                // info!("Removing file {:?}", path);
            }
        };
        Ok(())
    }

    pub fn remove_directory(&self, path: PathBuf) -> Result<(), RemoveDirectoryError> {
        match self {
            Executor::Ssh(ssh_client) => ssh_client.remove_directory(path)?,
            Executor::Dry => {
                // info!("Removing directory {:?}", path);
            }
        };
        Ok(())
    }

    pub fn create_directory(&self, path: PathBuf) -> Result<(), CreateDirectoryError> {
        match self {
            Executor::Ssh(ssh_client) => ssh_client.create_directory(path)?,
            Executor::Dry => {
                // info!("Creating directory {:?}", path);
            }
        };
        Ok(())
    }

    pub fn set_permissions(&self, path: PathBuf, mode: u32) -> Result<(), SetPermissionsError> {
        match self {
            Executor::Ssh(ssh_client) => ssh_client.set_permissions(path, mode)?,
            Executor::Dry => {
                // info!("Setting permission of {:?} to {:o}", path, mode);
            }
        };
        Ok(())
    }

    pub fn run_command(&self, cmd: String) -> Result<CommandResult, TaskError> {
        Ok(match self {
            Executor::Ssh(ssh_client) => ssh_client.execute_command(&cmd)?,
            Executor::Dry => {
                // info!("Running command {:?}", cmd);

                CommandResult::default()
            }
        })
    }

    pub fn metadata(&self, path: PathBuf) -> Result<Option<MetadataResult>, MetadataError> {
        Ok(match self {
            Executor::Ssh(ssh_client) => ssh_client.metadata(path)?,
            Executor::Dry => {
                // info!("Retrieving metadata for {:?}", path);

                Some(MetadataResult {
                    path,
                    ..Default::default()
                })
            }
        })
    }
}

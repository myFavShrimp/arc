use std::{fmt::Display, path::PathBuf};

use mlua::IntoLua;
use serde::Serialize;

use super::ssh::{self, ConnectionError, SshClient, SshError};
use crate::{error::MutexLockError, memory::target_systems::TargetSystem};

#[derive(Clone)]
pub enum FileSystemOperator {
    Ssh(SshClient),
    Dry,
}

impl FileSystemOperator {
    pub fn new_for_system(
        config: &TargetSystem,
        is_dry_run: bool,
    ) -> Result<Self, OperationTargetSetError> {
        Ok(match is_dry_run {
            true => Self::Dry,
            false => Self::Ssh(SshClient::connect(config)?),
        })
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
pub enum OperationTargetSetError {
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

impl FileSystemOperator {
    pub fn read_file(&self, path: &PathBuf) -> Result<Vec<u8>, FileReadError> {
        Ok(match self {
            FileSystemOperator::Ssh(ssh_client) => ssh_client.read_file(path)?,
            FileSystemOperator::Dry => {
                // info!("CReading file {:?}", path);

                Vec::new()
            }
        })
    }

    pub fn write_file(
        &self,
        path: &PathBuf,
        content: &[u8],
    ) -> Result<FileWriteResult, FileWriteError> {
        Ok(match self {
            FileSystemOperator::Ssh(ssh_client) => ssh_client.write_file(path, content)?,
            FileSystemOperator::Dry => {
                // info!("Writing file {:?} with content {:?}", path, content);

                FileWriteResult {
                    path: path.clone(),
                    ..Default::default()
                }
            }
        })
    }

    pub fn rename(&self, from: &PathBuf, to: &PathBuf) -> Result<(), RenameError> {
        match self {
            FileSystemOperator::Ssh(ssh_client) => ssh_client.rename_file(from, to)?,
            FileSystemOperator::Dry => {
                // info!("Renaming file {:?} {:?}", from, to);
            }
        };
        Ok(())
    }

    pub fn remove_file(&self, path: &PathBuf) -> Result<(), RemoveFileError> {
        match self {
            FileSystemOperator::Ssh(ssh_client) => ssh_client.remove_file(path)?,
            FileSystemOperator::Dry => {
                // info!("Removing file {:?}", path);
            }
        };
        Ok(())
    }

    pub fn remove_directory(&self, path: &PathBuf) -> Result<(), RemoveDirectoryError> {
        match self {
            FileSystemOperator::Ssh(ssh_client) => ssh_client.remove_directory(path)?,
            FileSystemOperator::Dry => {
                // info!("Removing directory {:?}", path);
            }
        };
        Ok(())
    }

    pub fn create_directory(&self, path: &PathBuf) -> Result<(), CreateDirectoryError> {
        match self {
            FileSystemOperator::Ssh(ssh_client) => ssh_client.create_directory(path)?,
            FileSystemOperator::Dry => {
                // info!("Creating directory {:?}", path);
            }
        };
        Ok(())
    }

    pub fn set_permissions(&self, path: &PathBuf, mode: u32) -> Result<(), SetPermissionsError> {
        match self {
            FileSystemOperator::Ssh(ssh_client) => ssh_client.set_permissions(path, mode)?,
            FileSystemOperator::Dry => {
                // info!("Setting permission of {:?} to {:o}", path, mode);
            }
        };
        Ok(())
    }

    pub fn metadata(&self, path: &PathBuf) -> Result<Option<MetadataResult>, MetadataError> {
        Ok(match self {
            FileSystemOperator::Ssh(ssh_client) => ssh_client.metadata(path)?,
            FileSystemOperator::Dry => {
                // info!("Retrieving metadata for {:?}", path);

                Some(MetadataResult {
                    path: path.clone(),
                    ..Default::default()
                })
            }
        })
    }
}

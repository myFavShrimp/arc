use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use mlua::IntoLua;
use serde::Serialize;

use super::{
    host::{self, HostClient},
    ssh::{self, ConnectionError, SshClient},
};
use crate::{
    engine::{
        delegator::local::{LocalError, with_local_dir},
        objects::{directory::Directory, file::File},
        readonly::set_readonly,
    },
    error::{ErrorReport, MutexLockError},
    memory::target_systems::{TargetSystem, TargetSystemKind},
};

#[derive(Clone)]
pub enum FileSystemEntry {
    File(File),
    Directory(Directory),
}

impl FileSystemEntry {
    pub fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        match self {
            FileSystemEntry::File(file) => Ok(mlua::Value::UserData(lua.create_userdata(file)?)),
            FileSystemEntry::Directory(dir) => Ok(mlua::Value::UserData(lua.create_userdata(dir)?)),
        }
    }
}

#[derive(Clone)]
pub enum FileSystemOperator {
    Ssh(SshClient),
    Local(HostClient),
    Host(HostClient),
}

impl FileSystemOperator {
    pub fn new_for_system(config: &TargetSystem) -> Result<Self, OperationTargetSetError> {
        Ok(match &config.kind {
            TargetSystemKind::Remote(remote_target_system) => {
                Self::Ssh(SshClient::connect(remote_target_system)?)
            }
            TargetSystemKind::Local => Self::new_local(),
        })
    }

    pub fn new_local() -> Self {
        Self::Local(HostClient)
    }

    pub fn new_host() -> Self {
        Self::Host(HostClient)
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

        let result_table = set_readonly(lua, result_table)
            .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))?;

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

#[derive(Default, PartialEq, Eq, Debug)]
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

        let result_table = set_readonly(lua, result_table)
            .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))?;

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
#[error(transparent)]
pub enum FileReadError {
    Ssh(#[from] ssh::FileReadError),
    Local(#[from] host::FileReadError),
    LocalDir(#[from] LocalError),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum FileWriteError {
    Ssh(#[from] ssh::FileWriteError),
    Local(#[from] host::FileWriteError),
    LocalDir(#[from] LocalError),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum RenameError {
    Ssh(#[from] ssh::RenameError),
    Local(#[from] host::RenameError),
    LocalDir(#[from] LocalError),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum RemoveFileError {
    Ssh(#[from] ssh::RemoveFileError),
    Local(#[from] host::RemoveFileError),
    LocalDir(#[from] LocalError),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum RemoveDirectoryError {
    Ssh(#[from] ssh::RemoveDirectoryError),
    Local(#[from] host::RemoveDirectoryError),
    LocalDir(#[from] LocalError),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum CreateDirectoryError {
    Ssh(#[from] ssh::CreateDirectoryError),
    Local(#[from] host::CreateDirectoryError),
    LocalDir(#[from] LocalError),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum SetPermissionsError {
    Ssh(#[from] ssh::SetPermissionsError),
    Local(#[from] host::SetPermissionsError),
    LocalDir(#[from] LocalError),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum MetadataError {
    Ssh(#[from] ssh::MetadataError),
    Local(#[from] host::MetadataError),
    LocalDir(#[from] LocalError),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum ListDirectoryError {
    Ssh(#[from] ssh::DirectoryEntriesError),
    Local(#[from] host::DirectoryEntriesError),
    LocalDir(#[from] LocalError),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum FileError {
    Ssh(#[from] ssh::MetadataError),
    Local(#[from] host::MetadataError),
    Metadata(#[from] MetadataError),
    UnexpectedDirectory(#[from] UnexpectedDirectoryError),
    NotAFile(#[from] NotAFileError),
}

#[derive(Debug, thiserror::Error)]
#[error("{0:?} is a directory - expected a file")]
pub struct UnexpectedDirectoryError(PathBuf);

#[derive(Debug, thiserror::Error)]
#[error("{0:?} is not a file")]
pub struct NotAFileError(PathBuf);

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum DirectoryError {
    Ssh(#[from] ssh::MetadataError),
    Local(#[from] host::MetadataError),
    Metadata(#[from] MetadataError),
    UnexpectedType(#[from] UnexpectedTypeError),
}

#[derive(Debug, thiserror::Error)]
#[error("{path:?} is of type {actual} - expected {expected}")]
pub struct UnexpectedTypeError {
    path: PathBuf,
    expected: MetadataType,
    actual: MetadataType,
}

impl FileSystemOperator {
    pub fn read_file(&self, path: &PathBuf) -> Result<Vec<u8>, FileReadError> {
        Ok(match self {
            FileSystemOperator::Ssh(ssh_client) => ssh_client.read_file(path)?,
            FileSystemOperator::Local(local_client) => {
                with_local_dir(|| local_client.read_file(path))?
            }
            FileSystemOperator::Host(host_client) => host_client.read_file(path)?,
        })
    }

    pub fn write_file(
        &self,
        path: &PathBuf,
        content: &[u8],
    ) -> Result<FileWriteResult, FileWriteError> {
        Ok(match self {
            FileSystemOperator::Ssh(ssh_client) => ssh_client.write_file(path, content)?,
            FileSystemOperator::Local(local_client) => {
                with_local_dir(|| local_client.write_file(path, content))?
            }
            FileSystemOperator::Host(host_client) => host_client.write_file(path, content)?,
        })
    }

    pub fn rename(&self, from: &PathBuf, to: &PathBuf) -> Result<(), RenameError> {
        match self {
            FileSystemOperator::Ssh(ssh_client) => ssh_client.rename_file(from, to)?,
            FileSystemOperator::Local(local_client) => {
                with_local_dir(|| local_client.rename_file(from, to))?
            }
            FileSystemOperator::Host(host_client) => host_client.rename_file(from, to)?,
        };
        Ok(())
    }

    pub fn remove_file(&self, path: &PathBuf) -> Result<(), RemoveFileError> {
        match self {
            FileSystemOperator::Ssh(ssh_client) => ssh_client.remove_file(path)?,
            FileSystemOperator::Local(local_client) => {
                with_local_dir(|| local_client.remove_file(path))?
            }
            FileSystemOperator::Host(host_client) => host_client.remove_file(path)?,
        };
        Ok(())
    }

    pub fn remove_directory(&self, path: &PathBuf) -> Result<(), RemoveDirectoryError> {
        match self {
            FileSystemOperator::Ssh(ssh_client) => ssh_client.remove_directory(path)?,
            FileSystemOperator::Local(local_client) => {
                with_local_dir(|| local_client.remove_directory(path))?
            }
            FileSystemOperator::Host(host_client) => host_client.remove_directory(path)?,
        };
        Ok(())
    }

    pub fn create_directory(&self, path: &PathBuf) -> Result<(), CreateDirectoryError> {
        match self {
            FileSystemOperator::Ssh(ssh_client) => ssh_client.create_directory(path)?,
            FileSystemOperator::Local(local_client) => {
                with_local_dir(|| local_client.create_directory(path))?
            }
            FileSystemOperator::Host(host_client) => host_client.create_directory(path)?,
        };
        Ok(())
    }

    pub fn set_permissions(&self, path: &PathBuf, mode: u32) -> Result<(), SetPermissionsError> {
        match self {
            FileSystemOperator::Ssh(ssh_client) => ssh_client.set_permissions(path, mode)?,
            FileSystemOperator::Local(local_client) => {
                with_local_dir(|| local_client.set_permissions(path, mode))?
            }
            FileSystemOperator::Host(host_client) => host_client.set_permissions(path, mode)?,
        };
        Ok(())
    }

    pub fn metadata(&self, path: &PathBuf) -> Result<Option<MetadataResult>, MetadataError> {
        Ok(match self {
            FileSystemOperator::Ssh(ssh_client) => ssh_client.metadata(path)?,
            FileSystemOperator::Local(local_client) => {
                with_local_dir(|| local_client.metadata(path))?
            }
            FileSystemOperator::Host(host_client) => host_client.metadata(path)?,
        })
    }

    pub fn file(&self, path: &PathBuf) -> Result<File, FileError> {
        let metadata = self.metadata(path)?;

        match metadata {
            None => Ok(File {
                path: path.clone(),
                file_system_operator: self.clone(),
            }),
            Some(metadata) => match metadata.r#type {
                MetadataType::File => Ok(File {
                    path: path.clone(),
                    file_system_operator: self.clone(),
                }),
                MetadataType::Directory => Err(UnexpectedDirectoryError(path.clone()))?,
                MetadataType::Unknown => Err(NotAFileError(path.clone()))?,
            },
        }
    }

    pub fn list_directory(
        &self,
        path: &PathBuf,
    ) -> Result<Vec<FileSystemEntry>, ListDirectoryError> {
        let directory_entries = match self {
            FileSystemOperator::Ssh(ssh_client) => ssh_client.list_directory(path)?,
            FileSystemOperator::Local(local_client) => {
                with_local_dir(|| local_client.list_directory(path))?
            }
            FileSystemOperator::Host(host_client) => host_client.list_directory(path)?,
        };

        let result = directory_entries
            .into_iter()
            .filter_map(|entry| match entry.r#type {
                MetadataType::File => Some(FileSystemEntry::File(File {
                    path: entry.path,
                    file_system_operator: self.clone(),
                })),
                MetadataType::Directory => Some(FileSystemEntry::Directory(Directory {
                    path: entry.path,
                    file_system_operator: self.clone(),
                })),
                MetadataType::Unknown => None,
            })
            .collect();

        Ok(result)
    }

    pub fn directory(&self, path: &PathBuf) -> Result<Directory, DirectoryError> {
        let metadata = self.metadata(path)?;

        match metadata {
            None => Ok(Directory {
                path: path.clone(),
                file_system_operator: self.clone(),
            }),
            Some(metadata) => match metadata.r#type {
                MetadataType::Directory => Ok(Directory {
                    path: path.clone(),
                    file_system_operator: self.clone(),
                }),
                MetadataType::File | MetadataType::Unknown => Err(UnexpectedTypeError {
                    path: path.clone(),
                    expected: MetadataType::Directory,
                    actual: metadata.r#type,
                })?,
            },
        }
    }

    pub fn parent_directory(&self, path: &Path) -> Result<Option<Directory>, DirectoryError> {
        let Some(parent_path) = path.parent() else {
            return Ok(None);
        };

        let metadata = self.metadata(&parent_path.to_path_buf())?;

        match metadata {
            None => Ok(Some(Directory {
                path: parent_path.to_path_buf(),
                file_system_operator: self.clone(),
            })),
            Some(metadata) => match metadata.r#type {
                MetadataType::Directory => Ok(Some(Directory {
                    path: parent_path.to_path_buf(),
                    file_system_operator: self.clone(),
                })),
                MetadataType::File | MetadataType::Unknown => Err(UnexpectedTypeError {
                    path: parent_path.to_path_buf(),
                    expected: MetadataType::Directory,
                    actual: metadata.r#type,
                })?,
            },
        }
    }

    pub fn file_name(&self, path: &Path) -> Result<Option<String>, DirectoryError> {
        Ok(path
            .file_name()
            .map(|file_name| file_name.to_string_lossy().to_string()))
    }

    pub fn set_file_name(&self, path: &PathBuf, new_name: &str) -> Result<(), RenameError> {
        let mut new_path = path.clone();
        new_path.set_file_name(new_name);

        self.rename(path, &new_path)
    }
}

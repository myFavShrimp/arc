use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use mlua::IntoLua;
use serde::Serialize;

use super::{
    error::{FfiError, OperationError},
    host::HostClient,
    local::with_local_dir,
    ssh::{ConnectionError, SshClient},
};
use crate::{
    engine::{
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

#[derive(Debug, Clone, Copy)]
pub enum Locality {
    Local,
    Remote,
}

impl Display for Locality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Locality::Local => "local",
            Locality::Remote => "remote",
        })
    }
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

    fn locality(&self) -> Locality {
        match self {
            FileSystemOperator::Ssh(_) => Locality::Remote,
            FileSystemOperator::Local(_) | FileSystemOperator::Host(_) => Locality::Local,
        }
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

#[derive(Debug, thiserror::Error)]
#[error("Failed to read {locality} file {path:?}")]
pub struct FileReadError {
    path: PathBuf,
    locality: Locality,
    #[source]
    source: OperationError,
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to write {locality} file {path:?}")]
pub struct FileWriteError {
    path: PathBuf,
    locality: Locality,
    #[source]
    source: OperationError,
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to rename {locality} file {from:?} to {to:?}")]
pub struct RenameError {
    from: PathBuf,
    to: PathBuf,
    locality: Locality,
    #[source]
    source: OperationError,
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to delete {locality} file {path:?}")]
pub struct RemoveFileError {
    path: PathBuf,
    locality: Locality,
    #[source]
    source: OperationError,
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to remove {locality} directory {path:?}")]
pub struct RemoveDirectoryError {
    path: PathBuf,
    locality: Locality,
    #[source]
    source: OperationError,
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to create {locality} directory {path:?}")]
pub struct CreateDirectoryError {
    path: PathBuf,
    locality: Locality,
    #[source]
    source: OperationError,
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to set permissions on {locality} path {path:?}")]
pub struct SetPermissionsError {
    path: PathBuf,
    locality: Locality,
    #[source]
    source: OperationError,
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to get metadata for {locality} file {path:?}")]
pub struct MetadataError {
    path: PathBuf,
    locality: Locality,
    #[source]
    source: OperationError,
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to list {locality} directory entries for {path:?}")]
pub struct DirectoryEntriesError {
    path: PathBuf,
    locality: Locality,
    #[source]
    source: OperationError,
}

#[derive(Debug, thiserror::Error)]
#[error("Invalid {locality} file path {path:?}")]
pub struct FileValidityError {
    path: PathBuf,
    locality: Locality,
    #[source]
    source: OperationError,
}

#[derive(Debug, thiserror::Error)]
#[error("Invalid {locality} directory path {path:?}")]
pub struct DirectoryValidityError {
    path: PathBuf,
    locality: Locality,
    #[source]
    source: OperationError,
}

macro_rules! delegate_ffi_error {
    ($($name:ident),* $(,)?) => {
        $(
            impl FfiError for $name {
                fn is_user_error(&self) -> bool {
                    self.source.is_user_error()
                }
            }
        )*
    };
}

delegate_ffi_error!(
    FileReadError,
    FileWriteError,
    RenameError,
    RemoveFileError,
    RemoveDirectoryError,
    CreateDirectoryError,
    SetPermissionsError,
    MetadataError,
    DirectoryEntriesError,
    FileValidityError,
    DirectoryValidityError,
);

impl FileSystemOperator {
    pub fn read_file(&self, path: &PathBuf) -> Result<Vec<u8>, FileReadError> {
        match self {
            FileSystemOperator::Ssh(ssh_client) => {
                ssh_client.read_file(path).map_err(OperationError::Remote)
            }
            FileSystemOperator::Local(local_client) => {
                with_local_dir(|| local_client.read_file(path)).map_err(OperationError::Local)
            }
            FileSystemOperator::Host(host_client) => {
                host_client.read_file(path).map_err(OperationError::Local)
            }
        }
        .map_err(|source| FileReadError {
            path: path.clone(),
            locality: self.locality(),
            source,
        })
    }

    pub fn write_file(
        &self,
        path: &PathBuf,
        content: &[u8],
    ) -> Result<FileWriteResult, FileWriteError> {
        match self {
            FileSystemOperator::Ssh(ssh_client) => ssh_client
                .write_file(path, content)
                .map_err(OperationError::Remote),
            FileSystemOperator::Local(local_client) => {
                with_local_dir(|| local_client.write_file(path, content))
                    .map_err(OperationError::Local)
            }
            FileSystemOperator::Host(host_client) => host_client
                .write_file(path, content)
                .map_err(OperationError::Local),
        }
        .map_err(|source| FileWriteError {
            path: path.clone(),
            locality: self.locality(),
            source,
        })
    }

    pub fn rename(&self, from: &PathBuf, to: &PathBuf) -> Result<(), RenameError> {
        match self {
            FileSystemOperator::Ssh(ssh_client) => ssh_client
                .rename_file(from, to)
                .map_err(OperationError::Remote),
            FileSystemOperator::Local(local_client) => {
                with_local_dir(|| local_client.rename_file(from, to)).map_err(OperationError::Local)
            }
            FileSystemOperator::Host(host_client) => host_client
                .rename_file(from, to)
                .map_err(OperationError::Local),
        }
        .map_err(|source| RenameError {
            from: from.clone(),
            to: to.clone(),
            locality: self.locality(),
            source,
        })
    }

    pub fn remove_file(&self, path: &PathBuf) -> Result<(), RemoveFileError> {
        match self {
            FileSystemOperator::Ssh(ssh_client) => {
                ssh_client.remove_file(path).map_err(OperationError::Remote)
            }
            FileSystemOperator::Local(local_client) => {
                with_local_dir(|| local_client.remove_file(path)).map_err(OperationError::Local)
            }
            FileSystemOperator::Host(host_client) => {
                host_client.remove_file(path).map_err(OperationError::Local)
            }
        }
        .map_err(|source| RemoveFileError {
            path: path.clone(),
            locality: self.locality(),
            source,
        })
    }

    pub fn remove_directory(&self, path: &PathBuf) -> Result<(), RemoveDirectoryError> {
        match self {
            FileSystemOperator::Ssh(ssh_client) => ssh_client
                .remove_directory(path)
                .map_err(OperationError::Remote),
            FileSystemOperator::Local(local_client) => {
                with_local_dir(|| local_client.remove_directory(path))
                    .map_err(OperationError::Local)
            }
            FileSystemOperator::Host(host_client) => host_client
                .remove_directory(path)
                .map_err(OperationError::Local),
        }
        .map_err(|source| RemoveDirectoryError {
            path: path.clone(),
            locality: self.locality(),
            source,
        })
    }

    pub fn create_directory(&self, path: &Path) -> Result<(), CreateDirectoryError> {
        match self {
            FileSystemOperator::Ssh(ssh_client) => ssh_client
                .create_directory(path)
                .map_err(OperationError::Remote),
            FileSystemOperator::Local(local_client) => {
                with_local_dir(|| local_client.create_directory(path))
                    .map_err(OperationError::Local)
            }
            FileSystemOperator::Host(host_client) => host_client
                .create_directory(path)
                .map_err(OperationError::Local),
        }
        .map_err(|source| CreateDirectoryError {
            path: path.to_path_buf(),
            locality: self.locality(),
            source,
        })
    }

    pub fn set_permissions(&self, path: &Path, mode: u32) -> Result<(), SetPermissionsError> {
        match self {
            FileSystemOperator::Ssh(ssh_client) => ssh_client
                .set_permissions(path, mode)
                .map_err(OperationError::Remote),
            FileSystemOperator::Local(local_client) => {
                with_local_dir(|| local_client.set_permissions(path, mode))
                    .map_err(OperationError::Local)
            }
            FileSystemOperator::Host(host_client) => host_client
                .set_permissions(path, mode)
                .map_err(OperationError::Local),
        }
        .map_err(|source| SetPermissionsError {
            path: path.to_path_buf(),
            locality: self.locality(),
            source,
        })
    }

    pub fn metadata(&self, path: &Path) -> Result<Option<MetadataResult>, MetadataError> {
        match self {
            FileSystemOperator::Ssh(ssh_client) => {
                ssh_client.metadata(path).map_err(OperationError::Remote)
            }
            FileSystemOperator::Local(local_client) => {
                with_local_dir(|| local_client.metadata(path)).map_err(OperationError::Local)
            }
            FileSystemOperator::Host(host_client) => {
                host_client.metadata(path).map_err(OperationError::Local)
            }
        }
        .map_err(|source| MetadataError {
            path: path.to_path_buf(),
            locality: self.locality(),
            source,
        })
    }

    pub fn file(&self, path: &Path) -> Result<File, FileValidityError> {
        match self {
            FileSystemOperator::Ssh(ssh_client) => ssh_client
                .check_file_validity(path)
                .map_err(OperationError::Remote),
            FileSystemOperator::Local(local_client) => {
                with_local_dir(|| local_client.check_file_validity(path))
                    .map_err(OperationError::Local)
            }
            FileSystemOperator::Host(host_client) => host_client
                .check_file_validity(path)
                .map_err(OperationError::Local),
        }
        .map_err(|source| FileValidityError {
            path: path.to_path_buf(),
            locality: self.locality(),
            source,
        })?;

        Ok(File {
            path: path.to_path_buf(),
            file_system_operator: self.clone(),
        })
    }

    pub fn list_directory(
        &self,
        path: &Path,
    ) -> Result<Vec<FileSystemEntry>, DirectoryEntriesError> {
        let directory_entries = match self {
            FileSystemOperator::Ssh(ssh_client) => ssh_client
                .list_directory(path)
                .map_err(OperationError::Remote),
            FileSystemOperator::Local(local_client) => {
                with_local_dir(|| local_client.list_directory(path)).map_err(OperationError::Local)
            }
            FileSystemOperator::Host(host_client) => host_client
                .list_directory(path)
                .map_err(OperationError::Local),
        }
        .map_err(|source| DirectoryEntriesError {
            path: path.to_path_buf(),
            locality: self.locality(),
            source,
        })?;

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

    pub fn directory(&self, path: &Path) -> Result<Directory, DirectoryValidityError> {
        match self {
            FileSystemOperator::Ssh(ssh_client) => ssh_client
                .check_directory_validity(path)
                .map_err(OperationError::Remote),
            FileSystemOperator::Local(local_client) => {
                with_local_dir(|| local_client.check_directory_validity(path))
                    .map_err(OperationError::Local)
            }
            FileSystemOperator::Host(host_client) => host_client
                .check_directory_validity(path)
                .map_err(OperationError::Local),
        }
        .map_err(|source| DirectoryValidityError {
            path: path.to_path_buf(),
            locality: self.locality(),
            source,
        })?;

        Ok(Directory {
            path: path.to_path_buf(),
            file_system_operator: self.clone(),
        })
    }

    pub fn parent_directory(
        &self,
        path: &Path,
    ) -> Result<Option<Directory>, DirectoryValidityError> {
        let Some(parent_path) = path.parent() else {
            return Ok(None);
        };

        match self {
            FileSystemOperator::Ssh(ssh_client) => ssh_client
                .check_directory_validity(parent_path)
                .map_err(OperationError::Remote),
            FileSystemOperator::Local(local_client) => {
                with_local_dir(|| local_client.check_directory_validity(parent_path))
                    .map_err(OperationError::Local)
            }
            FileSystemOperator::Host(host_client) => host_client
                .check_directory_validity(parent_path)
                .map_err(OperationError::Local),
        }
        .map_err(|source| DirectoryValidityError {
            path: parent_path.to_path_buf(),
            locality: self.locality(),
            source,
        })?;

        Ok(Some(Directory {
            path: parent_path.to_path_buf(),
            file_system_operator: self.clone(),
        }))
    }

    pub fn file_name(&self, path: &Path) -> Option<String> {
        path.file_name()
            .map(|file_name| file_name.to_string_lossy().to_string())
    }

    pub fn set_file_name(&self, path: &PathBuf, new_name: &str) -> Result<(), RenameError> {
        let mut new_path = path.clone();
        new_path.set_file_name(new_name);

        self.rename(path, &new_path)
    }
}

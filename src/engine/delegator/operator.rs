use std::{
    fmt::Display,
    io::Read,
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
    error::ErrorReport,
    memory::target_systems::{TargetSystem, TargetSystemKind},
    progress::{ProgressContext, TransferDirection, TransferProgress},
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
            FileSystemEntry::Directory(directory) => {
                Ok(mlua::Value::UserData(lua.create_userdata(directory)?))
            }
        }
    }
}

#[derive(Clone)]
pub struct FileSystemOperator {
    kind: FileSystemOperatorKind,
    progress: ProgressContext,
}

#[derive(Clone)]
enum FileSystemOperatorKind {
    Ssh(SshClient),
    Local(HostClient, PathBuf),
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
    pub fn new_for_system(
        config: &TargetSystem,
        progress: ProgressContext,
        home_path: PathBuf,
    ) -> Result<Self, OperationTargetSetError> {
        Ok(match &config.kind {
            TargetSystemKind::Remote(remote_target_system) => Self {
                kind: FileSystemOperatorKind::Ssh(SshClient::connect(remote_target_system)?),
                progress,
            },
            TargetSystemKind::Local => Self::new_local(progress, home_path),
        })
    }

    pub fn new_local(progress: ProgressContext, home_path: PathBuf) -> Self {
        Self {
            kind: FileSystemOperatorKind::Local(HostClient, home_path),
            progress,
        }
    }

    pub fn new_host(progress: ProgressContext) -> Self {
        Self {
            kind: FileSystemOperatorKind::Host(HostClient),
            progress,
        }
    }

    fn locality(&self) -> Locality {
        match &self.kind {
            FileSystemOperatorKind::Ssh(_) => Locality::Remote,
            FileSystemOperatorKind::Local(_, _) | FileSystemOperatorKind::Host(_) => {
                Locality::Local
            }
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

        let result_table = set_readonly(lua, result_table).map_err(|error| {
            mlua::Error::RuntimeError(ErrorReport::boxed_from(error).build_report())
        })?;

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

        let result_table = set_readonly(lua, result_table).map_err(|error| {
            mlua::Error::RuntimeError(ErrorReport::boxed_from(error).build_report())
        })?;

        Ok(mlua::Value::Table(result_table))
    }
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to set execution target")]
pub enum OperationTargetSetError {
    Connection(#[from] ConnectionError),
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

#[derive(Debug, thiserror::Error)]
#[error(
    "Failed to stream {source_locality} file {source_path:?} to {target_locality} file {target_path:?}"
)]
pub struct FileStreamError {
    source_path: PathBuf,
    source_locality: Locality,
    target_path: PathBuf,
    target_locality: Locality,
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
    FileStreamError,
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
        match &self.kind {
            FileSystemOperatorKind::Ssh(ssh_client) => self
                .progress
                .transfer(
                    TransferDirection::Download {
                        source_file_path: path.to_string_lossy().into_owned(),
                        target_file_path: None,
                    },
                    ssh_client.file_size(path).unwrap_or(0),
                )
                .map_err(OperationError::Progress)
                .and_then(|progress| {
                    ssh_client
                        .read_file(path, &progress)
                        .map_err(OperationError::Remote)
                }),
            FileSystemOperatorKind::Local(local_client, home_path) => {
                with_local_dir(home_path, || local_client.read_file(path))
                    .map_err(OperationError::Local)
            }
            FileSystemOperatorKind::Host(host_client) => {
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
        match &self.kind {
            FileSystemOperatorKind::Ssh(ssh_client) => self
                .progress
                .transfer(
                    TransferDirection::Upload {
                        source_file_path: None,
                        target_file_path: path.to_string_lossy().into_owned(),
                    },
                    content.len() as u64,
                )
                .map_err(OperationError::Progress)
                .and_then(|progress| {
                    ssh_client
                        .write_file(path, content, &progress)
                        .map_err(OperationError::Remote)
                }),
            FileSystemOperatorKind::Local(local_client, home_path) => {
                with_local_dir(home_path, || local_client.write_file(path, content))
                    .map_err(OperationError::Local)
            }
            FileSystemOperatorKind::Host(host_client) => host_client
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
        match &self.kind {
            FileSystemOperatorKind::Ssh(ssh_client) => ssh_client
                .rename_file(from, to)
                .map_err(OperationError::Remote),
            FileSystemOperatorKind::Local(local_client, home_path) => {
                with_local_dir(home_path, || local_client.rename_file(from, to))
                    .map_err(OperationError::Local)
            }
            FileSystemOperatorKind::Host(host_client) => host_client
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
        match &self.kind {
            FileSystemOperatorKind::Ssh(ssh_client) => {
                ssh_client.remove_file(path).map_err(OperationError::Remote)
            }
            FileSystemOperatorKind::Local(local_client, home_path) => {
                with_local_dir(home_path, || local_client.remove_file(path))
                    .map_err(OperationError::Local)
            }
            FileSystemOperatorKind::Host(host_client) => {
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
        match &self.kind {
            FileSystemOperatorKind::Ssh(ssh_client) => ssh_client
                .remove_directory(path)
                .map_err(OperationError::Remote),
            FileSystemOperatorKind::Local(local_client, home_path) => {
                with_local_dir(home_path, || local_client.remove_directory(path))
                    .map_err(OperationError::Local)
            }
            FileSystemOperatorKind::Host(host_client) => host_client
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
        match &self.kind {
            FileSystemOperatorKind::Ssh(ssh_client) => ssh_client
                .create_directory(path)
                .map_err(OperationError::Remote),
            FileSystemOperatorKind::Local(local_client, home_path) => {
                with_local_dir(home_path, || local_client.create_directory(path))
                    .map_err(OperationError::Local)
            }
            FileSystemOperatorKind::Host(host_client) => host_client
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
        match &self.kind {
            FileSystemOperatorKind::Ssh(ssh_client) => ssh_client
                .set_permissions(path, mode)
                .map_err(OperationError::Remote),
            FileSystemOperatorKind::Local(local_client, home_path) => {
                with_local_dir(home_path, || local_client.set_permissions(path, mode))
                    .map_err(OperationError::Local)
            }
            FileSystemOperatorKind::Host(host_client) => host_client
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
        match &self.kind {
            FileSystemOperatorKind::Ssh(ssh_client) => {
                ssh_client.metadata(path).map_err(OperationError::Remote)
            }
            FileSystemOperatorKind::Local(local_client, home_path) => {
                with_local_dir(home_path, || local_client.metadata(path))
                    .map_err(OperationError::Local)
            }
            FileSystemOperatorKind::Host(host_client) => {
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
        match &self.kind {
            FileSystemOperatorKind::Ssh(ssh_client) => ssh_client
                .check_file_validity(path)
                .map_err(OperationError::Remote),
            FileSystemOperatorKind::Local(local_client, home_path) => {
                with_local_dir(home_path, || local_client.check_file_validity(path))
                    .map_err(OperationError::Local)
            }
            FileSystemOperatorKind::Host(host_client) => host_client
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
        let directory_entries = match &self.kind {
            FileSystemOperatorKind::Ssh(ssh_client) => ssh_client
                .list_directory(path)
                .map_err(OperationError::Remote),
            FileSystemOperatorKind::Local(local_client, home_path) => {
                with_local_dir(home_path, || local_client.list_directory(path))
                    .map_err(OperationError::Local)
            }
            FileSystemOperatorKind::Host(host_client) => host_client
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
        match &self.kind {
            FileSystemOperatorKind::Ssh(ssh_client) => ssh_client
                .check_directory_validity(path)
                .map_err(OperationError::Remote),
            FileSystemOperatorKind::Local(local_client, home_path) => {
                with_local_dir(home_path, || local_client.check_directory_validity(path))
                    .map_err(OperationError::Local)
            }
            FileSystemOperatorKind::Host(host_client) => host_client
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

        match &self.kind {
            FileSystemOperatorKind::Ssh(ssh_client) => ssh_client
                .check_directory_validity(parent_path)
                .map_err(OperationError::Remote),
            FileSystemOperatorKind::Local(local_client, home_path) => {
                with_local_dir(home_path, || {
                    local_client.check_directory_validity(parent_path)
                })
                .map_err(OperationError::Local)
            }
            FileSystemOperatorKind::Host(host_client) => host_client
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

    fn get_file_size(&self, path: &PathBuf) -> u64 {
        match &self.kind {
            FileSystemOperatorKind::Ssh(ssh_client) => ssh_client.file_size(path).unwrap_or(0),
            FileSystemOperatorKind::Local(_, home_path) => with_local_dir(home_path, || {
                Ok::<u64, std::convert::Infallible>(
                    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0),
                )
            })
            .unwrap_or(0),
            FileSystemOperatorKind::Host(_) => {
                std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
            }
        }
    }

    pub fn stream_to_other(
        &self,
        source_path: &PathBuf,
        target: &FileSystemOperator,
        target_path: &Path,
    ) -> Result<FileWriteResult, FileStreamError> {
        let source_size = self.get_file_size(source_path);

        let source_file_path = source_path.to_string_lossy().into_owned();
        let target_file_path = target_path.to_string_lossy().into_owned();

        let progress = match (&self.kind, &target.kind) {
            (FileSystemOperatorKind::Ssh(_), FileSystemOperatorKind::Ssh(_))
            | (
                FileSystemOperatorKind::Local(_, _) | FileSystemOperatorKind::Host(_),
                FileSystemOperatorKind::Local(_, _) | FileSystemOperatorKind::Host(_),
            ) => self.progress.transfer(
                TransferDirection::Copy {
                    source_file_path,
                    target_file_path,
                },
                source_size,
            ),
            (
                FileSystemOperatorKind::Ssh(_),
                FileSystemOperatorKind::Local(_, _) | FileSystemOperatorKind::Host(_),
            ) => self.progress.transfer(
                TransferDirection::Download {
                    source_file_path,
                    target_file_path: Some(target_file_path),
                },
                source_size,
            ),
            (
                FileSystemOperatorKind::Local(_, _) | FileSystemOperatorKind::Host(_),
                FileSystemOperatorKind::Ssh(_),
            ) => target.progress.transfer(
                TransferDirection::Upload {
                    source_file_path: Some(source_file_path),
                    target_file_path,
                },
                source_size,
            ),
        }
        .map_err(|source| FileStreamError {
            source_path: source_path.clone(),
            source_locality: self.locality(),
            target_path: target_path.to_path_buf(),
            target_locality: target.locality(),
            source: OperationError::Progress(source),
        })?;

        let result = match &self.kind {
            FileSystemOperatorKind::Ssh(ssh_client) => ssh_client
                .open_reader(source_path)
                .map_err(OperationError::Remote)
                .and_then(|mut reader| {
                    write_reader_to_writer(&mut reader, target, target_path, &progress)
                }),
            FileSystemOperatorKind::Local(host_client, home_path) => {
                with_local_dir(home_path, || host_client.open_reader(source_path))
                    .map_err(OperationError::Local)
                    .and_then(|mut reader| {
                        write_reader_to_writer(&mut reader, target, target_path, &progress)
                    })
            }
            FileSystemOperatorKind::Host(host_client) => host_client
                .open_reader(source_path)
                .map_err(OperationError::Local)
                .and_then(|mut reader| {
                    write_reader_to_writer(&mut reader, target, target_path, &progress)
                }),
        };

        progress.finish();

        result.map_err(|source| FileStreamError {
            source_path: source_path.clone(),
            source_locality: self.locality(),
            target_path: target_path.to_path_buf(),
            target_locality: target.locality(),
            source,
        })
    }
}

fn write_reader_to_writer(
    reader: &mut dyn Read,
    target: &FileSystemOperator,
    target_path: &Path,
    progress: &TransferProgress,
) -> Result<FileWriteResult, OperationError> {
    match &target.kind {
        FileSystemOperatorKind::Ssh(ssh_client) => ssh_client
            .write_from_reader(target_path, reader, progress)
            .map_err(OperationError::Remote),
        FileSystemOperatorKind::Local(host_client, home_path) => with_local_dir(home_path, || {
            host_client.write_from_reader(target_path, reader, progress)
        })
        .map_err(OperationError::Local),
        FileSystemOperatorKind::Host(host_client) => host_client
            .write_from_reader(target_path, reader, progress)
            .map_err(OperationError::Local),
    }
}

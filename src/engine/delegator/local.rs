use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

use super::{
    executor::CommandResult,
    operator::{FileWriteResult, MetadataResult, MetadataType},
};

#[derive(Clone)]
pub struct LocalClient;

#[derive(thiserror::Error, Debug)]
#[error("Failed to perform local operation")]
pub enum HostError {
    Io(#[from] std::io::Error),
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to read local file {path:?}")]
pub struct FileReadError {
    path: PathBuf,
    #[source]
    kind: FileReadErrorKind,
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum FileReadErrorKind {
    Io(#[from] std::io::Error),
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to write local file {path:?}")]
pub struct FileWriteError {
    path: PathBuf,
    #[source]
    kind: FileWriteErrorKind,
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum FileWriteErrorKind {
    Io(#[from] std::io::Error),
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to rename local file {from:?} to {to:?}")]
pub struct RenameError {
    from: PathBuf,
    to: PathBuf,
    #[source]
    kind: RenameErrorKind,
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum RenameErrorKind {
    Io(#[from] std::io::Error),
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to delete local file {path:?}")]
pub struct RemoveFileError {
    path: PathBuf,
    #[source]
    source: std::io::Error,
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to remove local directory {path:?}")]
pub struct RemoveDirectoryError {
    path: PathBuf,
    #[source]
    source: std::io::Error,
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to create local directory {path:?}")]
pub struct CreateDirectoryError {
    path: PathBuf,
    #[source]
    source: std::io::Error,
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to set permissions on local path {path:?}")]
pub struct SetPermissionsError {
    path: PathBuf,
    #[source]
    source: std::io::Error,
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to list directory entries for remote file {path:?}")]
pub struct DirectoryEntriesError {
    path: PathBuf,
    #[source]
    source: std::io::Error,
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to get metadata for local file {path:?}")]
pub struct MetadataError {
    path: PathBuf,
    #[source]
    source: std::io::Error,
}

impl LocalClient {
    pub fn execute_command(&self, command: &str) -> Result<CommandResult, HostError> {
        let output = Command::new("sh").arg("-c").arg(command).output()?;

        Ok(CommandResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }

    pub fn read_file(&self, path: &PathBuf) -> Result<Vec<u8>, FileReadError> {
        std::fs::read(path).map_err(|e| FileReadError {
            path: path.clone(),
            kind: FileReadErrorKind::Io(e),
        })
    }

    pub fn write_file(
        &self,
        path: &PathBuf,
        content: &[u8],
    ) -> Result<FileWriteResult, FileWriteError> {
        std::fs::write(path, content).map_err(|e| FileWriteError {
            path: path.clone(),
            kind: FileWriteErrorKind::Io(e),
        })?;

        Ok(FileWriteResult {
            path: path.clone(),
            bytes_written: content.len(),
        })
    }

    pub fn rename_file(&self, from: &PathBuf, to: &PathBuf) -> Result<(), RenameError> {
        std::fs::rename(from, to).map_err(|e| RenameError {
            from: from.clone(),
            to: to.clone(),
            kind: RenameErrorKind::Io(e),
        })
    }

    pub fn remove_file(&self, path: &PathBuf) -> Result<(), RemoveFileError> {
        std::fs::remove_file(path).map_err(|e| RemoveFileError {
            path: path.clone(),
            source: e,
        })
    }

    pub fn remove_directory(&self, path: &PathBuf) -> Result<(), RemoveDirectoryError> {
        std::fs::remove_dir_all(path).map_err(|e| RemoveDirectoryError {
            path: path.clone(),
            source: e,
        })
    }

    pub fn create_directory(&self, path: &PathBuf) -> Result<(), CreateDirectoryError> {
        std::fs::create_dir_all(path).map_err(|e| CreateDirectoryError {
            path: path.clone(),
            source: e,
        })
    }

    pub fn set_permissions(&self, path: &PathBuf, mode: u32) -> Result<(), SetPermissionsError> {
        let perms = std::fs::Permissions::from_mode(mode);
        std::fs::set_permissions(path, perms).map_err(|e| SetPermissionsError {
            path: path.clone(),
            source: e,
        })
    }

    pub fn list_directory(
        &self,
        path: &PathBuf,
    ) -> Result<Vec<MetadataResult>, DirectoryEntriesError> {
        let entries = std::fs::read_dir(path).map_err(|e| DirectoryEntriesError {
            path: path.clone(),
            source: e,
        })?;

        let mut result = Vec::new();

        for entry in entries {
            let entry = entry.map_err(|e| DirectoryEntriesError {
                path: path.clone(),
                source: e,
            })?;

            let path = entry.path();
            let metadata = entry
                .metadata()
                .map_err(|e| MetadataError {
                    path: path.clone(),
                    source: e,
                })
                .map_err(|e| DirectoryEntriesError {
                    path: path.clone(),
                    source: std::io::Error::other(e),
                })?;

            let file_type = metadata.file_type();
            let r#type = if file_type.is_file() {
                MetadataType::File
            } else if file_type.is_dir() {
                MetadataType::Directory
            } else {
                MetadataType::Unknown
            };

            result.push(MetadataResult {
                path: path.clone(),
                size: Some(metadata.len()),
                permissions: Some(metadata.permissions().mode() & 0o777),
                r#type,
                uid: None, // Would need nix crate to get this
                gid: None, // Would need nix crate to get this
                accessed: metadata
                    .accessed()
                    .ok()
                    .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()),
                modified: metadata
                    .modified()
                    .ok()
                    .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()),
            });
        }

        Ok(result)
    }

    pub fn metadata(&self, path: &PathBuf) -> Result<Option<MetadataResult>, MetadataError> {
        match std::fs::metadata(path) {
            Ok(metadata) => {
                let file_type = metadata.file_type();
                let r#type = if file_type.is_file() {
                    MetadataType::File
                } else if file_type.is_dir() {
                    MetadataType::Directory
                } else {
                    MetadataType::Unknown
                };

                Ok(Some(MetadataResult {
                    path: path.clone(),
                    size: Some(metadata.len()),
                    permissions: Some(metadata.permissions().mode() & 0o777),
                    r#type,
                    uid: None, // Would need nix crate to get this
                    gid: None, // Would need nix crate to get this
                    accessed: metadata
                        .accessed()
                        .ok()
                        .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()),
                    modified: metadata
                        .modified()
                        .ok()
                        .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()),
                }))
            }
            Err(e) => {
                if e.kind() == std::io::ErrorKind::NotFound {
                    Ok(None)
                } else {
                    Err(MetadataError {
                        path: path.clone(),
                        source: e,
                    })
                }
            }
        }
    }
}

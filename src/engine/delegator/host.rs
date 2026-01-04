use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{
    executor::CommandResult,
    operator::{FileWriteResult, MetadataResult, MetadataType},
};

#[derive(Clone)]
pub struct HostClient;

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
    kind: CreateDirectoryErrorKind,
}

#[derive(thiserror::Error, Debug)]
pub enum CreateDirectoryErrorKind {
    #[error("path {0:?} is not a directory")]
    NotADirectory(PathBuf),
    #[error(transparent)]
    Io(#[from] std::io::Error),
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

#[derive(thiserror::Error, Debug)]
#[error("Invalid path {path:?}")]
pub struct DirectoryValidityError {
    path: PathBuf,
    #[source]
    kind: DirectoryValidityErrorKind,
}

#[derive(thiserror::Error, Debug)]
pub enum DirectoryValidityErrorKind {
    #[error("Ancestor {0:?} is not a directory")]
    AncestorNotADirectory(PathBuf),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(thiserror::Error, Debug)]
#[error("Invalid path {path:?}")]
pub struct FileValidityError {
    path: PathBuf,
    #[source]
    kind: FileValidityErrorKind,
}

#[derive(thiserror::Error, Debug)]
pub enum FileValidityErrorKind {
    #[error("Path is a directory")]
    IsADirectory,
    #[error(transparent)]
    DirectoryValidity(#[from] DirectoryValidityErrorKind),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl HostClient {
    pub fn execute_command(&self, command: &str) -> Result<CommandResult, HostError> {
        let output = Command::new("sh").arg("-c").arg(command).output()?;

        Ok(CommandResult {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }

    pub fn read_file(&self, path: &PathBuf) -> Result<Vec<u8>, FileReadError> {
        std::fs::read(path).map_err(|error| FileReadError {
            path: path.clone(),
            kind: FileReadErrorKind::Io(error),
        })
    }

    pub fn write_file(
        &self,
        path: &PathBuf,
        content: &[u8],
    ) -> Result<FileWriteResult, FileWriteError> {
        std::fs::write(path, content).map_err(|error| FileWriteError {
            path: path.clone(),
            kind: FileWriteErrorKind::Io(error),
        })?;

        Ok(FileWriteResult {
            path: path.clone(),
            bytes_written: content.len(),
        })
    }

    pub fn rename_file(&self, from: &PathBuf, to: &PathBuf) -> Result<(), RenameError> {
        std::fs::rename(from, to).map_err(|error| RenameError {
            from: from.clone(),
            to: to.clone(),
            kind: RenameErrorKind::Io(error),
        })
    }

    pub fn remove_file(&self, path: &PathBuf) -> Result<(), RemoveFileError> {
        std::fs::remove_file(path).map_err(|error| RemoveFileError {
            path: path.clone(),
            source: error,
        })
    }

    pub fn remove_directory(&self, path: &PathBuf) -> Result<(), RemoveDirectoryError> {
        std::fs::remove_dir_all(path).map_err(|error| RemoveDirectoryError {
            path: path.clone(),
            source: error,
        })
    }

    pub fn create_directory(&self, path: &Path) -> Result<(), CreateDirectoryError> {
        let ancestors = path
            .ancestors()
            .filter(|p| !p.as_os_str().is_empty())
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>();

        for ancestor_path in ancestors {
            match std::fs::metadata(ancestor_path) {
                Ok(meta) if meta.is_dir() => continue,
                Ok(_) => {
                    return Err(CreateDirectoryError {
                        path: path.to_path_buf(),
                        kind: CreateDirectoryErrorKind::NotADirectory(ancestor_path.to_path_buf()),
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    std::fs::create_dir(ancestor_path).map_err(|error| CreateDirectoryError {
                        path: path.to_path_buf(),
                        kind: error.into(),
                    })?;
                }
                Err(error) => {
                    return Err(CreateDirectoryError {
                        path: path.to_path_buf(),
                        kind: error.into(),
                    });
                }
            }
        }

        Ok(())
    }

    pub fn set_permissions(&self, path: &Path, mode: u32) -> Result<(), SetPermissionsError> {
        let perms = std::fs::Permissions::from_mode(mode);
        std::fs::set_permissions(path, perms).map_err(|error| SetPermissionsError {
            path: path.to_path_buf(),
            source: error,
        })
    }

    pub fn list_directory(
        &self,
        path: &Path,
    ) -> Result<Vec<MetadataResult>, DirectoryEntriesError> {
        let entries = std::fs::read_dir(path).map_err(|error| DirectoryEntriesError {
            path: path.to_path_buf(),
            source: error,
        })?;

        let mut result = Vec::new();

        for entry in entries {
            let entry = entry.map_err(|e| DirectoryEntriesError {
                path: path.to_path_buf(),
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

    pub fn metadata(&self, path: &Path) -> Result<Option<MetadataResult>, MetadataError> {
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
                    path: path.to_path_buf(),
                    size: Some(metadata.len()),
                    permissions: Some(metadata.permissions().mode() & 0o777),
                    r#type,
                    uid: None, // Would need nix crate to get this
                    gid: None, // Would need nix crate to get this
                    accessed: metadata.accessed().ok().map(|time| {
                        time.duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs()
                    }),
                    modified: metadata.modified().ok().map(|time| {
                        time.duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs()
                    }),
                }))
            }
            Err(error) => {
                if error.kind() == std::io::ErrorKind::NotFound {
                    Ok(None)
                } else {
                    Err(MetadataError {
                        path: path.to_path_buf(),
                        source: error,
                    })
                }
            }
        }
    }

    pub fn check_directory_validity(&self, path: &Path) -> Result<(), DirectoryValidityError> {
        let ancestors = path
            .ancestors()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>();

        for ancestor in ancestors {
            if ancestor.as_os_str().is_empty() {
                break;
            }

            match std::fs::metadata(ancestor) {
                Ok(meta) if meta.is_dir() => continue,
                Ok(_) => {
                    return Err(DirectoryValidityError {
                        path: path.to_path_buf(),
                        kind: DirectoryValidityErrorKind::AncestorNotADirectory(
                            ancestor.to_path_buf(),
                        ),
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
                Err(error) => {
                    return Err(DirectoryValidityError {
                        path: path.to_path_buf(),
                        kind: error.into(),
                    });
                }
            }
        }

        Ok(())
    }

    pub fn check_file_validity(&self, path: &Path) -> Result<(), FileValidityError> {
        if let Some(parent_path) = path.parent() {
            self.check_directory_validity(parent_path)
                .map_err(|error| FileValidityError {
                    path: path.to_path_buf(),
                    kind: error.kind.into(),
                })?;
        }

        match std::fs::metadata(path) {
            Ok(meta) if meta.is_dir() => {
                return Err(FileValidityError {
                    path: path.to_path_buf(),
                    kind: FileValidityErrorKind::IsADirectory,
                });
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(FileValidityError {
                    path: path.to_path_buf(),
                    kind: error.into(),
                });
            }
        }

        Ok(())
    }
}

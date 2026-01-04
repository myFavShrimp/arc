use ssh2::{Session, Sftp};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::{
    executor::CommandResult,
    operator::{FileWriteResult, MetadataResult, MetadataType},
};
use crate::memory::target_systems::RemoteTargetSystem;

#[derive(Clone)]
pub struct SshClient {
    session: Session,
    sftp: Arc<Sftp>,
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to connect")]
pub enum ConnectionError {
    TcpConnection(#[source] std::io::Error),
    Ssh(#[from] ssh2::Error),
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to perform ssh operation")]
pub enum SshError {
    Io(#[from] std::io::Error),
    Ssh(#[from] ssh2::Error),
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to read remote file {path:?}")]
pub struct FileReadError {
    path: PathBuf,
    #[source]
    kind: FileReadErrorKind,
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum FileReadErrorKind {
    Io(#[from] std::io::Error),
    Ssh(#[from] ssh2::Error),
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to write remote file {path:?}")]
pub struct FileWriteError {
    path: PathBuf,
    #[source]
    kind: FileWriteErrorKind,
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum FileWriteErrorKind {
    Io(#[from] std::io::Error),
    Ssh(#[from] ssh2::Error),
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to rename remote file {from:?} to {to:?}")]
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
    Ssh(#[from] ssh2::Error),
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to delete remote file {path:?}")]
pub struct RemoveFileError {
    path: PathBuf,
    #[source]
    source: ssh2::Error,
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to remove remote directory {path:?}")]
pub struct RemoveDirectoryError {
    path: PathBuf,
    #[source]
    source: ssh2::Error,
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to create remote directory {path:?}")]
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
    Ssh(#[from] ssh2::Error),
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to set permissions on remote path {path:?}")]
pub struct SetPermissionsError {
    path: PathBuf,
    #[source]
    source: ssh2::Error,
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to list directory entries for remote file {path:?}")]
pub struct DirectoryEntriesError {
    path: PathBuf,
    #[source]
    source: ssh2::Error,
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to get metadata for remote file {path:?}")]
pub struct MetadataError {
    path: PathBuf,
    #[source]
    source: ssh2::Error,
}

#[derive(thiserror::Error, Debug)]
#[error("Invalid path {path:?}")]
pub struct DirectoryValidityError {
    path: PathBuf,
    #[source]
    pub kind: DirectoryValidityErrorKind,
}

#[derive(thiserror::Error, Debug)]
pub enum DirectoryValidityErrorKind {
    #[error("Ancestor {0:?} is not a directory")]
    AncestorNotADirectory(PathBuf),
    #[error(transparent)]
    Ssh(#[from] ssh2::Error),
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
    Ssh(#[from] ssh2::Error),
}

const SFTP_ERROR_CODE_NO_SUCH_FILE: i32 = 2;
const SSH_SESSION_ERROR_CODE_FILE_ERROR: i32 = -16;

impl SshClient {
    pub fn connect(system: &RemoteTargetSystem) -> Result<Self, ConnectionError> {
        let tcp =
            TcpStream::connect(system.socket_address()).map_err(ConnectionError::TcpConnection)?;

        let mut session = Session::new()?;
        session.set_tcp_stream(tcp);
        session.handshake()?;

        // Try to authenticate without agent (e.g. without credentials)
        session.auth_methods(&system.user)?;

        if !session.authenticated() {
            session.userauth_agent(&system.user)?;
        }

        let sftp = Arc::new(session.sftp()?);

        Ok(Self { session, sftp })
    }

    pub fn execute_command(&self, command: &str) -> Result<CommandResult, SshError> {
        let mut channel = self.session.channel_session()?;
        channel.exec(command)?;

        let mut stdout = String::new();
        channel.read_to_string(&mut stdout)?;

        let mut stderr = String::new();
        channel.stderr().read_to_string(&mut stderr)?;

        channel.close()?;
        let exit_code = channel.exit_status()?;

        Ok(CommandResult {
            stdout,
            stderr,
            exit_code,
        })
    }

    pub fn read_file(&self, path: &PathBuf) -> Result<Vec<u8>, FileReadError> {
        let mut file = self.sftp.open(path).map_err(|error| FileReadError {
            path: path.clone(),
            kind: FileReadErrorKind::Ssh(error),
        })?;

        let mut content = Vec::new();
        file.read_to_end(&mut content)
            .map_err(|error| FileReadError {
                path: path.clone(),
                kind: FileReadErrorKind::Io(error),
            })?;

        Ok(content)
    }

    pub fn write_file(
        &self,
        path: &Path,
        content: &[u8],
    ) -> Result<FileWriteResult, FileWriteError> {
        let mut file = self.sftp.create(path).map_err(|error| FileWriteError {
            path: path.to_path_buf(),
            kind: FileWriteErrorKind::Ssh(error),
        })?;

        file.write_all(content).map_err(|error| FileWriteError {
            path: path.to_path_buf(),
            kind: FileWriteErrorKind::Io(error),
        })?;

        Ok(FileWriteResult {
            path: path.to_path_buf(),
            bytes_written: content.len(),
        })
    }

    pub fn rename_file(&self, from: &Path, to: &Path) -> Result<(), RenameError> {
        self.sftp
            .rename(from, to, None)
            .map_err(|error| RenameError {
                from: from.to_path_buf(),
                to: to.to_path_buf(),
                kind: RenameErrorKind::Ssh(error),
            })?;

        Ok(())
    }

    pub fn remove_file(&self, path: &Path) -> Result<(), RemoveFileError> {
        self.sftp.unlink(path).map_err(|error| RemoveFileError {
            path: path.to_path_buf(),
            source: error,
        })?;

        Ok(())
    }

    pub fn remove_directory(&self, path: &Path) -> Result<(), RemoveDirectoryError> {
        self.sftp
            .rmdir(path)
            .map_err(|error| RemoveDirectoryError {
                path: path.to_path_buf(),
                source: error,
            })?;

        Ok(())
    }

    pub fn create_directory(&self, path: &Path) -> Result<(), CreateDirectoryError> {
        let ancestors = path
            .ancestors()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>();

        for ancestor_path in ancestors {
            match self.sftp.stat(ancestor_path) {
                Ok(stat) if stat.is_dir() => continue,
                Ok(_) => {
                    return Err(CreateDirectoryError {
                        path: path.to_path_buf(),
                        kind: CreateDirectoryErrorKind::NotADirectory(ancestor_path.to_path_buf()),
                    });
                }
                Err(e) => match e.code() {
                    ssh2::ErrorCode::SFTP(SFTP_ERROR_CODE_NO_SUCH_FILE) => {
                        self.sftp.mkdir(ancestor_path, 0o755).map_err(|error| {
                            CreateDirectoryError {
                                path: path.to_path_buf(),
                                kind: error.into(),
                            }
                        })?;
                    }
                    _ => {
                        return Err(CreateDirectoryError {
                            path: path.to_path_buf(),
                            kind: e.into(),
                        });
                    }
                },
            }
        }

        Ok(())
    }

    pub fn set_permissions(&self, path: &Path, mode: u32) -> Result<(), SetPermissionsError> {
        let stat = ssh2::FileStat {
            size: None,
            uid: None,
            gid: None,
            perm: Some(mode),
            atime: None,
            mtime: None,
        };

        self.sftp
            .setstat(path, stat)
            .map_err(|error| SetPermissionsError {
                path: path.to_path_buf(),
                source: error,
            })?;

        Ok(())
    }

    pub fn list_directory(
        &self,
        path: &Path,
    ) -> Result<Vec<MetadataResult>, DirectoryEntriesError> {
        let mut dir = self
            .sftp
            .opendir(path)
            .map_err(|error| DirectoryEntriesError {
                path: path.to_path_buf(),
                source: error,
            })?;

        let mut entries = Vec::new();

        loop {
            match dir.readdir() {
                Ok((entry_path, stat)) => {
                    if entry_path == Path::new(".") || entry_path == Path::new("..") {
                        continue;
                    }

                    let file_type = if stat.is_dir() {
                        MetadataType::Directory
                    } else if stat.is_file() {
                        MetadataType::File
                    } else {
                        MetadataType::Unknown
                    };

                    let mut file_path = path.to_path_buf();
                    file_path.push(entry_path);

                    entries.push(MetadataResult {
                        path: file_path,
                        size: stat.size,
                        permissions: stat.perm,
                        r#type: file_type,
                        uid: stat.uid,
                        gid: stat.gid,
                        accessed: stat.atime,
                        modified: stat.mtime,
                    });
                }
                Err(error) => match error.code() {
                    ssh2::ErrorCode::Session(SSH_SESSION_ERROR_CODE_FILE_ERROR) => {
                        break;
                    }
                    ssh2::ErrorCode::SFTP(_) | ssh2::ErrorCode::Session(_) => {
                        Err(DirectoryEntriesError {
                            path: path.to_path_buf(),
                            source: error,
                        })?
                    }
                },
            }
        }

        Ok(entries)
    }

    pub fn metadata(&self, path: &Path) -> Result<Option<MetadataResult>, MetadataError> {
        let stat = match self.sftp.stat(path) {
            Ok(stat) => stat,
            Err(error) => match error.code() {
                ssh2::ErrorCode::SFTP(SFTP_ERROR_CODE_NO_SUCH_FILE) => return Ok(None),
                ssh2::ErrorCode::SFTP(_) | ssh2::ErrorCode::Session(_) => Err(MetadataError {
                    path: path.to_path_buf(),
                    source: error,
                })?,
            },
        };

        let file_type = if stat.is_dir() {
            MetadataType::Directory
        } else if stat.is_file() {
            MetadataType::File
        } else {
            MetadataType::Unknown
        };

        Ok(Some(MetadataResult {
            path: path.to_path_buf(),
            size: stat.size,
            permissions: stat.perm,
            r#type: file_type,
            uid: stat.uid,
            gid: stat.gid,
            accessed: stat.atime,
            modified: stat.mtime,
        }))
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

            match self.sftp.stat(ancestor) {
                Ok(stat) if stat.is_dir() => continue,
                Ok(_) => {
                    return Err(DirectoryValidityError {
                        path: path.to_path_buf(),
                        kind: DirectoryValidityErrorKind::AncestorNotADirectory(
                            ancestor.to_path_buf(),
                        ),
                    });
                }
                Err(error) => match error.code() {
                    ssh2::ErrorCode::SFTP(SFTP_ERROR_CODE_NO_SUCH_FILE) => break,
                    _ => {
                        return Err(DirectoryValidityError {
                            path: path.to_path_buf(),
                            kind: error.into(),
                        });
                    }
                },
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

        match self.sftp.stat(path) {
            Ok(stat) if stat.is_dir() => {
                return Err(FileValidityError {
                    path: path.to_path_buf(),
                    kind: FileValidityErrorKind::IsADirectory,
                });
            }
            Ok(_) => {}
            Err(error) => match error.code() {
                ssh2::ErrorCode::SFTP(SFTP_ERROR_CODE_NO_SUCH_FILE) => {}
                _ => {
                    return Err(FileValidityError {
                        path: path.to_path_buf(),
                        kind: error.into(),
                    });
                }
            },
        }

        Ok(())
    }
}

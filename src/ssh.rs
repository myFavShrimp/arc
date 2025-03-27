// use log::debug;
use ssh2::Session;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;

use crate::engine::system::executor::{
    CommandResult, FileReadResult, FileWriteResult, MetadataResult, MetadataType,
};
use crate::memory::target_systems::TargetSystem;

#[derive(Clone)]
pub struct SshClient {
    session: Session,
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
pub struct FileError<E: std::error::Error> {
    path: PathBuf,
    #[source]
    kind: E,
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum FileReadErrorKind {
    Io(#[from] std::io::Error),
    Ssh(#[from] ssh2::Error),
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
    source: ssh2::Error,
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to set permissions on remote path {path:?}")]
pub struct SetPermissionsError {
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

impl SshClient {
    pub fn connect(system: &TargetSystem) -> Result<Self, ConnectionError> {
        // debug!("Connecting to {}...", system.socket_address());

        let tcp =
            TcpStream::connect(system.socket_address()).map_err(ConnectionError::TcpConnection)?;

        let mut session = Session::new()?;
        session.set_tcp_stream(tcp);
        session.handshake()?;

        session.userauth_agent(&system.user)?;

        Ok(Self { session })
    }

    pub fn execute_command(&self, command: &str) -> Result<CommandResult, SshError> {
        // debug!("Executing command `{}`", command);

        let mut channel = self.session.channel_session()?;
        channel.exec(command)?;

        let mut stdout = String::new();
        channel.read_to_string(&mut stdout)?;

        let mut stderr = String::new();
        channel.stderr().read_to_string(&mut stderr)?;

        channel.close()?;
        let exit_code = channel.exit_status()?;

        // debug!("Command completed with exit code: {}", exit_code);

        Ok(CommandResult {
            stdout,
            stderr,
            exit_code,
        })
    }

    pub fn read_file(&self, path: PathBuf) -> Result<FileReadResult, FileError<FileReadErrorKind>> {
        // debug!("Reading remote file {:?}", path);

        let sftp = self.session.sftp().map_err(|e| FileError {
            path: path.clone(),
            kind: FileReadErrorKind::Ssh(e),
        })?;
        let mut file = sftp.open(&path).map_err(|e| FileError {
            path: path.clone(),
            kind: FileReadErrorKind::Ssh(e),
        })?;

        let mut content = String::new();
        file.read_to_string(&mut content).map_err(|e| FileError {
            path: path.clone(),
            kind: FileReadErrorKind::Io(e),
        })?;

        Ok(FileReadResult { path, content })
    }

    pub fn write_file(
        &self,
        path: PathBuf,
        content: &str,
    ) -> Result<FileWriteResult, FileError<FileWriteErrorKind>> {
        // debug!("Writing to remote file {:?}", path);

        let sftp = self.session.sftp().map_err(|e| FileError {
            path: path.clone(),
            kind: FileWriteErrorKind::Ssh(e),
        })?;
        let mut file = sftp.create(&path).map_err(|e| FileError {
            path: path.clone(),
            kind: FileWriteErrorKind::Ssh(e),
        })?;

        let bytes_written = file.write(content.as_bytes()).map_err(|e| FileError {
            path: path.clone(),
            kind: FileWriteErrorKind::Io(e),
        })?;

        Ok(FileWriteResult {
            path,
            bytes_written,
        })
    }

    pub fn rename_file(&self, from: PathBuf, to: PathBuf) -> Result<(), RenameError> {
        // debug!("Renaming remote file {:?} to {:?}", from, to);

        let sftp = self.session.sftp().map_err(|e| RenameError {
            from: from.clone(),
            to: to.clone(),
            kind: RenameErrorKind::Ssh(e),
        })?;
        sftp.rename(&from, &to, None).map_err(|e| RenameError {
            from: from.clone(),
            to: to.clone(),
            kind: RenameErrorKind::Ssh(e),
        })?;

        Ok(())
    }

    pub fn remove_file(&self, path: PathBuf) -> Result<(), RemoveFileError> {
        // debug!("Deleting remote file {:?}", path);

        let sftp = self.session.sftp().map_err(|e| RemoveFileError {
            path: path.clone(),
            source: e,
        })?;
        sftp.unlink(&path).map_err(|e| RemoveFileError {
            path: path.clone(),
            source: e,
        })?;

        Ok(())
    }

    pub fn remove_directory(&self, path: PathBuf) -> Result<(), RemoveDirectoryError> {
        // debug!("Removing remote directory {:?}", path);

        let sftp = self.session.sftp().map_err(|e| RemoveDirectoryError {
            path: path.clone(),
            source: e,
        })?;
        sftp.rmdir(&path).map_err(|e| RemoveDirectoryError {
            path: path.clone(),
            source: e,
        })?;

        Ok(())
    }

    pub fn create_directory(&self, path: PathBuf) -> Result<(), CreateDirectoryError> {
        // debug!("Creating remote directory {:?}", path);

        let sftp = self.session.sftp().map_err(|e| CreateDirectoryError {
            path: path.clone(),
            source: e,
        })?;
        sftp.mkdir(&path, 0o755).map_err(|e| CreateDirectoryError {
            path: path.clone(),
            source: e,
        })?;

        Ok(())
    }

    pub fn set_permissions(&self, path: PathBuf, mode: u32) -> Result<(), SetPermissionsError> {
        // debug!(
        //     "Setting permissions on remote path {:?} to {:o}",
        //     path, mode
        // );

        let sftp = self.session.sftp().map_err(|e| SetPermissionsError {
            path: path.clone(),
            source: e,
        })?;

        let stat = ssh2::FileStat {
            size: None,
            uid: None,
            gid: None,
            perm: Some(mode),
            atime: None,
            mtime: None,
        };

        sftp.setstat(&path, stat).map_err(|e| SetPermissionsError {
            path: path.clone(),
            source: e,
        })?;

        Ok(())
    }

    pub fn metadata(&self, path: PathBuf) -> Result<Option<MetadataResult>, MetadataError> {
        // debug!("Getting metadata for remote file {:?}", path);

        let sftp = self.session.sftp().map_err(|e| MetadataError {
            path: path.clone(),
            source: e,
        })?;

        let stat = match sftp.stat(&path) {
            Ok(stat) => stat,
            Err(error) => match error.code() {
                // No such file
                ssh2::ErrorCode::SFTP(2) => return Ok(None),
                ssh2::ErrorCode::SFTP(_) | ssh2::ErrorCode::Session(_) => Err(MetadataError {
                    path: path.clone(),
                    source: error,
                })?,
            },
        };

        let file_type = if stat.is_dir() {
            MetadataType::File
        } else if stat.is_file() {
            MetadataType::Directory
        } else {
            MetadataType::Unknown
        };

        Ok(Some(MetadataResult {
            path,
            size: stat.size,
            permissions: stat.perm,
            r#type: file_type,
            uid: stat.uid,
            gid: stat.gid,
            accessed: stat.atime,
            modified: stat.mtime,
        }))
    }
}

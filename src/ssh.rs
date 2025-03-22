use log::debug;
use ssh2::Session;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;

use crate::engine::system::{CommandResult, FileReadResult, FileWriteResult};
use crate::engine::targets::systems::SystemConfig;

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
    path: String,
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
    from: String,
    to: String,
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
    path: String,
    #[source]
    source: ssh2::Error,
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to remove remote directory {path:?}")]
pub struct RemoveDirectoryError {
    path: String,
    #[source]
    source: ssh2::Error,
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to create remote directory {path:?}")]
pub struct CreateDirectoryError {
    path: String,
    #[source]
    source: ssh2::Error,
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to set permissions on remote path {path:?}")]
pub struct SetPermissionsError {
    path: String,
    #[source]
    source: ssh2::Error,
}

impl SshClient {
    pub fn connect(system: &SystemConfig) -> Result<Self, ConnectionError> {
        debug!("Connecting to {}...", system.socket_address());

        let tcp =
            TcpStream::connect(system.socket_address()).map_err(ConnectionError::TcpConnection)?;

        let mut session = Session::new()?;
        session.set_tcp_stream(tcp);
        session.handshake()?;

        session.userauth_agent(&system.user)?;

        Ok(Self { session })
    }

    pub fn execute_command(&self, command: &str) -> Result<CommandResult, SshError> {
        debug!("Executing command `{}`", command);

        let mut channel = self.session.channel_session()?;
        channel.exec(command)?;

        let mut stdout = String::new();
        channel.read_to_string(&mut stdout)?;

        let mut stderr = String::new();
        channel.stderr().read_to_string(&mut stderr)?;

        channel.close()?;
        let exit_code = channel.exit_status()?;

        debug!("Command completed with exit code: {}", exit_code);

        Ok(CommandResult {
            stdout,
            stderr,
            exit_code,
        })
    }

    pub fn read_file(&self, path: &str) -> Result<FileReadResult, FileError<FileReadErrorKind>> {
        debug!("Reading remote file {:?}", path);

        let sftp = self.session.sftp().map_err(|e| FileError {
            path: path.to_string(),
            kind: FileReadErrorKind::Ssh(e),
        })?;
        let mut file = sftp.open(path).map_err(|e| FileError {
            path: path.to_string(),
            kind: FileReadErrorKind::Ssh(e),
        })?;

        let mut content = String::new();
        file.read_to_string(&mut content).map_err(|e| FileError {
            path: path.to_string(),
            kind: FileReadErrorKind::Io(e),
        })?;

        Ok(FileReadResult {
            path: path.to_string(),
            content,
        })
    }

    pub fn write_file(
        &self,
        path: &str,
        content: &str,
    ) -> Result<FileWriteResult, FileError<FileWriteErrorKind>> {
        debug!("Writing to remote file {:?}", path);

        let sftp = self.session.sftp().map_err(|e| FileError {
            path: path.to_string(),
            kind: FileWriteErrorKind::Ssh(e),
        })?;
        let mut file = sftp.create(path.as_ref()).map_err(|e| FileError {
            path: path.to_string(),
            kind: FileWriteErrorKind::Ssh(e),
        })?;

        let bytes_written = file.write(content.as_bytes()).map_err(|e| FileError {
            path: path.to_string(),
            kind: FileWriteErrorKind::Io(e),
        })?;

        Ok(FileWriteResult {
            path: path.to_string(),
            bytes_written,
        })
    }

    pub fn rename_file(&self, from: &str, to: &str) -> Result<(), RenameError> {
        debug!("Renaming remote file {} to {}", from, to);

        let sftp = self.session.sftp().map_err(|e| RenameError {
            from: from.to_string(),
            to: to.to_string(),
            kind: RenameErrorKind::Ssh(e),
        })?;
        sftp.rename(&PathBuf::from(from), &PathBuf::from(to), None)
            .map_err(|e| RenameError {
                from: from.to_string(),
                to: to.to_string(),
                kind: RenameErrorKind::Ssh(e),
            })?;

        Ok(())
    }

    pub fn remove_file(&self, path: &str) -> Result<(), RemoveFileError> {
        debug!("Deleting remote file {}", path);

        let sftp = self.session.sftp().map_err(|e| RemoveFileError {
            path: path.to_string(),
            source: e,
        })?;
        sftp.unlink(&PathBuf::from(path))
            .map_err(|e| RemoveFileError {
                path: path.to_string(),
                source: e,
            })?;

        Ok(())
    }

    pub fn remove_directory(&self, path: &str) -> Result<(), RemoveDirectoryError> {
        debug!("Removing remote directory {}", path);

        let sftp = self.session.sftp().map_err(|e| RemoveDirectoryError {
            path: path.to_string(),
            source: e,
        })?;
        sftp.rmdir(&PathBuf::from(path))
            .map_err(|e| RemoveDirectoryError {
                path: path.to_string(),
                source: e,
            })?;

        Ok(())
    }

    pub fn create_directory(&self, path: &str) -> Result<(), CreateDirectoryError> {
        debug!("Creating remote directory {}", path);

        let sftp = self.session.sftp().map_err(|e| CreateDirectoryError {
            path: path.to_string(),
            source: e,
        })?;
        sftp.mkdir(&PathBuf::from(path), 0o755)
            .map_err(|e| CreateDirectoryError {
                path: path.to_string(),
                source: e,
            })?;

        Ok(())
    }

    pub fn set_permissions(&self, path: &str, mode: u32) -> Result<(), SetPermissionsError> {
        debug!("Setting permissions on remote path {} to {:o}", path, mode);

        let sftp = self.session.sftp().map_err(|e| SetPermissionsError {
            path: path.to_string(),
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

        sftp.setstat(&PathBuf::from(path), stat)
            .map_err(|e| SetPermissionsError {
                path: path.to_string(),
                source: e,
            })?;

        Ok(())
    }
}

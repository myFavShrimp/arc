use ssh2::{Session, Sftp};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::{
    executor::CommandResult,
    operator::{FileWriteResult, MetadataResult, MetadataType},
};
use crate::engine::delegator::ssh::error::{classify_io_error, classify_ssh_error};
use crate::memory::target_systems::RemoteTargetSystem;

mod error;
use error::ExecutionError;
pub use error::{InfrastructureError, UserError};

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

    pub fn read_file(&self, path: &PathBuf) -> Result<Vec<u8>, ExecutionError> {
        let mut file = self
            .sftp
            .open(path)
            .map_err(|error| classify_ssh_error(error, path))?;

        let mut content = Vec::new();
        file.read_to_end(&mut content).map_err(classify_io_error)?;

        Ok(content)
    }

    pub fn write_file(
        &self,
        path: &Path,
        content: &[u8],
    ) -> Result<FileWriteResult, ExecutionError> {
        let mut file = self
            .sftp
            .create(path)
            .map_err(|error| classify_ssh_error(error, path))?;

        file.write_all(content).map_err(classify_io_error)?;

        Ok(FileWriteResult {
            path: path.to_path_buf(),
            bytes_written: content.len(),
        })
    }

    pub fn rename_file(&self, from: &Path, to: &Path) -> Result<(), ExecutionError> {
        self.sftp
            .rename(from, to, None)
            .map_err(|error| classify_ssh_error(error, from))
    }

    pub fn remove_file(&self, path: &Path) -> Result<(), ExecutionError> {
        self.sftp
            .unlink(path)
            .map_err(|error| classify_ssh_error(error, path))
    }

    pub fn remove_directory(&self, path: &Path) -> Result<(), ExecutionError> {
        self.sftp
            .rmdir(path)
            .map_err(|error| classify_ssh_error(error, path))
    }

    pub fn create_directory(&self, path: &Path) -> Result<(), ExecutionError> {
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
                    return Err(ExecutionError::User(UserError::NotADirectory(
                        ancestor_path.to_path_buf(),
                    )));
                }
                Err(e) => match e.code() {
                    ssh2::ErrorCode::SFTP(error::SFTP_NO_SUCH_FILE) => {
                        self.sftp
                            .mkdir(ancestor_path, 0o755)
                            .map_err(|error| classify_ssh_error(error, ancestor_path))?;
                    }
                    _ => {
                        return Err(classify_ssh_error(e, ancestor_path));
                    }
                },
            }
        }

        Ok(())
    }

    pub fn set_permissions(&self, path: &Path, mode: u32) -> Result<(), ExecutionError> {
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
            .map_err(|error| classify_ssh_error(error, path))
    }

    pub fn list_directory(&self, path: &Path) -> Result<Vec<MetadataResult>, ExecutionError> {
        let mut dir = self
            .sftp
            .opendir(path)
            .map_err(|error| classify_ssh_error(error, path))?;

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
                    ssh2::ErrorCode::Session(error::SSH_SESSION_ERROR_CODE_FILE_ERROR) => {
                        break;
                    }
                    ssh2::ErrorCode::SFTP(_) | ssh2::ErrorCode::Session(_) => {
                        return Err(classify_ssh_error(error, path));
                    }
                },
            }
        }

        Ok(entries)
    }

    pub fn metadata(&self, path: &Path) -> Result<Option<MetadataResult>, ExecutionError> {
        let stat = match self.sftp.stat(path) {
            Ok(stat) => stat,
            Err(error) => match error.code() {
                ssh2::ErrorCode::SFTP(error::SFTP_NO_SUCH_FILE) => return Ok(None),
                ssh2::ErrorCode::SFTP(_) | ssh2::ErrorCode::Session(_) => {
                    return Err(classify_ssh_error(error, path));
                }
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

    pub fn check_directory_validity(&self, path: &Path) -> Result<(), ExecutionError> {
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
                    return Err(ExecutionError::User(UserError::NotADirectory(
                        ancestor.to_path_buf(),
                    )));
                }
                Err(error) => match error.code() {
                    ssh2::ErrorCode::SFTP(error::SFTP_NO_SUCH_FILE) => break,
                    _ => {
                        return Err(classify_ssh_error(error, ancestor));
                    }
                },
            }
        }

        Ok(())
    }

    pub fn check_file_validity(&self, path: &Path) -> Result<(), ExecutionError> {
        if let Some(parent_path) = path.parent() {
            self.check_directory_validity(parent_path)?;
        }

        match self.sftp.stat(path) {
            Ok(stat) if stat.is_dir() => Err(ExecutionError::User(UserError::IsADirectory)),
            Ok(_) => Ok(()),
            Err(error) => match error.code() {
                ssh2::ErrorCode::SFTP(error::SFTP_NO_SUCH_FILE) => Ok(()),
                _ => Err(classify_ssh_error(error, path)),
            },
        }
    }
}

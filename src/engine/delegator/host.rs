use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;

use crate::engine::delegator::host::error::classify_io_error;
use crate::progress::CommandProgress;

pub mod error;

use error::ExecutionError;
pub use error::{InfrastructureError, UserError};

use super::{
    executor::CommandResult,
    operator::{FileWriteResult, MetadataResult, MetadataType},
};

#[derive(Clone)]
pub struct HostClient;

#[derive(thiserror::Error, Debug)]
#[error("Output reader thread panicked")]
pub struct OutputReaderPanicError;

#[derive(thiserror::Error, Debug)]
#[error("Output reader failed")]
pub struct OutputReaderError(pub std::io::Error);

#[derive(thiserror::Error, Debug)]
#[error("Failed to perform local operation")]
pub enum CommandError {
    Io(#[from] std::io::Error),
    OutputReaderPanic(#[from] OutputReaderPanicError),
    OutputReader(#[from] OutputReaderError),
}

impl HostClient {
    pub fn execute_command(
        &self,
        command: &str,
        progress: &CommandProgress,
    ) -> Result<CommandResult, CommandError> {
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let mut stdout_pipe = child.stdout.take().expect("command stdout has been taken");
        let mut stderr_pipe = child.stderr.take().expect("command stderr has been taken");

        let (tx, rx) = mpsc::channel::<String>();
        let tx_stderr = tx.clone();

        let stdout_thread = std::thread::spawn(move || -> std::io::Result<String> {
            let mut data = String::new();
            let mut buf = [0u8; 4096];

            loop {
                let n = stdout_pipe.read(&mut buf)?;

                if n == 0 {
                    break;
                }

                let chunk = String::from_utf8_lossy(&buf[..n]).to_string();
                data.push_str(&chunk);

                _ = tx.send(chunk);
            }

            Ok(data)
        });

        let stderr_thread = std::thread::spawn(move || -> std::io::Result<String> {
            let mut data = String::new();
            let mut buf = [0u8; 4096];

            loop {
                let n = stderr_pipe.read(&mut buf)?;

                if n == 0 {
                    break;
                }

                let chunk = String::from_utf8_lossy(&buf[..n]).to_string();
                data.push_str(&chunk);

                _ = tx_stderr.send(chunk);
            }
            Ok(data)
        });

        let mut combined = String::new();
        for chunk in rx {
            combined.push_str(&chunk);
            progress.update_output(&combined);
        }

        let stdout_data = stdout_thread
            .join()
            .map_err(|_| OutputReaderPanicError)?
            .map_err(OutputReaderError)?;
        let stderr_data = stderr_thread
            .join()
            .map_err(|_| OutputReaderPanicError)?
            .map_err(OutputReaderError)?;

        let status = child.wait()?;

        Ok(CommandResult {
            stdout: stdout_data,
            stderr: stderr_data,
            exit_code: status.code().unwrap_or(-1),
        })
    }

    pub fn read_file(&self, path: &PathBuf) -> Result<Vec<u8>, ExecutionError> {
        std::fs::read(path).map_err(|error| classify_io_error(error, path))
    }

    pub fn write_file(
        &self,
        path: &PathBuf,
        content: &[u8],
    ) -> Result<FileWriteResult, ExecutionError> {
        std::fs::write(path, content).map_err(|error| classify_io_error(error, path))?;

        Ok(FileWriteResult {
            path: path.clone(),
            bytes_written: content.len(),
        })
    }

    pub fn rename_file(&self, from: &PathBuf, to: &PathBuf) -> Result<(), ExecutionError> {
        std::fs::rename(from, to).map_err(|error| classify_io_error(error, from))
    }

    pub fn remove_file(&self, path: &PathBuf) -> Result<(), ExecutionError> {
        std::fs::remove_file(path).map_err(|error| classify_io_error(error, path))
    }

    pub fn remove_directory(&self, path: &PathBuf) -> Result<(), ExecutionError> {
        std::fs::remove_dir_all(path).map_err(|error| classify_io_error(error, path))
    }

    pub fn create_directory(&self, path: &Path) -> Result<(), ExecutionError> {
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
                    return Err(ExecutionError::User(UserError::NotADirectory(
                        ancestor_path.to_path_buf(),
                    )));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    std::fs::create_dir(ancestor_path)
                        .map_err(|error| classify_io_error(error, ancestor_path))?;
                }
                Err(error) => {
                    return Err(classify_io_error(error, ancestor_path));
                }
            }
        }

        Ok(())
    }

    pub fn set_permissions(&self, path: &Path, mode: u32) -> Result<(), ExecutionError> {
        let perms = std::fs::Permissions::from_mode(mode);
        std::fs::set_permissions(path, perms).map_err(|error| classify_io_error(error, path))
    }

    pub fn list_directory(&self, path: &Path) -> Result<Vec<MetadataResult>, ExecutionError> {
        let entries = std::fs::read_dir(path).map_err(|error| classify_io_error(error, path))?;

        let mut result = Vec::new();

        for entry in entries {
            let entry = entry.map_err(|error| classify_io_error(error, path))?;

            let entry_path = entry.path();
            let metadata = entry
                .metadata()
                .map_err(|error| classify_io_error(error, &entry_path))?;

            let file_type = metadata.file_type();
            let r#type = if file_type.is_file() {
                MetadataType::File
            } else if file_type.is_dir() {
                MetadataType::Directory
            } else {
                MetadataType::Unknown
            };

            result.push(MetadataResult {
                path: entry_path,
                size: Some(metadata.len()),
                permissions: Some(metadata.permissions().mode() & 0o777),
                r#type,
                uid: None,
                gid: None,
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

    pub fn metadata(&self, path: &Path) -> Result<Option<MetadataResult>, ExecutionError> {
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
                    uid: None,
                    gid: None,
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
                    Err(classify_io_error(error, path))
                }
            }
        }
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

            match std::fs::metadata(ancestor) {
                Ok(meta) if meta.is_dir() => continue,
                Ok(_) => {
                    return Err(ExecutionError::User(UserError::NotADirectory(
                        ancestor.to_path_buf(),
                    )));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
                Err(error) => {
                    return Err(classify_io_error(error, ancestor));
                }
            }
        }

        Ok(())
    }

    pub fn check_file_validity(&self, path: &Path) -> Result<(), ExecutionError> {
        if let Some(parent_path) = path.parent() {
            self.check_directory_validity(parent_path)?;
        }

        match std::fs::metadata(path) {
            Ok(meta) if meta.is_dir() => Err(ExecutionError::User(UserError::IsADirectory)),
            Ok(_) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(classify_io_error(error, path)),
        }
    }
}

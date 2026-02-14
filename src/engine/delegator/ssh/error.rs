use std::path::{Path, PathBuf};

use crate::engine::delegator::error::ExecutionError as GenericExecutionError;

#[derive(Debug, thiserror::Error)]
pub enum UserError {
    #[error("Not found")]
    NotFound(ssh2::Error),
    #[error("Permission denied")]
    PermissionDenied(ssh2::Error),
    #[error("Path is a directory")]
    IsADirectory,
    #[error("Not a directory: {0:?}")]
    NotADirectory(PathBuf),
    #[error("Operation failed")]
    Failure(ssh2::Error),
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum InfrastructureError {
    NeedsReconnect(Box<dyn std::error::Error + Send + Sync>),
    OtherSsh(ssh2::Error),
    OtherIo(std::io::Error),
}

pub type ExecutionError = GenericExecutionError<UserError, InfrastructureError>;

pub const SFTP_NO_SUCH_FILE: i32 = 2;
const SFTP_PERMISSION_DENIED: i32 = 3;
const SFTP_BAD_MESSAGE: i32 = 5;
const SFTP_NO_CONNECTION: i32 = 6;
const SFTP_CONNECTION_LOST: i32 = 7;

pub const SSH_SESSION_ERROR_CODE_FILE_ERROR: i32 = -16;

const LIBSSH2_ERROR_SFTP_PROTOCOL: i32 = -31;

pub fn classify_ssh_error(error: ssh2::Error, _context_path: &Path) -> ExecutionError {
    match error.code() {
        ssh2::ErrorCode::SFTP(SFTP_BAD_MESSAGE)
        | ssh2::ErrorCode::SFTP(SFTP_NO_CONNECTION)
        | ssh2::ErrorCode::SFTP(SFTP_CONNECTION_LOST) => {
            ExecutionError::Infrastructure(InfrastructureError::OtherSsh(error))
        }
        ssh2::ErrorCode::SFTP(SFTP_NO_SUCH_FILE) => {
            ExecutionError::User(UserError::NotFound(error))
        }
        ssh2::ErrorCode::SFTP(SFTP_PERMISSION_DENIED) => {
            ExecutionError::User(UserError::PermissionDenied(error))
        }

        ssh2::ErrorCode::SFTP(_) => ExecutionError::User(UserError::Failure(error)),

        // Sftp::rmdir and Sftp::symlink report SFTP failures as
        // Session(LIBSSH2_ERROR_SFTP_PROTOCOL) instead of extracting the actual SFTP status code.
        ssh2::ErrorCode::Session(LIBSSH2_ERROR_SFTP_PROTOCOL) => {
            ExecutionError::User(UserError::Failure(error))
        }

        ssh2::ErrorCode::Session(_) => {
            ExecutionError::Infrastructure(InfrastructureError::NeedsReconnect(Box::new(error)))
        }
    }
}

pub fn classify_io_error(error: std::io::Error) -> ExecutionError {
    ExecutionError::Infrastructure(InfrastructureError::OtherIo(error))
}

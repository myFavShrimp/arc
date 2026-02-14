use std::path::{Path, PathBuf};

use crate::engine::delegator::error::ExecutionError as GenericExecutionError;

#[derive(Debug, thiserror::Error)]
pub enum UserError {
    #[error("Not found")]
    NotFound(std::io::Error),
    #[error("Permission denied")]
    PermissionDenied(std::io::Error),
    #[error("Already exists")]
    AlreadyExists(std::io::Error),
    #[error("Path is a directory")]
    IsADirectory,
    #[error("Not a directory: {0:?}")]
    NotADirectory(PathBuf),
    #[error("Directory not empty")]
    DirectoryNotEmpty(std::io::Error),
    #[error("Read-only filesystem")]
    ReadOnlyFilesystem(std::io::Error),
    #[error("File too large")]
    FileTooLarge(std::io::Error),
    #[error("Invalid cross-device link")]
    CrossesDevices(std::io::Error),
    #[error("Disk quota exceeded")]
    QuotaExceeded(std::io::Error),
    #[error("Invalid filename")]
    InvalidFilename(std::io::Error),
    #[error("Resource busy")]
    ResourceBusy(std::io::Error),
    #[error("Text file busy")]
    ExecutableFileBusy(std::io::Error),
    #[error("Too many links")]
    TooManyLinks(std::io::Error),
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub struct InfrastructureError(#[from] std::io::Error);

pub type ExecutionError = GenericExecutionError<UserError, InfrastructureError>;

// TODO: maybe add string checking of kind for unstable variants?
//       like https://doc.rust-lang.org/std/io/enum.ErrorKind.html#variant.FilesystemLoop
pub fn classify_io_error(error: std::io::Error, context_path: &Path) -> ExecutionError {
    match error.kind() {
        std::io::ErrorKind::NotFound => ExecutionError::User(UserError::NotFound(error)),
        std::io::ErrorKind::PermissionDenied => {
            ExecutionError::User(UserError::PermissionDenied(error))
        }
        std::io::ErrorKind::AlreadyExists => ExecutionError::User(UserError::AlreadyExists(error)),
        std::io::ErrorKind::IsADirectory => ExecutionError::User(UserError::IsADirectory),
        std::io::ErrorKind::NotADirectory => {
            ExecutionError::User(UserError::NotADirectory(context_path.to_path_buf()))
        }
        std::io::ErrorKind::DirectoryNotEmpty => {
            ExecutionError::User(UserError::DirectoryNotEmpty(error))
        }
        std::io::ErrorKind::ReadOnlyFilesystem => {
            ExecutionError::User(UserError::ReadOnlyFilesystem(error))
        }
        std::io::ErrorKind::FileTooLarge => ExecutionError::User(UserError::FileTooLarge(error)),
        std::io::ErrorKind::CrossesDevices => {
            ExecutionError::User(UserError::CrossesDevices(error))
        }
        std::io::ErrorKind::QuotaExceeded => ExecutionError::User(UserError::QuotaExceeded(error)),
        std::io::ErrorKind::InvalidFilename => {
            ExecutionError::User(UserError::InvalidFilename(error))
        }
        std::io::ErrorKind::ResourceBusy => ExecutionError::User(UserError::ResourceBusy(error)),
        std::io::ErrorKind::ExecutableFileBusy => {
            ExecutionError::User(UserError::ExecutableFileBusy(error))
        }
        std::io::ErrorKind::TooManyLinks => ExecutionError::User(UserError::TooManyLinks(error)),

        std::io::ErrorKind::BrokenPipe
        | std::io::ErrorKind::ConnectionRefused
        | std::io::ErrorKind::ConnectionReset
        | std::io::ErrorKind::ConnectionAborted
        | std::io::ErrorKind::NotConnected
        | std::io::ErrorKind::AddrInUse
        | std::io::ErrorKind::AddrNotAvailable
        | std::io::ErrorKind::NetworkDown
        | std::io::ErrorKind::NetworkUnreachable
        | std::io::ErrorKind::HostUnreachable
        | std::io::ErrorKind::WouldBlock
        | std::io::ErrorKind::InvalidInput
        | std::io::ErrorKind::InvalidData
        | std::io::ErrorKind::TimedOut
        | std::io::ErrorKind::WriteZero
        | std::io::ErrorKind::Interrupted
        | std::io::ErrorKind::UnexpectedEof
        | std::io::ErrorKind::Unsupported
        | std::io::ErrorKind::OutOfMemory
        | std::io::ErrorKind::StaleNetworkFileHandle
        | std::io::ErrorKind::NotSeekable
        | std::io::ErrorKind::Deadlock
        | std::io::ErrorKind::ArgumentListTooLong
        | std::io::ErrorKind::Other
        | _ => ExecutionError::Infrastructure(InfrastructureError(error)),
    }
}

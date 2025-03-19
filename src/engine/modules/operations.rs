use std::path::PathBuf;

use serde::Serialize;

use crate::{
    error::MutexLockError,
    ssh::{ConnectionError, SshError},
};

use super::targets::SystemConfig;

#[derive(Debug, Serialize)]
pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

#[derive(Debug, Serialize)]
pub struct FileCopyResult {
    pub src: PathBuf,
    pub dest: PathBuf,
    pub size: usize,
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to set execution target")]
pub enum ExecutionTargetSetError {
    Connection(#[from] ConnectionError),
    Lock(#[from] MutexLockError),
}

#[derive(thiserror::Error, Debug)]
#[error("Missing execution target")]
pub struct UninitializedSshClientError;

#[derive(thiserror::Error, Debug)]
#[error("Failed to execute tasks")]
pub enum TaskError {
    Ssh(#[from] SshError),
    Lock(#[from] MutexLockError),
    UninitializedSshClient(#[from] UninitializedSshClientError),
}

pub trait OperationsModule {
    fn set_execution_target(&self, system: &SystemConfig) -> Result<(), ExecutionTargetSetError>;
    fn copy_file(&self, src: PathBuf, dest: PathBuf) -> Result<FileCopyResult, TaskError>;
    fn run_command(&self, cmd: String) -> Result<CommandResult, TaskError>;
}

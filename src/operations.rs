use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::{
    engine::modules::operations::{OperationsModule, UninitializedSshClientError},
    error::MutexLockError,
    ssh::SshClient,
};

#[derive(Default)]
pub struct OperationsExecutionModule {
    ssh_client: Arc<Mutex<Option<SshClient>>>,
}

impl OperationsModule for OperationsExecutionModule {
    fn set_execution_target(
        &self,
        system: &crate::targets::SystemConfig,
    ) -> Result<(), crate::engine::modules::operations::ExecutionTargetSetError> {
        let ssh_client = SshClient::connect(&system)?;
        let mut ssh_client_guard = self.ssh_client.lock().map_err(|_| MutexLockError)?;
        *ssh_client_guard = Some(ssh_client);

        Ok(())
    }

    fn copy_file(
        &self,
        src: std::path::PathBuf,
        dest: std::path::PathBuf,
    ) -> Result<
        crate::engine::modules::operations::FileCopyResult,
        crate::engine::modules::operations::TaskError,
    > {
        let mut guard = self.ssh_client.lock().map_err(|_| MutexLockError)?;

        let command_result = guard
            .as_mut()
            .ok_or(UninitializedSshClientError)?
            .copy_file(PathBuf::from(src), PathBuf::from(dest))?;

        Ok(command_result)
    }

    fn run_command(
        &self,
        cmd: String,
    ) -> Result<
        crate::engine::modules::operations::CommandResult,
        crate::engine::modules::operations::TaskError,
    > {
        let mut guard = self.ssh_client.lock().map_err(|_| MutexLockError)?;

        let command_result = guard
            .as_mut()
            .ok_or(UninitializedSshClientError)?
            .execute_command(&cmd)?;

        Ok(command_result)
    }
}

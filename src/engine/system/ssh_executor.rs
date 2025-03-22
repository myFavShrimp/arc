use std::sync::{Arc, Mutex};

use crate::{error::MutexLockError, ssh::SshClient};

use super::Executor;

#[derive(Clone)]
pub struct SshExecutor {
    ssh_client: Arc<Mutex<SshClient>>,
}

impl SshExecutor {
    pub fn new(ssh_client: SshClient) -> Self {
        Self {
            ssh_client: Arc::new(Mutex::new(ssh_client)),
        }
    }
}

impl Executor for SshExecutor {
    fn copy_file(
        &self,
        src: std::path::PathBuf,
        dest: std::path::PathBuf,
    ) -> Result<super::FileCopyResult, super::TaskError> {
        let client = self.ssh_client.lock().map_err(|_| MutexLockError)?;

        let command_result = client.copy_file(src, dest)?;

        Ok(command_result)
    }

    fn run_command(&self, cmd: String) -> Result<super::CommandResult, super::TaskError> {
        let client = self.ssh_client.lock().map_err(|_| MutexLockError)?;

        let command_result = client.execute_command(&cmd)?;

        Ok(command_result)
    }
}

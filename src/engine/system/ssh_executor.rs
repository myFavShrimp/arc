use std::path::PathBuf;

use crate::ssh::SshClient;

use super::Executor;

#[derive(Clone)]
pub struct SshExecutor {
    ssh_client: SshClient,
}

impl SshExecutor {
    pub fn new(ssh_client: SshClient) -> Self {
        Self { ssh_client }
    }
}

impl Executor for SshExecutor {
    fn read_file(&self, path: PathBuf) -> Result<super::FileReadResult, super::FileReadError> {
        Ok(self.ssh_client.read_file(&path.to_string_lossy())?)
    }

    fn write_file(
        &self,
        path: PathBuf,
        content: String,
    ) -> Result<super::FileWriteResult, super::FileWriteError> {
        Ok(self
            .ssh_client
            .write_file(&path.to_string_lossy(), &content)?)
    }

    fn rename_file(&self, from: PathBuf, to: PathBuf) -> Result<(), super::RenameError> {
        Ok(self
            .ssh_client
            .rename_file(&from.to_string_lossy(), &to.to_string_lossy())?)
    }

    fn remove_file(&self, path: PathBuf) -> Result<(), super::RemoveFileError> {
        Ok(self.ssh_client.remove_file(&path.to_string_lossy())?)
    }

    fn remove_directory(&self, path: PathBuf) -> Result<(), super::RemoveDirectoryError> {
        Ok(self.ssh_client.remove_directory(&path.to_string_lossy())?)
    }

    fn create_directory(&self, path: PathBuf) -> Result<(), super::CreateDirectoryError> {
        Ok(self.ssh_client.create_directory(&path.to_string_lossy())?)
    }

    fn set_permissions(&self, path: PathBuf, mode: u32) -> Result<(), super::SetPermissionsError> {
        Ok(self
            .ssh_client
            .set_permissions(&path.to_string_lossy(), mode)?)
    }

    fn run_command(&self, cmd: String) -> Result<super::CommandResult, super::TaskError> {
        Ok(self.ssh_client.execute_command(&cmd)?)
    }

    fn metadata(
        &self,
        path: PathBuf,
    ) -> Result<Option<super::MetadataResult>, super::MetadataError> {
        Ok(self.ssh_client.metadata(&path)?)
    }
}

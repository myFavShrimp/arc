use log::debug;
use ssh2::Session;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;

use crate::engine::system::{CommandResult, FileCopyResult};
use crate::engine::targets::SystemConfig;

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

    pub fn copy_file(&self, src: PathBuf, dest: PathBuf) -> Result<FileCopyResult, SshError> {
        debug!(
            "Copying file from {} to remote path {}",
            src.display(),
            dest.display(),
        );

        let content = std::fs::read(&src)?;

        let mut channel = self
            .session
            .scp_send(&dest, 0o644, content.len() as u64, None)?;

        channel.write_all(&content)?;

        channel.send_eof()?;
        channel.wait_eof()?;
        channel.close()?;
        channel.wait_close()?;

        debug!("File copied successfully");

        Ok(FileCopyResult {
            src: src.to_path_buf(),
            dest: dest.to_path_buf(),
            size: content.len(),
        })
    }
}

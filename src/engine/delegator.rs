pub const TRANSFER_BUFFER_SIZE: usize = 64 * 1024 * 1024;

pub mod error;
pub mod executor;
pub mod host;
pub mod local;
pub mod operator;
mod ssh;

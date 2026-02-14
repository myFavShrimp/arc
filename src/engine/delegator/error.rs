use std::panic::resume_unwind;

use crate::engine::delegator::{host, ssh};

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct FfiPanicError(pub Box<dyn std::error::Error + Send + Sync>);

pub trait FfiError: std::error::Error + Send + Sync + Sized + 'static {
    fn is_user_error(&self) -> bool;

    fn enforce_ffi_boundary(self) -> Self {
        if !self.is_user_error() {
            resume_unwind(Box::new(FfiPanicError(Box::new(self))))
        }
        self
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OperationError {
    #[error(transparent)]
    Remote(ExecutionError<ssh::UserError, ssh::InfrastructureError>),
    #[error(transparent)]
    Local(ExecutionError<host::UserError, host::InfrastructureError>),
}

impl FfiError for OperationError {
    fn is_user_error(&self) -> bool {
        match self {
            Self::Remote(e) => e.is_user_error(),
            Self::Local(e) => e.is_user_error(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ExecutionError<U, I>
where
    U: std::error::Error + Send + Sync + 'static,
    I: std::error::Error + Send + Sync + 'static,
{
    #[error(transparent)]
    User(U),
    #[error(transparent)]
    Infrastructure(I),
}

impl<U, I> FfiError for ExecutionError<U, I>
where
    U: std::error::Error + Send + Sync + 'static,
    I: std::error::Error + Send + Sync + 'static,
{
    fn is_user_error(&self) -> bool {
        matches!(self, ExecutionError::User(_))
    }
}

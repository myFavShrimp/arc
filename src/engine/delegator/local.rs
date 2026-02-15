use std::panic::resume_unwind;
use std::path::PathBuf;

use super::error::FfiPanicError;

pub fn with_local_dir<T, E>(f: impl FnOnce() -> Result<T, E>) -> Result<T, E> {
    let original_dir = std::env::current_dir()
        .unwrap_or_else(|error| resume_unwind(Box::new(FfiPanicError(Box::new(error)))));
    let target_dir = std::env::home_dir().unwrap_or_else(|| PathBuf::from("/"));
    std::env::set_current_dir(&target_dir)
        .unwrap_or_else(|error| resume_unwind(Box::new(FfiPanicError(Box::new(error)))));

    let result = f();

    std::env::set_current_dir(&original_dir)
        .unwrap_or_else(|error| resume_unwind(Box::new(FfiPanicError(Box::new(error)))));

    result
}

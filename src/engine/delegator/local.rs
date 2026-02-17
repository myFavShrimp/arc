use std::panic::resume_unwind;
use std::path::Path;

use super::error::FfiPanicError;

pub fn with_local_dir<T, E>(home_path: &Path, f: impl FnOnce() -> Result<T, E>) -> Result<T, E> {
    let original_dir = std::env::current_dir()
        .unwrap_or_else(|error| resume_unwind(Box::new(FfiPanicError(Box::new(error)))));
    std::env::set_current_dir(home_path)
        .unwrap_or_else(|error| resume_unwind(Box::new(FfiPanicError(Box::new(error)))));

    let result = f();

    std::env::set_current_dir(&original_dir)
        .unwrap_or_else(|error| resume_unwind(Box::new(FfiPanicError(Box::new(error)))));

    result
}

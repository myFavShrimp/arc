use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum LocalError {
    Any(#[from] Box<dyn std::error::Error + Send + Sync>),
}

impl From<std::io::Error> for LocalError {
    fn from(e: std::io::Error) -> Self {
        LocalError::Any(Box::new(e))
    }
}

pub fn with_local_dir<T, E>(f: impl FnOnce() -> Result<T, E>) -> Result<T, LocalError>
where
    E: std::error::Error + Send + Sync + 'static,
{
    let original_dir = std::env::current_dir()?;
    let target_dir = std::env::home_dir().unwrap_or_else(|| PathBuf::from("/"));
    std::env::set_current_dir(&target_dir)?;

    let result = f().map_err(|e| LocalError::Any(Box::new(e)))?;

    std::env::set_current_dir(&original_dir)?;

    Ok(result)
}

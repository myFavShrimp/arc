use std::path::PathBuf;

use log::debug;
use mlua::UserData;

use crate::error::ErrorReport;

#[derive(Debug, Clone)]
pub struct FileSystem {
    root: PathBuf,
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to read file {path:?}")]
pub struct FileReadError {
    path: PathBuf,
    #[source]
    kind: FileReadErrorKind,
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum FileReadErrorKind {
    PathNotInRoot(#[from] PathNotInRootError),
    Io(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
#[error("The path is outside the arc root directory")]
pub struct PathNotInRootError;

impl FileSystem {
    pub fn new(root_directory: PathBuf) -> Self {
        Self {
            root: root_directory,
        }
    }

    fn read_file_to_string(&self, path: PathBuf) -> Result<String, FileReadError> {
        debug!("Reading file {:?}", path);

        let path = std::fs::canonicalize(path.clone()).map_err(|e| FileReadError {
            path: path.clone(),
            kind: FileReadErrorKind::Io(e),
        })?;

        if !path.starts_with(&self.root) {
            Err(FileReadError {
                path: path.clone(),
                kind: FileReadErrorKind::PathNotInRoot(PathNotInRootError),
            })?
        }

        std::fs::read_to_string(path.clone()).map_err(|e| FileReadError {
            path,
            kind: FileReadErrorKind::Io(e),
        })
    }
}

impl UserData for FileSystem {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("read_file", |_, this, path: String| {
            this.read_file_to_string(PathBuf::from(path))
                .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
        });
    }
}

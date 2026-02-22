use std::path::PathBuf;

use mlua::{MetaMethod, UserData};

use crate::{
    engine::delegator::{
        error::FfiError,
        operator::{FileReadError, FileSystemOperator},
    },
    error::ErrorReport,
};

#[derive(Clone)]
pub struct FileContent {
    pub path: PathBuf,
    pub file_system_operator: FileSystemOperator,
}

impl FileContent {
    fn materialize(&self) -> Result<Vec<u8>, FileReadError> {
        self.file_system_operator.read_file(&self.path)
    }
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to resolve value concatenation")]
enum ConcatResolveError {
    FileRead(#[from] FileReadError),
    Lua(#[from] mlua::Error),
}

impl ConcatResolveError {
    fn enforce_ffi_boundary(self) -> Self {
        match self {
            Self::FileRead(error) => Self::FileRead(error.enforce_ffi_boundary()),
            other => other,
        }
    }
}

fn resolve_file_content_concat_value(
    lua_to_string: &mlua::Function,
    value: mlua::Value,
) -> Result<Vec<u8>, ConcatResolveError> {
    match value {
        mlua::Value::String(string) => Ok(string.as_bytes().to_vec()),
        mlua::Value::UserData(user_data) => {
            if let Ok(content) = user_data.borrow::<FileContent>() {
                return Ok(content.materialize()?);
            }

            let lua_string: mlua::String = lua_to_string.call(mlua::Value::UserData(user_data))?;

            Ok(lua_string.as_bytes().to_vec())
        }
        other => {
            let lua_string: mlua::String = lua_to_string.call(other)?;

            Ok(lua_string.as_bytes().to_vec())
        }
    }
}

impl UserData for FileContent {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(MetaMethod::ToString, |_, this, ()| {
            this.materialize().map(mlua::BString::new).map_err(|error| {
                mlua::Error::RuntimeError(
                    ErrorReport::boxed_from(error.enforce_ffi_boundary()).build_report(),
                )
            })
        });

        methods.add_meta_function(
            MetaMethod::Concat,
            |lua, (left, right): (mlua::Value, mlua::Value)| {
                let lua_to_string: mlua::Function = lua.globals().get("tostring")?;

                let map_error = |error: ConcatResolveError| match error.enforce_ffi_boundary() {
                    ConcatResolveError::FileRead(error) => {
                        mlua::Error::RuntimeError(ErrorReport::boxed_from(error).build_report())
                    }
                    ConcatResolveError::Lua(error) => error,
                };

                let mut result =
                    resolve_file_content_concat_value(&lua_to_string, left).map_err(map_error)?;
                result.extend_from_slice(
                    &resolve_file_content_concat_value(&lua_to_string, right).map_err(map_error)?,
                );

                Ok(mlua::BString::new(result))
            },
        );
    }
}

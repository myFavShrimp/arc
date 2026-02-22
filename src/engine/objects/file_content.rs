use std::path::PathBuf;

use mlua::{FromLua, MetaMethod, UserData};

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

pub enum FileContentOrString {
    FileContent(FileContent),
    String(String),
}

#[derive(Debug, thiserror::Error)]
pub enum IntoStringError {
    #[error(transparent)]
    FileRead(#[from] FileReadError),
    #[error("File content is not valid UTF-8")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),
}

impl FfiError for IntoStringError {
    fn is_user_error(&self) -> bool {
        match self {
            Self::FileRead(error) => error.is_user_error(),
            Self::InvalidUtf8(_) => true,
        }
    }
}

impl FileContentOrString {
    pub fn into_string(self) -> Result<String, IntoStringError> {
        match self {
            Self::String(string) => Ok(string),
            Self::FileContent(file_content) => {
                let bytes = file_content.materialize()?;

                Ok(String::from_utf8(bytes)?)
            }
        }
    }
}

impl FromLua for FileContentOrString {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::String(string) => Ok(Self::String(string.to_str()?.to_owned())),
            mlua::Value::UserData(user_data) => {
                let content = user_data.borrow::<FileContent>()?.clone();

                Ok(Self::FileContent(content))
            }
            other => Err(mlua::Error::FromLuaConversionError {
                from: other.type_name(),
                to: "string".to_string(),
                message: Some("expected string or file content".to_string()),
            }),
        }
    }
}

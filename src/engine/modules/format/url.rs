use mlua::{LuaSerdeExt, UserData};

use crate::{
    engine::{delegator::error::FfiError, objects::file_content::FileContentOrString},
    error::ErrorReport,
};

pub struct Url;

#[derive(Debug, thiserror::Error)]
#[error("Failed to encode value as URL query string")]
enum EncodeError {
    Json(#[from] serde_json::Error),
    Qs(#[from] serde_qs::Error),
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to decode URL query string")]
enum DecodeError {
    Qs(#[from] serde_qs::Error),
    Lua(#[from] mlua::Error),
}

impl Url {
    fn encode(value: mlua::Value) -> Result<String, EncodeError> {
        Ok(serde_qs::to_string(&serde_json::to_value(&value)?)?)
    }

    fn decode(lua: &mlua::Lua, input: String) -> Result<mlua::Value, DecodeError> {
        Ok(lua.to_value(&serde_qs::from_str::<serde_json::Value>(&input)?)?)
    }
}

impl UserData for Url {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_function("encode", |_, value: mlua::Value| {
            Self::encode(value).map_err(|error| {
                mlua::Error::RuntimeError(ErrorReport::boxed_from(error).build_report())
            })
        });

        methods.add_function("decode", |lua, input: FileContentOrString| {
            let input = input.into_string().map_err(|error| {
                mlua::Error::RuntimeError(
                    ErrorReport::boxed_from(error.enforce_ffi_boundary()).build_report(),
                )
            })?;

            Self::decode(lua, input).map_err(|error| {
                mlua::Error::RuntimeError(ErrorReport::boxed_from(error).build_report())
            })
        });
    }
}

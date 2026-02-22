use mlua::{LuaSerdeExt, UserData};

use crate::{
    engine::{delegator::error::FfiError, objects::file_content::FileContentOrString},
    error::ErrorReport,
};

pub struct Toml;

#[derive(Debug, thiserror::Error)]
#[error("Failed to encode value as TOML")]
enum EncodeError {
    Json(#[from] serde_json::Error),
    Toml(#[from] toml::ser::Error),
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to decode TOML")]
enum DecodeError {
    Toml(#[from] toml::de::Error),
    Lua(#[from] mlua::Error),
}

impl Toml {
    fn encode(value: mlua::Value) -> Result<String, EncodeError> {
        Ok(toml::to_string(&serde_json::to_value(&value)?)?)
    }

    fn decode(lua: &mlua::Lua, input: String) -> Result<mlua::Value, DecodeError> {
        Ok(lua.to_value(&toml::from_str::<toml::Value>(&input)?)?)
    }
}

impl UserData for Toml {
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

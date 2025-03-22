use mlua::{LuaSerdeExt, UserData};
use serde_json;

use crate::error::ErrorReport;

pub struct Format;

#[derive(Debug, thiserror::Error)]
#[error("Failed to convert value to json")]
pub struct ToJsonError(#[from] serde_json::Error);

#[derive(Debug, thiserror::Error)]
#[error("Failed to convert value to json")]
pub enum FromJsonError {
    Serde(#[from] serde_json::Error),
    Lua(#[from] mlua::Error),
}

impl Format {
    pub fn new() -> Self {
        Self {}
    }

    fn to_json(value: mlua::Value) -> Result<String, ToJsonError> {
        Ok(serde_json::to_string(&serde_json::to_value(&value)?)?)
    }

    fn to_json_pretty(value: mlua::Value) -> Result<String, ToJsonError> {
        Ok(serde_json::to_string_pretty(&serde_json::to_value(
            &value,
        )?)?)
    }

    pub fn from_json(lua: &mlua::Lua, json: String) -> Result<mlua::Value, FromJsonError> {
        Ok(lua.to_value(&serde_json::from_str::<serde_json::Value>(&json)?)?)
    }
}

impl UserData for Format {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_function("to_json", |_, value: mlua::Value| {
            Ok(Self::to_json(value)
                .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))?)
        });

        methods.add_function("to_json_pretty", |_, value: mlua::Value| {
            Ok(Self::to_json_pretty(value)
                .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))?)
        });

        methods.add_function("from_json", |lua, json_str: String| {
            Ok(Self::from_json(lua, json_str)
                .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))?)
        });
    }
}

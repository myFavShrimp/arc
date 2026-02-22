use std::collections::BTreeMap;

use mlua::{LuaSerdeExt, UserData};

use crate::{
    engine::{delegator::error::FfiError, objects::file_content::FileContentOrString},
    error::ErrorReport,
};

pub struct Env;

#[derive(Debug, thiserror::Error)]
#[error("Failed to decode environment variables")]
enum DecodeError {
    Dotenvy(#[from] dotenvy::Error),
    Lua(#[from] mlua::Error),
}

impl Env {
    fn decode(lua: &mlua::Lua, input: String) -> Result<mlua::Value, DecodeError> {
        let map: BTreeMap<String, String> =
            dotenvy::from_read_iter(input.as_bytes()).collect::<Result<_, _>>()?;

        Ok(lua.to_value(&map)?)
    }
}

impl UserData for Env {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
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

use std::net::AddrParseError;

use mlua::{FromLua, IntoLua, LuaSerdeExt, MetaMethod, UserData};
use serde::Serialize;

use crate::{
    error::{ErrorReport, MutexLockError},
    memory::{
        target_systems::{
            TargetSystem, TargetSystemAdditionError, TargetSystemRetrievalError,
            TargetSystemsMemory,
        },
        SharedMemory,
    },
};

#[derive(Debug, Clone, Serialize)]
pub struct SystemConfig {
    pub address: String,
    pub port: u16,
    pub user: String,
}

impl FromLua for SystemConfig {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::Table(table) => {
                let address_field = table
                    .get::<mlua::Value>("address")
                    .or(Err(mlua::Error::runtime("\"address\" is missing")))?;
                let address = lua
                    .from_value(address_field)
                    .or(Err(mlua::Error::runtime("\"address\" is invalid")))?;

                let user_field = table
                    .get("user")
                    .or(Err(mlua::Error::runtime("\"user\" is missing")))?;
                let user = lua
                    .from_value(user_field)
                    .or(Err(mlua::Error::runtime("\"user\" is invalid")))?;

                let port = table
                    .get::<Option<u16>>("port")
                    .or(Err(mlua::Error::runtime("\"port\" is invalid")))?
                    .unwrap_or(22);

                Ok(SystemConfig {
                    address,
                    port,
                    user,
                })
            }
            mlua::Value::Function(_)
            | mlua::Value::Nil
            | mlua::Value::Boolean(_)
            | mlua::Value::LightUserData(_)
            | mlua::Value::Integer(_)
            | mlua::Value::Number(_)
            | mlua::Value::Vector(_)
            | mlua::Value::String(_)
            | mlua::Value::Thread(_)
            | mlua::Value::UserData(_)
            | mlua::Value::Buffer(_)
            | mlua::Value::Error(_)
            | mlua::Value::Other(_) => Err(mlua::Error::runtime(format!(
                "{:?} is not a valid system config",
                value.type_name()
            ))),
        }
    }
}

impl IntoLua for TargetSystem {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        let config_table = lua.create_table()?;
        config_table.set("name", self.name)?;
        config_table.set("address", self.address.to_string())?;
        config_table.set("port", self.port)?;
        config_table.set("user", self.user)?;

        config_table.set_readonly(true);

        Ok(mlua::Value::Table(config_table))
    }
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to add system")]
pub enum SystemAdditionError {
    Lock(#[from] MutexLockError),
    SystemAddition(#[from] TargetSystemAdditionError),
    AddrParse(#[from] AddrParseError),
}

#[derive(Debug, thiserror::Error)]
#[error("Duplicate system: {0:?}")]
pub struct DuplicateSystemError(pub String);

#[derive(Debug, thiserror::Error)]
#[error("Failed to retrieve system configuration")]
pub enum SystemRetrievalError {
    Lock(#[from] MutexLockError),
    TargetSystemRetrieval(#[from] TargetSystemRetrievalError),
}

#[derive(Debug, thiserror::Error)]
#[error("System {0:?} is not defined")]
pub struct SystemNotDefinedError(String);

pub struct SystemsTable {
    pub systems_memory: SharedMemory<TargetSystemsMemory>,
}

impl SystemsTable {
    fn add(&self, name: String, config: SystemConfig) -> Result<(), SystemAdditionError> {
        let mut guard = self.systems_memory.lock().map_err(|_| MutexLockError)?;

        guard.add(TargetSystem {
            name,
            address: config.address.parse()?,
            port: config.port,
            user: config.user,
        })?;

        Ok(())
    }

    fn get(&self, name: String) -> Result<TargetSystem, SystemRetrievalError> {
        let guard = self.systems_memory.lock().map_err(|_| MutexLockError)?;

        Ok(guard.get(&name)?)
    }
}

impl UserData for SystemsTable {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(
            MetaMethod::NewIndex,
            |_, this, (name, config): (String, SystemConfig)| {
                this.add(name, config)
                    .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
            },
        );

        methods.add_meta_method(MetaMethod::Index, |_, this, (name,): (String,)| {
            this.get(name)
                .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
        });
    }
}

use std::net::AddrParseError;

use mlua::{FromLua, IntoLua, LuaSerdeExt, MetaMethod, UserData};
use serde::Serialize;

use crate::{
    engine::readonly::set_readonly,
    error::{ErrorReport, MutexLockError},
    memory::{
        SharedMemory,
        target_systems::{
            RemoteTargetSystem, TargetSystem, TargetSystemAdditionError, TargetSystemKind,
            TargetSystemRetrievalError, TargetSystemsMemory,
        },
    },
};

#[derive(Debug, Clone, Serialize)]
pub enum SystemConfig {
    Local,
    Remote {
        address: String,
        port: u16,
        user: String,
    },
}

#[derive(Default)]
enum SystemType {
    Local,
    #[default]
    Remote,
}

static INVALID_TYPE_MESSAGE: &str = "\"type\" is invalid - must be either \"remote\" or \"local\"";

impl SystemType {
    fn try_from_string(value: String) -> Result<SystemType, ()> {
        match value.as_str() {
            "local" => Ok(Self::Local),
            "remote" => Ok(Self::Remote),
            _ => Err(()),
        }
    }
}

impl FromLua for SystemConfig {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::Table(table) => {
                let r#type = {
                    match table.get::<mlua::Value>("type") {
                        Ok(mlua::Value::Nil) => SystemType::default(),
                        Ok(type_value) => {
                            let type_value = lua.from_value::<String>(type_value);

                            match type_value {
                                Ok(type_str) => SystemType::try_from_string(type_str)
                                    .or(Err(mlua::Error::runtime(INVALID_TYPE_MESSAGE)))?,
                                Err(_) => Err(mlua::Error::runtime(INVALID_TYPE_MESSAGE))?,
                            }
                        }
                        Err(_) => SystemType::Remote,
                    }
                };

                match r#type {
                    SystemType::Local => Ok(SystemConfig::Local),
                    SystemType::Remote => {
                        let address = {
                            let address_field = table
                                .get::<mlua::Value>("address")
                                .or(Err(mlua::Error::runtime("\"address\" is missing")))?;

                            lua.from_value(address_field)
                                .or(Err(mlua::Error::runtime("\"address\" is invalid")))?
                        };

                        let user = {
                            let user_field = table
                                .get("user")
                                .or(Err(mlua::Error::runtime("\"user\" is missing")))?;

                            lua.from_value(user_field)
                                .or(Err(mlua::Error::runtime("\"user\" is invalid")))?
                        };

                        let port = table
                            .get::<Option<u16>>("port")
                            .or(Err(mlua::Error::runtime("\"port\" is invalid")))?
                            .unwrap_or(22);

                        Ok(SystemConfig::Remote {
                            address,
                            port,
                            user,
                        })
                    }
                }
            }
            mlua::Value::Function(_)
            | mlua::Value::Nil
            | mlua::Value::Boolean(_)
            | mlua::Value::LightUserData(_)
            | mlua::Value::Integer(_)
            | mlua::Value::Number(_)
            | mlua::Value::String(_)
            | mlua::Value::Thread(_)
            | mlua::Value::UserData(_)
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
        let address = match &self.kind {
            TargetSystemKind::Remote(remote_target_system) => {
                Some(remote_target_system.address.to_string())
            }
            TargetSystemKind::Local => None,
        };
        let port = match &self.kind {
            TargetSystemKind::Remote(remote_target_system) => Some(remote_target_system.port),
            TargetSystemKind::Local => None,
        };
        let user = match &self.kind {
            TargetSystemKind::Remote(remote_target_system) => {
                Some(remote_target_system.user.clone())
            }
            TargetSystemKind::Local => None,
        };

        let config_table = lua.create_table()?;
        config_table.set("name", self.name)?;
        config_table.set("address", address)?;
        config_table.set("port", port)?;
        config_table.set("user", user)?;

        let config_table = set_readonly(lua, config_table)
            .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))?;

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
#[error("Failed to retrieve system configuration")]
pub enum SystemRetrievalError {
    Lock(#[from] MutexLockError),
    TargetSystemRetrieval(#[from] TargetSystemRetrievalError),
}

pub struct SystemsTable {
    pub systems_memory: SharedMemory<TargetSystemsMemory>,
}

impl SystemsTable {
    fn add(&self, name: String, config: SystemConfig) -> Result<(), SystemAdditionError> {
        let mut guard = self.systems_memory.lock().map_err(|_| MutexLockError)?;

        guard.add(TargetSystem {
            name,
            kind: match config {
                SystemConfig::Local => TargetSystemKind::Local,
                SystemConfig::Remote {
                    address,
                    port,
                    user,
                } => TargetSystemKind::Remote(RemoteTargetSystem {
                    address: address.parse()?,
                    port,
                    user,
                }),
            },
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

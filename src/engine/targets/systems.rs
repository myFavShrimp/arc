use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
};

use mlua::{IntoLua, MetaMethod, UserData};

use crate::error::{ErrorReport, MutexLockError};

#[derive(Debug, Clone)]
pub struct SystemConfig {
    pub name: String,
    pub address: IpAddr,
    pub port: u16,
    pub user: String,
}

impl TryFrom<(String, mlua::Value)> for SystemConfig {
    type Error = mlua::Error;

    fn try_from((name, value): (String, mlua::Value)) -> mlua::Result<Self> {
        match value {
            mlua::Value::Table(table) => {
                let address_string = table
                    .get::<String>("address")
                    .or(Err(mlua::Error::runtime("\"address\" is invalid")))?;

                let port = table
                    .get::<Option<u16>>("port")
                    .or(Err(mlua::Error::runtime("\"port\" is invalid")))?
                    .unwrap_or(22);

                let user = table
                    .get("user")
                    .or(Err(mlua::Error::runtime("\"user\" is invalid")))?;

                let address = address_string.parse()?;

                Ok(SystemConfig {
                    name,
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
            | mlua::Value::Other(_) => Err(mlua::Error::runtime("Invalid system config")),
        }
    }
}

impl IntoLua for SystemConfig {
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

impl SystemConfig {
    pub fn socket_address(&self) -> SocketAddr {
        SocketAddr::new(self.address, self.port)
    }
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to add system")]
pub enum SystemAdditionError {
    Lock(#[from] MutexLockError),
    DuplicateSystem(#[from] DuplicateSystemError),
}

#[derive(Debug, thiserror::Error)]
#[error("Duplicate system: {0:?}")]
pub struct DuplicateSystemError(pub String);

#[derive(Debug, thiserror::Error)]
#[error("Failed to retrieve system configuration")]
pub enum SystemAcquisitionError {
    Lock(#[from] MutexLockError),
    SystemNotDefinedError(#[from] SystemNotDefinedError),
}

#[derive(Debug, thiserror::Error)]
#[error("System {0:?} is not defined")]
pub struct SystemNotDefinedError(String);

#[derive(Debug, Clone, Default)]
pub struct Systems(Arc<Mutex<HashMap<String, SystemConfig>>>);

impl Systems {
    pub fn all(&self) -> Result<HashMap<String, SystemConfig>, SystemAcquisitionError> {
        let guard = self.0.lock().map_err(|_| MutexLockError)?;

        Ok(guard.clone())
    }

    fn add(&self, name: String, config: SystemConfig) -> Result<(), SystemAdditionError> {
        let mut guard = self.0.lock().map_err(|_| MutexLockError)?;

        if guard.insert(name.clone(), config).is_some() {
            Err(DuplicateSystemError(name))?;
        }

        Ok(())
    }

    fn get(&self, name: String) -> Result<SystemConfig, SystemAcquisitionError> {
        let guard = self.0.lock().map_err(|_| MutexLockError)?;

        Ok(guard
            .get(&name)
            .ok_or(SystemNotDefinedError(name.clone()))?
            .clone())
    }
}

impl UserData for Systems {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(
            MetaMethod::NewIndex,
            |_, this, (name, config): (String, mlua::Value)| {
                let config = SystemConfig::try_from((name.clone(), config))?;

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

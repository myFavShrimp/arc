use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use mlua::{FromLua, IntoLua, MetaMethod, UserData};
use serde::{Deserialize, Serialize};

use crate::error::{ErrorReport, MutexLockError};

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct GroupConfig {
    pub members: Vec<String>,
}

impl FromLua for GroupConfig {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::Table(table) => {
                let members = table
                    .get::<Vec<String>>("members")
                    .or(Err(mlua::Error::runtime("\"members\" is invalid")))?;

                Ok(GroupConfig { members })
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

impl IntoLua for GroupConfig {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        let config_table = lua.create_table()?;

        let members_table = lua.create_table()?;
        for member in self.members {
            members_table.push(member)?;
        }
        members_table.set_readonly(true);

        config_table.set("members", members_table)?;
        config_table.set_readonly(true);

        Ok(mlua::Value::Table(config_table))
    }
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to add group")]
pub enum GroupAdditionError {
    Lock(#[from] MutexLockError),
    DuplicateGroup(#[from] DuplicateGroupError),
}

#[derive(Debug, thiserror::Error)]
#[error("Duplicate group: {0:?}")]
pub struct DuplicateGroupError(pub String);

#[derive(Debug, thiserror::Error)]
#[error("Failed to retrieve group configuration")]
pub enum GroupAcquisitionError {
    Lock(#[from] MutexLockError),
    GroupNotDefined(#[from] GroupNotDefinedError),
}

#[derive(Debug, thiserror::Error)]
#[error("Group {0:?} is not defined")]
pub struct GroupNotDefinedError(String);

#[derive(Debug, Clone, Default)]
pub struct Groups(Arc<Mutex<HashMap<String, GroupConfig>>>);

impl Groups {
    pub fn all(&self) -> Result<HashMap<String, GroupConfig>, GroupAcquisitionError> {
        let guard = self.0.lock().map_err(|_| MutexLockError)?;

        Ok(guard.clone())
    }

    fn add(&self, name: String, config: GroupConfig) -> Result<(), GroupAdditionError> {
        let mut guard = self.0.lock().map_err(|_| MutexLockError)?;

        if let Some(_) = guard.insert(name.clone(), config) {
            Err(DuplicateGroupError(name))?;
        }

        Ok(())
    }

    fn get(&self, name: String) -> Result<GroupConfig, GroupAcquisitionError> {
        let guard = self.0.lock().map_err(|_| MutexLockError)?;

        Ok(guard
            .get(&name)
            .ok_or(GroupNotDefinedError(name.clone()))?
            .clone())
    }
}

impl UserData for Groups {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(
            MetaMethod::NewIndex,
            |_, this, (name, config): (String, GroupConfig)| {
                Ok(this
                    .add(name, config)
                    .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))?)
            },
        );

        methods.add_meta_method(MetaMethod::Index, |_, this, (name,): (String,)| {
            Ok(this
                .get(name)
                .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))?)
        });
    }
}

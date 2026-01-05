use mlua::{FromLua, IntoLua, LuaSerdeExt, MetaMethod, UserData};
use serde::{Deserialize, Serialize};

use crate::{
    engine::readonly::set_readonly,
    error::{ErrorReport, MutexLockError},
    memory::{
        SharedMemory,
        target_groups::{
            TargetGroup, TargetGroupAdditionError, TargetGroupRetrievalError, TargetGroupsMemory,
        },
    },
};

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct GroupConfig {
    pub members: Vec<String>,
}

impl FromLua for GroupConfig {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::Table(table) => {
                let members_field = table
                    .get::<mlua::Value>("members")
                    .or(Err(mlua::Error::runtime("\"members\" is missing")))?;

                let members = lua.from_value(members_field).or(Err(mlua::Error::runtime(
                    "\"members\" is invalid".to_string(),
                )))?;

                Ok(GroupConfig { members })
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
                "{:?} is not a valid group config",
                value.type_name()
            ))),
        }
    }
}

impl IntoLua for TargetGroup {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        let config_table = lua.create_table()?;

        let members_table = lua.create_table()?;
        for member in self.members {
            members_table.push(member)?;
        }
        let members_table = set_readonly(lua, members_table)
            .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))?;

        config_table.set("members", members_table)?;
        let config_table = set_readonly(lua, config_table)
            .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))?;

        Ok(mlua::Value::Table(config_table))
    }
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to add group")]
pub enum GroupAdditionError {
    Lock(#[from] MutexLockError),
    TargetGroupAddition(#[from] TargetGroupAdditionError),
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to retrieve group configuration")]
pub enum GroupRetrievalError {
    Lock(#[from] MutexLockError),
    TargetGroupRetrieval(#[from] TargetGroupRetrievalError),
}

pub struct GroupsTable {
    pub groups_memory: SharedMemory<TargetGroupsMemory>,
}

impl GroupsTable {
    fn add(&self, name: String, config: GroupConfig) -> Result<(), GroupAdditionError> {
        let mut groups_memory = self.groups_memory.lock().map_err(|_| MutexLockError)?;

        groups_memory.add(TargetGroup {
            name,
            members: config.members,
        })?;

        Ok(())
    }

    fn get(&self, name: String) -> Result<TargetGroup, GroupRetrievalError> {
        let groups_memory = self.groups_memory.lock().map_err(|_| MutexLockError)?;

        Ok(groups_memory.get(&name)?)
    }
}

impl UserData for GroupsTable {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(
            MetaMethod::NewIndex,
            |_, this, (name, config): (String, GroupConfig)| {
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

use groups::{GroupRetrievalError, GroupsTable};
use mlua::IntoLua;
use systems::{SystemRetrievalError, SystemsTable};

use crate::memory::{
    target_groups::TargetGroupsMemory, target_systems::TargetSystemsMemory, SharedMemory,
};

pub mod groups;
pub mod systems;

pub struct TargetsTable {
    pub systems: SystemsTable,
    pub groups: GroupsTable,
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum TargetsAcquisitionError {
    SystemAcquisition(#[from] SystemRetrievalError),
    GroupAcquisition(#[from] GroupRetrievalError),
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum TargetsValidationError {
    SystemAcquisition(#[from] SystemRetrievalError),
    GroupAcquisition(#[from] GroupRetrievalError),
    GroupMembersNotDefined(#[from] GroupMembersNotDefinedError),
}

#[derive(Debug, thiserror::Error)]
#[error("Group member {1:?} of group {0:?} is not defined")]
pub struct GroupMembersNotDefinedError(String, pub Vec<String>);

// pub type TargetsTuple = (HashMap<String, SystemConfig>, HashMap<String, GroupConfig>);

impl TargetsTable {
    pub fn new(
        groups_memory: SharedMemory<TargetGroupsMemory>,
        systems_memory: SharedMemory<TargetSystemsMemory>,
    ) -> Self {
        Self {
            systems: SystemsTable {
                systems_memory: systems_memory.clone(),
            },
            groups: GroupsTable {
                groups_memory,
                systems_memory,
            },
        }
    }
}

impl IntoLua for TargetsTable {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        let targets_table = lua.create_table()?;

        targets_table.set("systems", self.systems)?;
        targets_table.set("groups", self.groups)?;

        targets_table.set_readonly(true);

        Ok(mlua::Value::Table(targets_table))
    }
}

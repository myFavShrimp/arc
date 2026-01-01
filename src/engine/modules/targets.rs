use groups::GroupsTable;
use mlua::IntoLua;
use systems::SystemsTable;

use crate::{
    engine::readonly::set_readonly,
    error::ErrorReport,
    memory::{
        SharedMemory, target_groups::TargetGroupsMemory, target_systems::TargetSystemsMemory,
    },
};

pub mod groups;
pub mod systems;

pub struct TargetsTable {
    pub systems: SystemsTable,
    pub groups: GroupsTable,
}

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

        let targets_table = set_readonly(lua, targets_table)
            .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))?;

        Ok(mlua::Value::Table(targets_table))
    }
}

use std::collections::HashMap;

use groups::{GroupAcquisitionError, GroupConfig, Groups};
use mlua::IntoLua;
use systems::{SystemAcquisitionError, SystemConfig, Systems};

pub mod groups;
pub mod systems;

#[derive(Debug, Default, Clone)]
pub struct Targets {
    pub systems: Systems,
    pub groups: Groups,
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum TargetsAcquisitionError {
    SystemAcquisition(#[from] SystemAcquisitionError),
    GroupAcquisition(#[from] GroupAcquisitionError),
}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub enum TargetsValidationError {
    SystemAcquisition(#[from] SystemAcquisitionError),
    GroupAcquisition(#[from] GroupAcquisitionError),
    GroupMembersNotDefined(#[from] GroupMembersNotDefinedError),
}

#[derive(Debug, thiserror::Error)]
#[error("Group member {1:?} of group {0:?} is not defined")]
pub struct GroupMembersNotDefinedError(String, pub Vec<String>);

pub type TargetsTuple = (HashMap<String, SystemConfig>, HashMap<String, GroupConfig>);

impl Targets {
    pub fn targets(&self) -> Result<TargetsTuple, TargetsAcquisitionError> {
        Ok((self.systems.all()?, self.groups.all()?))
    }

    pub fn validate(&self) -> Result<(), TargetsValidationError> {
        let systems = self.systems.all()?;
        let groups = self.groups.all()?;

        for (name, group) in groups {
            match &group
                .members
                .iter()
                .filter_map(|member| {
                    let member = member.clone();

                    if !systems.contains_key(&member) {
                        Some(member)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()[..]
            {
                [] => {}
                missing_systems => {
                    Err(GroupMembersNotDefinedError(name, missing_systems.to_vec()))?
                }
            }
        }

        Ok(())
    }
}

impl IntoLua for Targets {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        let targets_table = lua.create_table()?;

        targets_table.set("systems", self.systems)?;
        targets_table.set("groups", self.groups)?;

        Ok(mlua::Value::Table(targets_table))
    }
}

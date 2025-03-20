use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
};

use mlua::FromLua;
use serde::{Deserialize, Serialize};

use crate::error::MutexLockError;

#[derive(Debug, Default, Clone, Serialize)]
pub struct Targets {
    pub systems: HashMap<String, SystemConfig>,
    pub groups: HashMap<String, GroupConfig>,
}

fn default_port() -> u16 {
    22
}

#[derive(Debug, Deserialize, Clone, Serialize, FromLua)]
pub struct SystemConfig {
    pub address: IpAddr,
    #[serde(default = "default_port")]
    pub port: u16,
    pub user: String,
}

impl SystemConfig {
    pub fn socket_address(&self) -> SocketAddr {
        SocketAddr::new(self.address, self.port)
    }
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct GroupConfig {
    pub members: Vec<String>,
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to add system")]
pub enum SystemAdditionError {
    Lock(#[from] MutexLockError),
    DuplicateSystem(#[from] DuplicateSystemError),
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to add group")]
pub enum GroupAdditionError {
    Lock(#[from] MutexLockError),
    MissingGroupMembers(#[from] UnregisteredGroupMembersError),
    DuplicateGroup(#[from] DuplicateGroupError),
}

#[derive(Debug, thiserror::Error)]
#[error("Unregistered group members: {0:?}")]
pub struct UnregisteredGroupMembersError(pub Vec<String>);

#[derive(Debug, thiserror::Error)]
#[error("Failed to retrieve targets configuration")]
pub enum TargetsAcquisitionError {
    Lock(#[from] MutexLockError),
}

#[derive(Debug, thiserror::Error)]
#[error("Duplicate system: {0:?}")]
pub struct DuplicateSystemError(pub String);

#[derive(Debug, thiserror::Error)]
#[error("Duplicate group: {0:?}")]
pub struct DuplicateGroupError(pub String);

#[derive(Debug, Default, Clone)]
pub struct TargetsModule {
    targets: Arc<Mutex<Targets>>,
}

impl TargetsModule {
    pub fn add_system(
        &self,
        name: String,
        config: SystemConfig,
    ) -> Result<(), SystemAdditionError> {
        let mut guard = self.targets.lock().map_err(|_| MutexLockError)?;

        if let Some(_) = guard.systems.insert(name.clone(), config) {
            Err(DuplicateSystemError(name))?;
        }

        Ok(())
    }

    pub fn add_group(&self, name: String, config: GroupConfig) -> Result<(), GroupAdditionError> {
        let mut guard = self.targets.lock().map_err(|_| MutexLockError)?;

        match &config
            .members
            .iter()
            .filter_map(|member| {
                let member = member.clone();

                if !guard.systems.contains_key(&member) {
                    Some(member)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()[..]
        {
            [] => {}
            missing_systems => Err(UnregisteredGroupMembersError(missing_systems.to_vec()))?,
        }

        if let Some(_) = guard.groups.insert(name.clone(), config) {
            Err(DuplicateGroupError(name))?;
        }

        Ok(())
    }

    pub fn targets(
        &self,
    ) -> Result<Targets, crate::engine::modules::targets::TargetsAcquisitionError> {
        let guard = self.targets.lock().map_err(|_| MutexLockError)?;

        Ok((*guard).clone())
    }
}

use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};

use crate::{
    engine::modules::targets::{
        DuplicateGroupError, DuplicateSystemError, GroupAdditionError, SystemAdditionError,
        TargetsModule, UnregisteredGroupMembersError,
    },
    error::MutexLockError,
};

#[derive(Debug, Default, Clone, Serialize)]
pub struct Targets {
    pub systems: HashMap<String, SystemConfig>,
    pub groups: HashMap<String, GroupConfig>,
}

fn default_port() -> u16 {
    22
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct SystemConfig {
    address: IpAddr,
    #[serde(default = "default_port")]
    port: u16,
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

#[derive(Debug, Default)]
pub struct TargetsRegistrationModule {
    targets: Arc<Mutex<Targets>>,
}

impl TargetsModule for TargetsRegistrationModule {
    fn add_system(&self, name: String, config: SystemConfig) -> Result<(), SystemAdditionError> {
        let mut guard = self.targets.lock().map_err(|_| MutexLockError)?;

        if let Some(_) = guard.systems.insert(name.clone(), config) {
            Err(DuplicateSystemError(name))?;
        }

        Ok(())
    }

    fn add_group(&self, name: String, config: GroupConfig) -> Result<(), GroupAdditionError> {
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

    fn targets(&self) -> Result<Targets, crate::engine::modules::targets::TargetsAcquisitionError> {
        let guard = self.targets.lock().map_err(|_| MutexLockError)?;

        Ok((*guard).clone())
    }
}

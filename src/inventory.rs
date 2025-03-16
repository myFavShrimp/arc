use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};

use crate::{
    engine::modules::{
        GroupAdditionError, InventoryModule, SystemAdditionError, UnregisteredGroupMembersError,
    },
    error::MutexLockError,
};

#[derive(Debug, Default, Clone, Serialize)]
pub struct Inventory {
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
pub struct InventoryRegistrationModule {
    inventory: Arc<Mutex<Inventory>>,
}

impl InventoryModule for InventoryRegistrationModule {
    fn add_system(&self, name: String, config: SystemConfig) -> Result<(), SystemAdditionError> {
        let mut guard = self.inventory.lock().map_err(|_| MutexLockError)?;

        guard.systems.insert(name.to_owned(), config);

        Ok(())
    }

    fn add_group(&self, name: String, config: GroupConfig) -> Result<(), GroupAdditionError> {
        let mut guard = self.inventory.lock().map_err(|_| MutexLockError)?;

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

        guard.groups.insert(name.to_owned(), config);

        Ok(())
    }

    fn inventory(&self) -> Result<Inventory, crate::engine::modules::InventoryAcquisitionError> {
        let guard = self.inventory.lock().map_err(|_| MutexLockError)?;

        Ok((*guard).clone())
    }
}

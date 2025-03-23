use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
};

#[derive(Debug, Clone)]
pub struct TargetSystem {
    pub name: String,
    pub address: IpAddr,
    pub port: u16,
    pub user: String,
}

impl TargetSystem {
    pub fn socket_address(&self) -> SocketAddr {
        SocketAddr::new(self.address, self.port)
    }
}

pub type TargetSystems = HashMap<String, TargetSystem>;

#[derive(Debug, Default)]
pub struct TargetSystemsMemory {
    memory: TargetSystems,
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to add system")]
pub enum TargetSystemAdditionError {
    DuplicateSystem(#[from] DuplicateTargetSystemError),
}

#[derive(Debug, thiserror::Error)]
#[error("Duplicate system: {0:?}")]
pub struct DuplicateTargetSystemError(pub String);

#[derive(Debug, thiserror::Error)]
#[error("Failed to retrieve system configuration")]
pub enum TargetSystemRetrievalError {
    SystemNotDefinedError(#[from] TargetSystemNotDefinedError),
}

#[derive(Debug, thiserror::Error)]
#[error("System {0:?} is not defined")]
pub struct TargetSystemNotDefinedError(String);

impl TargetSystemsMemory {
    pub fn all(&self) -> TargetSystems {
        self.memory.clone()
    }

    pub fn add(&mut self, config: TargetSystem) -> Result<(), TargetSystemAdditionError> {
        if self
            .memory
            .insert(config.name.clone(), config.clone())
            .is_some()
        {
            Err(DuplicateTargetSystemError(config.name.clone()))?;
        }

        Ok(())
    }

    pub fn get(&self, name: &str) -> Result<TargetSystem, TargetSystemRetrievalError> {
        Ok(self
            .memory
            .get(name)
            .ok_or(TargetSystemNotDefinedError(name.to_string()))?
            .clone())
    }

    pub fn has(&self, name: &str) -> bool {
        self.memory.contains_key(name)
    }
}

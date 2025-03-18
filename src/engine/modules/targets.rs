use crate::{
    error::MutexLockError,
    targets::{GroupConfig, SystemConfig, Targets},
};

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

pub trait TargetsModule {
    fn add_system(&self, name: String, config: SystemConfig) -> Result<(), SystemAdditionError>;
    fn add_group(&self, name: String, config: GroupConfig) -> Result<(), GroupAdditionError>;
    fn targets(&self) -> Result<Targets, TargetsAcquisitionError>;
}

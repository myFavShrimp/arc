use std::sync::{Arc, Mutex};

use crate::{
    engine::modules::targets::{
        DuplicateGroupError, DuplicateSystemError, GroupAdditionError, GroupConfig,
        SystemAdditionError, SystemConfig, Targets, TargetsModule, UnregisteredGroupMembersError,
    },
    error::MutexLockError,
};

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

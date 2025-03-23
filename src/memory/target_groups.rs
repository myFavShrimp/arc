use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct TargetGroup {
    pub name: String,
    pub members: Vec<String>,
}

pub type TargetGroups = HashMap<String, TargetGroup>;

#[derive(Debug, Default)]
pub struct TargetGroupsMemory {
    memory: TargetGroups,
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to add system")]
pub enum TargetGroupAdditionError {
    DuplicateGroup(#[from] DuplicateTargetGroupError),
}

#[derive(Debug, thiserror::Error)]
#[error("Duplicate system: {0:?}")]
pub struct DuplicateTargetGroupError(pub String);

#[derive(Debug, thiserror::Error)]
#[error("Failed to retrieve system configuration")]
pub enum TargetGroupRetrievalError {
    GroupNotDefinedError(#[from] TargetGroupNotDefinedError),
}

#[derive(Debug, thiserror::Error)]
#[error("Group {0:?} is not defined")]
pub struct TargetGroupNotDefinedError(String);

impl TargetGroupsMemory {
    pub fn all(&self) -> TargetGroups {
        self.memory.clone()
    }

    pub fn add(&mut self, config: TargetGroup) -> Result<(), TargetGroupAdditionError> {
        if self
            .memory
            .insert(config.name.clone(), config.clone())
            .is_some()
        {
            Err(DuplicateTargetGroupError(config.name.clone()))?;
        }

        Ok(())
    }

    pub fn get(&self, name: &str) -> Result<TargetGroup, TargetGroupRetrievalError> {
        Ok(self
            .memory
            .get(name)
            .ok_or(TargetGroupNotDefinedError(name.to_string()))?
            .clone())
    }
}

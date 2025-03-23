use crate::{
    error::MutexLockError,
    memory::{
        target_groups::{TargetGroups, TargetGroupsMemory},
        target_systems::{TargetSystems, TargetSystemsMemory},
        tasks::{Tasks, TasksMemory, TasksResultSetError},
        SharedMemory,
    },
};

pub struct State {
    target_systems: SharedMemory<TargetSystemsMemory>,
    target_groups: SharedMemory<TargetGroupsMemory>,
    tasks: SharedMemory<TasksMemory>,
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to reset tasks results")]
pub enum TasksResultResetError {
    Lock(#[from] MutexLockError),
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to set task's result")]
pub enum TasksResultStateSetError {
    Lock(#[from] MutexLockError),
    TaskResultSet(#[from] TasksResultSetError),
}

impl State {
    pub fn new(
        target_systems: SharedMemory<TargetSystemsMemory>,
        target_groups: SharedMemory<TargetGroupsMemory>,
        tasks: SharedMemory<TasksMemory>,
    ) -> Self {
        Self {
            target_systems,
            target_groups,
            tasks,
        }
    }

    pub fn systems_for_selected_groups(
        &self,
        selected_groups: &[String],
    ) -> Result<TargetSystems, MutexLockError> {
        let groups = self.target_groups.lock().map_err(|_| MutexLockError)?.all();
        let mut systems = self
            .target_systems
            .lock()
            .map_err(|_| MutexLockError)?
            .all();

        let mut filtered_group_configs = groups.clone();
        filtered_group_configs
            .retain(|name, _| selected_groups.is_empty() || selected_groups.contains(name));

        systems.retain(|name, _| {
            let is_in_group_selection = filtered_group_configs
                .iter()
                .any(|(_, group)| group.members.contains(name));
            let has_no_group = !groups.iter().any(|(_, group)| group.members.contains(name));

            is_in_group_selection || has_no_group
        });

        Ok(systems)
    }

    pub fn tasks_for_selected_groups_and_tags(
        &self,
        selected_groups: &[String],
        tags: &[String],
    ) -> Result<Tasks, MutexLockError> {
        let mut tasks = self.tasks.lock().map_err(|_| MutexLockError)?.all();

        tasks.retain(|_, task| {
            (selected_groups.is_empty()
                || task
                    .groups
                    .iter()
                    .any(|group| selected_groups.contains(group)))
                && (tags.is_empty() || task.tags.iter().any(|config_tag| tags.contains(config_tag)))
        });

        Ok(tasks)
    }

    pub fn missing_selected_groups(
        &self,
        selected_groups: &[String],
    ) -> Result<Vec<String>, MutexLockError> {
        let groups = self.target_groups.lock().map_err(|_| MutexLockError)?.all();

        let mut selected_groups = selected_groups.to_vec();
        selected_groups.retain(|name| !groups.contains_key(name));

        Ok(selected_groups)
    }

    pub fn selected_groups(
        &self,
        selected_groups: &[String],
    ) -> Result<TargetGroups, MutexLockError> {
        let mut groups = self.target_groups.lock().map_err(|_| MutexLockError)?.all();
        groups.retain(|name, _| !selected_groups.contains(name));

        Ok(groups)
    }

    pub fn reset_task_results(&self) -> Result<(), TasksResultResetError> {
        let mut guard = self.tasks.lock().map_err(|_| MutexLockError)?;

        guard.reset_results();

        Ok(())
    }

    pub fn set_task_result(
        &self,
        name: &str,
        value: mlua::Value,
    ) -> Result<(), TasksResultStateSetError> {
        let mut guard = self.tasks.lock().map_err(|_| MutexLockError)?;

        guard.set_task_result(name, value)?;

        Ok(())
    }
}

use std::collections::HashSet;

use crate::{
    error::MutexLockError,
    memory::{
        SharedMemory,
        target_groups::{TargetGroups, TargetGroupsMemory},
        target_systems::{TargetSystems, TargetSystemsMemory},
        tasks::{
            Task, TaskState, Tasks, TasksErrorSetError, TasksMemory, TasksResultSetError,
            TasksStateSetError,
        },
    },
};

pub struct UndefinedDependency {
    pub task_name: String,
    pub tag: String,
}

pub struct State {
    target_systems: SharedMemory<TargetSystemsMemory>,
    target_groups: SharedMemory<TargetGroupsMemory>,
    tasks: SharedMemory<TasksMemory>,
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to reset task execution state")]
pub enum TasksExecutionStateResetError {
    Lock(#[from] MutexLockError),
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to set task's result")]
pub enum TasksResultStateSetError {
    Lock(#[from] MutexLockError),
    TaskResultSet(#[from] TasksResultSetError),
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to set task's state")]
pub enum TasksStateStateSetError {
    Lock(#[from] MutexLockError),
    TaskStateSet(#[from] TasksStateSetError),
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to set task's error")]
pub enum TasksErrorStateSetError {
    Lock(#[from] MutexLockError),
    TaskErrorSet(#[from] TasksErrorSetError),
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
        selected_groups: &HashSet<String>,
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
        selected_groups: &HashSet<String>,
        selected_tags: &HashSet<String>,
    ) -> Result<Tasks, MutexLockError> {
        let mut tasks = self.tasks.lock().map_err(|_| MutexLockError)?.all();

        tasks.retain(|_, task| {
            (selected_groups.is_empty() || !task.groups.is_disjoint(selected_groups))
                && (selected_tags.is_empty() || !task.tags.is_disjoint(selected_tags))
        });

        Ok(tasks)
    }

    pub fn tasks_with_resolved_dependencies(
        &self,
        selected_groups: &HashSet<String>,
        selected_tags: &HashSet<String>,
    ) -> Result<(Tasks, Vec<UndefinedDependency>), MutexLockError> {
        let all_tasks = self.tasks.lock().map_err(|_| MutexLockError)?.all();
        let all_tags: HashSet<&String> = all_tasks
            .values()
            .flat_map(|task| task.tags.iter())
            .collect();

        let task_matches_groups = |task: &Task| -> bool {
            selected_groups.is_empty() || !task.groups.is_disjoint(selected_groups)
        };

        let task_matches_tags = |task: &Task| -> bool {
            selected_tags.is_empty() || !task.tags.is_disjoint(selected_tags)
        };

        let tasks_with_tag = |tag: &String| -> Vec<&String> {
            all_tasks
                .iter()
                .filter(|(_, task)| task.tags.contains(tag) && task_matches_groups(task))
                .map(|(name, _)| name)
                .collect()
        };

        let mut selected_task_names: HashSet<String> = all_tasks
            .iter()
            .filter(|(_, task)| task_matches_groups(task) && task_matches_tags(task))
            .map(|(name, _)| name.clone())
            .collect();

        let mut undefined_dependencies = Vec::new();
        let mut tasks_to_expand: Vec<String> = selected_task_names.iter().cloned().collect();

        while let Some(task_name) = tasks_to_expand.pop() {
            let Some(task) = all_tasks.get(&task_name) else {
                continue;
            };

            for dependency_tag in &task.dependencies {
                if !all_tags.contains(dependency_tag) {
                    undefined_dependencies.push(UndefinedDependency {
                        task_name: task_name.clone(),
                        tag: dependency_tag.clone(),
                    });
                    continue;
                }

                for name in tasks_with_tag(dependency_tag) {
                    if selected_task_names.insert(name.clone()) {
                        tasks_to_expand.push(name.clone());
                    }
                }
            }
        }

        let mut selected_tasks = all_tasks;
        selected_tasks.retain(|name, _| selected_task_names.contains(name));

        Ok((selected_tasks, undefined_dependencies))
    }

    pub fn missing_selected_groups(
        &self,
        selected_groups: &HashSet<String>,
    ) -> Result<Vec<String>, MutexLockError> {
        let groups = self.target_groups.lock().map_err(|_| MutexLockError)?.all();

        Ok(selected_groups
            .iter()
            .filter(|name| !groups.contains_key(*name))
            .cloned()
            .collect())
    }

    pub fn selected_groups(
        &self,
        selected_groups: &HashSet<String>,
    ) -> Result<TargetGroups, MutexLockError> {
        let mut groups = self.target_groups.lock().map_err(|_| MutexLockError)?.all();
        groups.retain(|name, _| !selected_groups.contains(name));

        Ok(groups)
    }

    pub fn reset_execution_state(&self) -> Result<(), TasksExecutionStateResetError> {
        let mut guard = self.tasks.lock().map_err(|_| MutexLockError)?;

        guard.reset_execution_state();

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

    pub fn set_task_state(
        &self,
        name: &str,
        state: TaskState,
    ) -> Result<(), TasksStateStateSetError> {
        let mut guard = self.tasks.lock().map_err(|_| MutexLockError)?;

        guard.set_task_state(name, state)?;

        Ok(())
    }

    pub fn set_task_error(&self, name: &str, error: String) -> Result<(), TasksErrorStateSetError> {
        let mut guard = self.tasks.lock().map_err(|_| MutexLockError)?;

        guard.set_task_error(name, error)?;

        Ok(())
    }
}

use crate::{
    error::MutexLockError,
    memory::{
        SharedMemory,
        target_groups::{TargetGroups, TargetGroupsMemory},
        target_systems::{TargetSystems, TargetSystemsMemory},
        tasks::{
            TaskState, Tasks, TasksErrorSetError, TasksMemory, TasksResultSetError,
            TasksStateSetError,
        },
    },
};

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

    pub fn all_systems(&self) -> Result<TargetSystems, MutexLockError> {
        Ok(self
            .target_systems
            .lock()
            .map_err(|_| MutexLockError)?
            .all())
    }

    pub fn all_groups(&self) -> Result<TargetGroups, MutexLockError> {
        Ok(self.target_groups.lock().map_err(|_| MutexLockError)?.all())
    }

    pub fn all_tasks(&self) -> Result<Tasks, MutexLockError> {
        Ok(self.tasks.lock().map_err(|_| MutexLockError)?.all())
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

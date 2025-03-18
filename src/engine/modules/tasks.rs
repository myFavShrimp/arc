use crate::{
    error::MutexLockError,
    tasks::{TaskConfig, Tasks},
};

#[derive(thiserror::Error, Debug)]
#[error("Failed to add task `{task}`")]
pub struct TaskAdditionError {
    pub task: String,
    #[source]
    pub kind: TaskAdditionErrorKind,
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum TaskAdditionErrorKind {
    Lock(#[from] MutexLockError),
    UnregisteredDependencies(#[from] UnregisteredDependenciesError),
    DuplicateTask(#[from] DuplicateTaskError),
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to retrieve tasks configuration")]
pub enum TasksAcquisitionError {
    Lock(#[from] MutexLockError),
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to retrieve task's result")]
pub enum TasksResultRetrievalError {
    Lock(#[from] MutexLockError),
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to reset tasks results")]
pub enum TasksResultResetError {
    Lock(#[from] MutexLockError),
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to set task's result")]
pub enum TasksResultSetError {
    Lock(#[from] MutexLockError),
}

#[derive(Debug, thiserror::Error)]
#[error("Unregistered task dependencies: {0:?}")]
pub struct UnregisteredDependenciesError(pub Vec<String>);

#[derive(Debug, thiserror::Error)]
#[error("Duplicate task: {0:?}")]
pub struct DuplicateTaskError(pub String);

pub trait TasksModule {
    fn tasks(&self) -> Result<Tasks, TasksAcquisitionError>;
    fn add_task(&self, name: String, config: TaskConfig) -> Result<(), TaskAdditionError>;
    fn reset_results(&self) -> Result<(), TasksResultResetError>;
    fn task_result(&self, name: &str) -> Result<Option<mlua::Value>, TasksResultRetrievalError>;
    fn set_task_result(&self, name: String, value: mlua::Value) -> Result<(), TasksResultSetError>;
}

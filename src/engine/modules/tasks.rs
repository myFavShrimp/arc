use std::collections::HashMap;

use crate::error::MutexLockError;

#[derive(Debug, Default, Clone)]
pub struct Tasks {
    pub tasks: HashMap<String, TaskConfig>,
}

impl Tasks {
    pub fn tasks_in_execution_order(&self) -> Vec<TaskConfig> {
        let mut tasks: Vec<TaskConfig> = self.tasks.clone().into_values().collect();
        tasks.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());

        tasks
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskConfig {
    pub name: String,
    pub handler: mlua::Function,
    pub dependencies: Vec<String>,
    pub tags: Vec<String>,
}

impl PartialOrd for TaskConfig {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let self_has_dependencies = !self.dependencies.is_empty();
        let other_has_dependencies = !other.dependencies.is_empty();

        Some(match (self_has_dependencies, other_has_dependencies) {
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            (false, false) => std::cmp::Ordering::Equal,
            (true, true) => {
                let other_depends_on_self = other.dependencies.contains(&self.name);
                let self_depends_on_other = self.dependencies.contains(&other.name);

                match (other_depends_on_self, self_depends_on_other) {
                    (true, true) | (false, false) => std::cmp::Ordering::Equal,
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                }
            }
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TaskConfigFromLuaValueError {
    #[error("Argument 2 of \"tasks.add\" must be a task handler or task configuration")]
    NotAFunctionOrTaskConfig,
    #[error("`dependencies` are invalid")]
    InvalidDependencies(#[source] mlua::Error),
    #[error("`handler` is invalid")]
    InvalidHandler(#[source] mlua::Error),
    #[error("`tags` is invalid")]
    InvalidTags(#[source] mlua::Error),
}

impl TryFrom<(String, mlua::Value)> for TaskConfig {
    type Error = TaskConfigFromLuaValueError;

    fn try_from((name, value): (String, mlua::Value)) -> Result<Self, Self::Error> {
        match value {
            mlua::Value::Table(table) => {
                let handler = table
                    .get::<mlua::Function>("handler")
                    .map_err(TaskConfigFromLuaValueError::InvalidHandler)?;
                let dependencies = table
                    .get::<Vec<String>>("dependencies")
                    .map_err(TaskConfigFromLuaValueError::InvalidDependencies)?;

                let tags = table
                    .get::<Option<Vec<String>>>("tags")
                    .map_err(TaskConfigFromLuaValueError::InvalidTags)?
                    .unwrap_or_default();

                Ok(TaskConfig {
                    name,
                    handler,
                    dependencies,
                    tags,
                })
            }
            mlua::Value::Function(handler) => Ok(TaskConfig {
                name,
                handler,
                dependencies: Default::default(),
                tags: Default::default(),
            }),
            mlua::Value::Nil
            | mlua::Value::Boolean(_)
            | mlua::Value::LightUserData(_)
            | mlua::Value::Integer(_)
            | mlua::Value::Number(_)
            | mlua::Value::Vector(_)
            | mlua::Value::String(_)
            | mlua::Value::Thread(_)
            | mlua::Value::UserData(_)
            | mlua::Value::Buffer(_)
            | mlua::Value::Error(_)
            | mlua::Value::Other(_) => Err(TaskConfigFromLuaValueError::NotAFunctionOrTaskConfig),
        }
    }
}

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

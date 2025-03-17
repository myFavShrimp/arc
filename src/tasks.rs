use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::{
    engine::modules::{
        DuplicateTaskError, TaskAdditionError, TasksModule, TasksResultResetError,
        UnregisteredDependenciesError,
    },
    error::MutexLockError,
};

#[derive(Debug, Default, Clone)]
pub struct Tasks {
    pub tasks: HashMap<String, TaskConfig>,
}

impl Tasks {
    pub fn tasks_in_execution_order(&self) -> Vec<TaskConfig> {
        let mut tasks: Vec<TaskConfig> = self.tasks.clone().into_values().collect();
        tasks.sort_by(|a, b| a.partial_cmp(b).unwrap());

        tasks
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskConfig {
    pub name: String,
    pub handler: mlua::Function,
    pub dependencies: Vec<String>,
}

impl PartialOrd for TaskConfig {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let other_depends_on_self = other.dependencies.contains(&self.name);
        let self_depends_on_other = self.dependencies.contains(&other.name);

        Some(match (other_depends_on_self, self_depends_on_other) {
            (true, true) | (false, false) => std::cmp::Ordering::Equal,
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
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

                Ok(TaskConfig {
                    name,
                    handler,
                    dependencies,
                })
            }
            mlua::Value::Function(handler) => Ok(TaskConfig {
                name,
                handler,
                dependencies: Default::default(),
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

#[derive(Debug, Default)]
pub struct TaskRegistrationModule {
    tasks: Arc<Mutex<Tasks>>,
    execution_results: Arc<Mutex<HashMap<String, mlua::Value>>>,
}

impl TasksModule for TaskRegistrationModule {
    fn add_task(&self, name: String, config: TaskConfig) -> Result<(), TaskAdditionError> {
        let mut guard = self.tasks.lock().map_err(|_| TaskAdditionError {
            task: name.clone(),
            kind: MutexLockError.into(),
        })?;

        let mut unregistered_dependencies = Vec::with_capacity(config.dependencies.len());
        for dep in &config.dependencies {
            if !guard.tasks.contains_key(dep) {
                unregistered_dependencies.push(dep.clone());
            }
        }
        if !unregistered_dependencies.is_empty() {
            Err(TaskAdditionError {
                task: name.clone(),
                kind: UnregisteredDependenciesError(unregistered_dependencies).into(),
            })?;
        }

        if let Some(_) = guard.tasks.insert(name.clone(), config) {
            Err(TaskAdditionError {
                task: name.clone(),
                kind: DuplicateTaskError(name).into(),
            })?;
        }

        Ok(())
    }

    fn tasks(&self) -> Result<Tasks, crate::engine::modules::TasksAcquisitionError> {
        let guard = self.tasks.lock().map_err(|_| MutexLockError)?;

        Ok((*guard).clone())
    }

    fn reset_results(&self) -> Result<(), TasksResultResetError> {
        let mut execution_results_guard =
            self.execution_results.lock().map_err(|_| MutexLockError)?;
        *execution_results_guard = Default::default();

        Ok(())
    }

    fn task_result(
        &self,
        name: &str,
    ) -> Result<Option<mlua::Value>, crate::engine::modules::TasksResultRetrievalError> {
        let execution_results_guard = self.execution_results.lock().map_err(|_| MutexLockError)?;

        Ok(execution_results_guard
            .get(name)
            .map(|result| result.clone()))
    }

    fn set_task_result(
        &self,
        name: String,
        value: mlua::Value,
    ) -> Result<(), crate::engine::modules::TasksResultSetError> {
        let mut guard = self.execution_results.lock().map_err(|_| MutexLockError)?;

        guard.insert(name, value);

        Ok(())
    }
}

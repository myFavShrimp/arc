use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::{
    engine::modules::tasks::{
        DuplicateTaskError, TaskAdditionError, TaskConfig, Tasks, TasksModule,
        TasksResultResetError, UnregisteredDependenciesError,
    },
    error::MutexLockError,
};

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

    fn tasks(&self) -> Result<Tasks, crate::engine::modules::tasks::TasksAcquisitionError> {
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
    ) -> Result<Option<mlua::Value>, crate::engine::modules::tasks::TasksResultRetrievalError> {
        let execution_results_guard = self.execution_results.lock().map_err(|_| MutexLockError)?;

        Ok(execution_results_guard
            .get(name)
            .map(|result| result.clone()))
    }

    fn set_task_result(
        &self,
        name: String,
        value: mlua::Value,
    ) -> Result<(), crate::engine::modules::tasks::TasksResultSetError> {
        let mut guard = self.execution_results.lock().map_err(|_| MutexLockError)?;

        guard.insert(name, value);

        Ok(())
    }
}

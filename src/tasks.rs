use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::{
    engine::modules::{TaskAdditionError, TasksModule, TasksResultResetError},
    error::MutexLockError,
};

#[derive(Debug, Default, Clone)]
pub struct Tasks {
    pub tasks: HashMap<String, TaskConfig>,
}

#[derive(Debug, Clone)]
pub struct TaskConfig {
    pub func: mlua::Function,
}

#[derive(Debug, Default)]
pub struct TaskRegistrationModule {
    tasks: Arc<Mutex<Tasks>>,
    execution_results: Arc<Mutex<HashMap<String, mlua::Value>>>,
}

impl TasksModule for TaskRegistrationModule {
    fn add_task(&self, name: String, func: mlua::Function) -> Result<(), TaskAdditionError> {
        let mut guard = self.tasks.lock().map_err(|_| MutexLockError)?;

        guard.tasks.insert(name, TaskConfig { func });

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

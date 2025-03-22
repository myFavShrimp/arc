use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use mlua::{IntoLua, MetaMethod, UserData};

use crate::error::{ErrorReport, MutexLockError};

#[derive(Debug, Clone, PartialEq)]
pub struct Task {
    pub name: String,
    pub handler: mlua::Function,
    pub dependencies: Vec<String>,
    pub tags: Vec<String>,
    pub result: Option<mlua::Value>,
}

impl PartialOrd for Task {
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
pub enum TaskFromLuaValueError {
    #[error("Argument 2 of \"tasks.add\" must be a task handler or task configuration")]
    NotAFunctionOrTaskConfig,
    #[error("`dependencies` are invalid")]
    InvalidDependencies(#[source] mlua::Error),
    #[error("`handler` is invalid")]
    InvalidHandler(#[source] mlua::Error),
    #[error("`tags` is invalid")]
    InvalidTags(#[source] mlua::Error),
}

impl TryFrom<(String, mlua::Value)> for Task {
    type Error = TaskFromLuaValueError;

    fn try_from((name, value): (String, mlua::Value)) -> Result<Self, Self::Error> {
        match value {
            mlua::Value::Table(table) => {
                let handler = table
                    .get::<mlua::Function>("handler")
                    .map_err(TaskFromLuaValueError::InvalidHandler)?;

                let dependencies = table
                    .get::<Option<Vec<String>>>("dependencies")
                    .map_err(TaskFromLuaValueError::InvalidDependencies)?
                    .unwrap_or_default();
                let tags = table
                    .get::<Option<Vec<String>>>("tags")
                    .map_err(TaskFromLuaValueError::InvalidTags)?
                    .unwrap_or_default();

                Ok(Task {
                    name,
                    handler,
                    dependencies,
                    tags,
                    result: None,
                })
            }
            mlua::Value::Function(handler) => Ok(Task {
                name,
                handler,
                dependencies: Default::default(),
                tags: Default::default(),
                result: None,
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
            | mlua::Value::Other(_) => Err(TaskFromLuaValueError::NotAFunctionOrTaskConfig),
        }
    }
}

impl IntoLua for Task {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        let task_table = lua.create_table()?;

        task_table.set("name", self.name)?;
        task_table.set("dependecies", self.dependencies)?;
        task_table.set("tags", self.tags)?;
        task_table.set("result", self.result)?;

        task_table.set_readonly(true);

        Ok(mlua::Value::Table(task_table))
    }
}

#[derive(Debug, Default, Clone)]
pub struct Tasks(Arc<Mutex<HashMap<String, Task>>>);

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
    TaskNotDefined(#[from] TaskNotDefinedError),
}

#[derive(Debug, thiserror::Error)]
#[error("Task {0:?} is not defined")]
pub struct TaskNotDefinedError(String);

#[derive(Debug, thiserror::Error)]
#[error("Failed to reset tasks results")]
pub enum TasksResultResetError {
    Lock(#[from] MutexLockError),
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to set task's result")]
pub enum TasksResultSetError {
    Lock(#[from] MutexLockError),
    TaskNotDefined(#[from] TaskNotDefinedError),
}

#[derive(Debug, thiserror::Error)]
#[error("Unregistered task dependencies: {0:?}")]
pub struct UnregisteredDependenciesError(pub Vec<String>);

#[derive(Debug, thiserror::Error)]
#[error("Duplicate task: {0:?}")]
pub struct DuplicateTaskError(pub String);

impl Tasks {
    pub fn tasks_in_execution_order(&self) -> Result<Vec<Task>, TasksAcquisitionError> {
        let guard = self.0.lock().map_err(|_| MutexLockError)?;

        let mut tasks: Vec<Task> = guard.clone().into_values().collect();
        tasks.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());

        Ok(tasks)
    }

    pub fn add(&self, name: String, config: Task) -> Result<(), TaskAdditionError> {
        let mut guard = self.0.lock().map_err(|_| TaskAdditionError {
            task: name.clone(),
            kind: MutexLockError.into(),
        })?;

        let mut unregistered_dependencies = Vec::with_capacity(config.dependencies.len());
        for dep in &config.dependencies {
            if !guard.contains_key(dep) {
                unregistered_dependencies.push(dep.clone());
            }
        }
        if !unregistered_dependencies.is_empty() {
            Err(TaskAdditionError {
                task: name.clone(),
                kind: UnregisteredDependenciesError(unregistered_dependencies).into(),
            })?;
        }

        if guard.insert(name.clone(), config).is_some() {
            Err(TaskAdditionError {
                task: name.clone(),
                kind: DuplicateTaskError(name).into(),
            })?;
        }

        Ok(())
    }

    pub fn reset_results(&self) -> Result<(), TasksResultResetError> {
        let mut guard = self.0.lock().map_err(|_| MutexLockError)?;

        guard.iter_mut().for_each(|(_, task)| task.result = None);

        Ok(())
    }

    pub fn set_task_result(
        &self,
        name: String,
        value: mlua::Value,
    ) -> Result<(), TasksResultSetError> {
        let mut guard = self.0.lock().map_err(|_| MutexLockError)?;

        match guard.get_mut(&name) {
            Some(task) => {
                task.result = Some(value);
            }
            None => Err(TaskNotDefinedError(name.clone()))?,
        };

        Ok(())
    }

    fn get(&self, name: String) -> Result<Task, TasksAcquisitionError> {
        let guard = self.0.lock().map_err(|_| MutexLockError)?;

        Ok(guard
            .get(&name)
            .ok_or(TaskNotDefinedError(name.clone()))?
            .clone())
    }
}

impl UserData for Tasks {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(
            MetaMethod::NewIndex,
            |_, this, (name, config): (String, mlua::Value)| {
                let task = Task::try_from((name.clone(), config))
                    .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))?;

                this.add(name, task)
                    .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
            },
        );

        methods.add_meta_method(MetaMethod::Index, |_, this, (name,): (String,)| {
            this.get(name)
                .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
        });
    }
}

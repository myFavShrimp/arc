use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Task {
    pub name: String,
    pub handler: mlua::Function,
    pub dependencies: Vec<String>,
    pub tags: Vec<String>,
    pub groups: Vec<String>,
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
                    (true, true) | (false, false) => self.name.cmp(&other.name),
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                }
            }
        })
    }
}

pub type Tasks = HashMap<String, Task>;

#[derive(Debug, Default)]
pub struct TasksMemory {
    memory: Tasks,
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
    UnregisteredDependencies(#[from] UnregisteredDependenciesError),
    DuplicateTask(#[from] DuplicateTaskError),
}

#[derive(Debug, thiserror::Error)]
#[error("Unregistered task dependencies: {0:?}")]
pub struct UnregisteredDependenciesError(pub Vec<String>);

#[derive(Debug, thiserror::Error)]
#[error("Duplicate task")]
pub struct DuplicateTaskError;

#[derive(Debug, thiserror::Error)]
#[error("Failed to set task's result")]
pub enum TasksResultSetError {
    TaskNotDefined(#[from] TaskNotDefinedError),
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to retrieve tasks configuration")]
pub enum TaskRetrievalError {
    TaskNotDefined(#[from] TaskNotDefinedError),
}

#[derive(Debug, thiserror::Error)]
#[error("Task {0:?} is not defined")]
pub struct TaskNotDefinedError(String);

impl TasksMemory {
    pub fn add(&mut self, task: Task) -> Result<(), TaskAdditionError> {
        if self
            .memory
            .insert(task.name.clone(), task.clone())
            .is_some()
        {
            Err(TaskAdditionError {
                task: task.name.clone(),
                kind: DuplicateTaskError.into(),
            })?;
        }

        let mut unregistered_dependencies = Vec::with_capacity(task.dependencies.len());
        for dep in &task.dependencies {
            if !self.memory.contains_key(dep) {
                unregistered_dependencies.push(dep.clone());
            }
        }
        if !unregistered_dependencies.is_empty() {
            Err(TaskAdditionError {
                task: task.name,
                kind: UnregisteredDependenciesError(unregistered_dependencies).into(),
            })?;
        }

        Ok(())
    }

    pub fn all(&self) -> Tasks {
        self.memory.clone()
    }

    pub fn reset_results(&mut self) {
        self.memory
            .iter_mut()
            .for_each(|(_, task)| task.result = None);
    }

    pub fn set_task_result(
        &mut self,
        task_name: &str,
        value: mlua::Value,
    ) -> Result<(), TasksResultSetError> {
        match self.memory.get_mut(task_name) {
            Some(task) => {
                task.result = Some(value);
            }
            None => Err(TaskNotDefinedError(task_name.to_string()))?,
        };

        Ok(())
    }

    pub fn get(&self, task_name: &str) -> Result<Task, TaskRetrievalError> {
        Ok(self
            .memory
            .get(task_name)
            .ok_or(TaskNotDefinedError(task_name.to_string()))?
            .clone())
    }
}

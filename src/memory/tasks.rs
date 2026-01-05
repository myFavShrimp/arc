use std::collections::HashSet;

use indexmap::IndexMap;
use strum::EnumString;

#[derive(Debug, Clone, Copy, PartialEq, Default, EnumString, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum OnFailBehavior {
    #[default]
    Continue,
    SkipSystem,
    Abort,
}

#[derive(Debug, Clone, Copy, PartialEq, EnumString, strum::Display)]
#[strum(serialize_all = "snake_case")]
pub enum TaskState {
    Success,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Task {
    pub name: String,
    pub handler: mlua::Function,
    pub when: Option<mlua::Function>,
    pub on_fail: OnFailBehavior,
    pub tags: HashSet<String>,
    pub groups: HashSet<String>,
    pub requires: HashSet<String>,
    pub important: bool,
    pub result: Option<mlua::Value>,
    pub state: Option<TaskState>,
    pub error: Option<String>,
}

pub type Tasks = IndexMap<String, Task>;

#[derive(Debug, Default)]
pub struct TasksMemory {
    memory: Tasks,
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to add task `{0}`: duplicate task")]
pub struct TaskAdditionError(pub String);

#[derive(Debug, thiserror::Error)]
#[error("Failed to set task's result")]
pub enum TasksResultSetError {
    TaskNotDefined(#[from] TaskNotDefinedError),
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to set task's state")]
pub enum TasksStateSetError {
    TaskNotDefined(#[from] TaskNotDefinedError),
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to set task's error")]
pub enum TasksErrorSetError {
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
    pub fn add(&mut self, mut task: Task) -> Result<(), TaskAdditionError> {
        task.tags.insert(task.name.clone());

        if self
            .memory
            .insert(task.name.clone(), task.clone())
            .is_some()
        {
            return Err(TaskAdditionError(task.name));
        }

        Ok(())
    }

    pub fn all(&self) -> Tasks {
        self.memory.clone()
    }

    pub fn reset_execution_state(&mut self) {
        self.memory.iter_mut().for_each(|(_, task)| {
            task.result = None;
            task.state = None;
            task.error = None;
        });
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

    pub fn set_task_state(
        &mut self,
        task_name: &str,
        state: TaskState,
    ) -> Result<(), TasksStateSetError> {
        match self.memory.get_mut(task_name) {
            Some(task) => {
                task.state = Some(state);
            }
            None => Err(TaskNotDefinedError(task_name.to_string()))?,
        };

        Ok(())
    }

    pub fn set_task_error(
        &mut self,
        task_name: &str,
        error: String,
    ) -> Result<(), TasksErrorSetError> {
        match self.memory.get_mut(task_name) {
            Some(task) => {
                task.error = Some(error);
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

use std::collections::{HashMap, HashSet};

use crate::memory::tasks::Task;

pub struct Recipe {
    pub tasks: Vec<Task>,
}

#[derive(thiserror::Error, Debug)]
pub enum RecipeCreationError {
    #[error("Cyclic dependency: {0}")]
    CyclicDependency(String),
}

impl Recipe {
    pub fn from_tasks(tasks: &[Task]) -> Result<Self, RecipeCreationError> {
        let mut ordered_tasks = Vec::new();

        let mut visited = HashSet::new();
        let mut temp_visited = HashSet::new();

        let task_map: HashMap<&String, &Task> = tasks.iter().map(|t| (&t.name, t)).collect();

        fn visit<'a>(
            task: &'a Task,
            task_map: &HashMap<&String, &'a Task>,
            visited: &mut HashSet<&'a str>,
            temp_visited: &mut HashSet<&'a str>,
            result: &mut Vec<Task>,
        ) -> Result<(), RecipeCreationError> {
            if temp_visited.contains(task.name.as_str()) {
                return Err(RecipeCreationError::CyclicDependency(task.name.clone()));
            }

            if visited.contains(task.name.as_str()) {
                return Ok(());
            }

            temp_visited.insert(task.name.as_str());

            for dep in &task.dependencies {
                if let Some(dep_task) = task_map.get(dep) {
                    visit(dep_task, task_map, visited, temp_visited, result)?;
                }
            }

            temp_visited.remove(task.name.as_str());
            visited.insert(task.name.as_str());
            result.push(task.clone());

            Ok(())
        }

        for task in tasks {
            if !visited.contains(task.name.as_str()) {
                visit(
                    task,
                    &task_map,
                    &mut visited,
                    &mut temp_visited,
                    &mut ordered_tasks,
                )?
            }
        }

        Ok(Recipe {
            tasks: ordered_tasks,
        })
    }
}

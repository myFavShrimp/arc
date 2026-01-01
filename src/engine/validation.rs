use std::collections::HashSet;

use crate::memory::{target_groups::TargetGroups, target_systems::TargetSystems, tasks::Tasks};

use super::selection::{GroupSelection, SystemSelection, TagSelection};

#[derive(Debug, thiserror::Error)]
#[error("The selected group {0:?} does not exist")]
pub struct MissingSelectedGroupError(pub Vec<String>);

pub fn validate_selected_groups(
    groups: &TargetGroups,
    selection: &GroupSelection,
) -> Result<(), MissingSelectedGroupError> {
    if let GroupSelection::Set(requested) = selection {
        let missing: Vec<String> = requested
            .iter()
            .filter(|name| !groups.contains_key(*name))
            .cloned()
            .collect();

        if !missing.is_empty() {
            return Err(MissingSelectedGroupError(missing));
        }
    }

    Ok(())
}

#[derive(Debug, thiserror::Error)]
#[error("The selected system {0:?} does not exist")]
pub struct MissingSelectedSystemError(pub Vec<String>);

pub fn validate_selected_systems(
    systems: &TargetSystems,
    selection: &SystemSelection,
) -> Result<(), MissingSelectedSystemError> {
    if let SystemSelection::Set(requested) = selection {
        let missing: Vec<String> = requested
            .iter()
            .filter(|name| !systems.contains_key(*name))
            .cloned()
            .collect();

        if !missing.is_empty() {
            return Err(MissingSelectedSystemError(missing));
        }
    }

    Ok(())
}

#[derive(Debug, thiserror::Error)]
#[error("The selected tag {0:?} does not exist")]
pub struct MissingSelectedTagError(pub Vec<String>);

pub fn validate_selected_tags(
    tasks: &Tasks,
    selection: &TagSelection,
) -> Result<(), MissingSelectedTagError> {
    if let TagSelection::Set(requested) = selection {
        let all_tags: HashSet<&String> = tasks.values().flat_map(|task| task.tags.iter()).collect();

        let missing: Vec<String> = requested
            .iter()
            .filter(|tag| !all_tags.contains(tag))
            .cloned()
            .collect();

        if !missing.is_empty() {
            return Err(MissingSelectedTagError(missing));
        }
    }

    Ok(())
}

#[derive(Debug)]
pub struct UndefinedDependenciesError(pub Vec<(String, Vec<String>)>);

impl std::error::Error for UndefinedDependenciesError {}

impl std::fmt::Display for UndefinedDependenciesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Tasks have undefined dependencies:")?;
        for (task_name, tags) in &self.0 {
            let tags_list: Vec<_> = tags.iter().map(|t| format!("{t:?}")).collect();
            writeln!(
                f,
                "  - task {task_name:?} depends on undefined tags {tags_list:?}",
            )?;
        }
        Ok(())
    }
}

pub fn validate_task_dependencies(tasks: &Tasks) -> Result<(), UndefinedDependenciesError> {
    let all_tags: HashSet<&String> = tasks.values().flat_map(|task| task.tags.iter()).collect();

    let undefined: Vec<(String, Vec<String>)> = tasks
        .iter()
        .filter_map(|(task_name, task)| {
            let missing: Vec<String> = task
                .dependencies
                .iter()
                .filter(|dep_tag| !all_tags.contains(dep_tag))
                .cloned()
                .collect();

            if missing.is_empty() {
                None
            } else {
                Some((task_name.clone(), missing))
            }
        })
        .collect();

    if undefined.is_empty() {
        Ok(())
    } else {
        Err(UndefinedDependenciesError(undefined))
    }
}

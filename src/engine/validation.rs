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
pub struct MissingSelectedSystemError(Vec<String>);

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
#[error("Group names conflict with system names: {0:?}")]
pub struct GroupSystemNameConflictError(Vec<String>);

pub fn validate_group_system_names(
    groups: &TargetGroups,
    systems: &TargetSystems,
) -> Result<(), GroupSystemNameConflictError> {
    let conflicting_names: Vec<String> = groups
        .keys()
        .filter(|name| systems.contains_key(*name))
        .cloned()
        .collect();

    if conflicting_names.is_empty() {
        Ok(())
    } else {
        Err(GroupSystemNameConflictError(conflicting_names))
    }
}

#[derive(Debug)]
pub struct UndefinedGroupMembersError(pub Vec<(String, Vec<String>)>);

impl std::error::Error for UndefinedGroupMembersError {}

impl std::fmt::Display for UndefinedGroupMembersError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Groups have undefined members:")?;

        for (group_name, members) in &self.0 {
            let members_list: Vec<_> = members.iter().map(|m| format!("{m:?}")).collect();

            writeln!(
                f,
                "  - group {group_name:?} has undefined members {members_list:?}",
            )?;
        }
        Ok(())
    }
}

pub fn validate_group_members(
    groups: &TargetGroups,
    systems: &TargetSystems,
) -> Result<(), UndefinedGroupMembersError> {
    let undefined_members: Vec<(String, Vec<String>)> = groups
        .iter()
        .filter_map(|(group_name, group)| {
            let missing: Vec<String> = group
                .members
                .iter()
                .filter(|member| !systems.contains_key(*member))
                .cloned()
                .collect();

            if missing.is_empty() {
                None
            } else {
                Some((group_name.clone(), missing))
            }
        })
        .collect();

    if undefined_members.is_empty() {
        Ok(())
    } else {
        Err(UndefinedGroupMembersError(undefined_members))
    }
}

#[derive(Debug, thiserror::Error)]
#[error("The selected tag {0:?} does not exist")]
pub struct MissingSelectedTagError(Vec<String>);

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
pub struct UndefinedRequiresError(Vec<(String, Vec<String>)>);

impl std::error::Error for UndefinedRequiresError {}

impl std::fmt::Display for UndefinedRequiresError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Tasks have undefined requires:")?;

        for (task_name, tags) in &self.0 {
            let tags_list: Vec<_> = tags.iter().map(|t| format!("{t:?}")).collect();

            writeln!(
                f,
                "  - task {task_name:?} requires undefined tags {tags_list:?}",
            )?;
        }
        Ok(())
    }
}

pub fn validate_task_requires(tasks: &Tasks) -> Result<(), UndefinedRequiresError> {
    let all_tags: HashSet<&String> = tasks.values().flat_map(|task| task.tags.iter()).collect();

    let undefined_requires: Vec<(String, Vec<String>)> = tasks
        .iter()
        .filter_map(|(task_name, task)| {
            let missing: Vec<String> = task
                .requires
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

    if undefined_requires.is_empty() {
        Ok(())
    } else {
        Err(UndefinedRequiresError(undefined_requires))
    }
}

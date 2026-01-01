use std::collections::HashSet;

use crate::memory::{
    target_groups::TargetGroups,
    target_systems::TargetSystems,
    tasks::{Task, Tasks},
};

#[derive(Debug)]
pub enum GroupSelection {
    All,
    Set(HashSet<String>),
}

impl GroupSelection {
    pub fn contains(&self, group_name: &str) -> bool {
        match self {
            GroupSelection::All => true,
            GroupSelection::Set(selected_set) => selected_set.contains(group_name),
        }
    }
}

#[derive(Debug)]
pub enum TagSelection {
    All,
    Set(HashSet<String>),
}

#[derive(Debug)]
pub enum SystemSelection {
    All,
    Set(HashSet<String>),
}

impl SystemSelection {
    pub fn contains(&self, system_name: &str) -> bool {
        match self {
            SystemSelection::All => true,
            SystemSelection::Set(selected_set) => selected_set.contains(system_name),
        }
    }
}

pub fn task_matches_groups(task: &Task, selection: &GroupSelection) -> bool {
    match selection {
        GroupSelection::All => true,
        GroupSelection::Set(selected_set) => !task.groups.is_disjoint(selected_set),
    }
}

pub fn task_matches_tags(task: &Task, selection: &TagSelection) -> bool {
    match selection {
        TagSelection::All => true,
        TagSelection::Set(selected_set) => !task.tags.is_disjoint(selected_set),
    }
}

pub fn select_groups(mut groups: TargetGroups, selection: &GroupSelection) -> TargetGroups {
    groups.retain(|name, _| selection.contains(name));
    groups
}

pub fn select_systems(
    mut systems: TargetSystems,
    groups: &TargetGroups,
    system_selection: &SystemSelection,
    group_selection: &GroupSelection,
) -> TargetSystems {
    let mut filtered_groups = groups.clone();
    filtered_groups.retain(|name, _| group_selection.contains(name));

    systems.retain(|name, _| {
        let matches_groups = filtered_groups.is_empty()
            || filtered_groups
                .iter()
                .any(|(_, group)| group.members.contains(name));
        let matches_systems = system_selection.contains(name);

        matches_groups && matches_systems
    });

    systems
}

pub fn select_tasks(
    mut tasks: Tasks,
    group_selection: &GroupSelection,
    tag_selection: &TagSelection,
) -> Tasks {
    tasks.retain(|_, task| {
        let matches_groups = task_matches_groups(task, group_selection);
        let matches_tags = task_matches_tags(task, tag_selection);

        matches_groups && (task.important || matches_tags)
    });

    tasks
}

pub fn select_tasks_with_dependencies(
    all_tasks: Tasks,
    group_selection: &GroupSelection,
    tag_selection: &TagSelection,
) -> Tasks {
    let all_tags: HashSet<&String> = all_tasks
        .values()
        .flat_map(|task| task.tags.iter())
        .collect();

    let tasks_with_tag = |tag: &String| -> Vec<&String> {
        all_tasks
            .iter()
            .filter(|(_, task)| {
                task.tags.contains(tag) && task_matches_groups(task, group_selection)
            })
            .map(|(name, _)| name)
            .collect()
    };

    let mut selected_task_names: HashSet<String> = all_tasks
        .iter()
        .filter(|(_, task)| {
            task_matches_groups(task, group_selection)
                && (task.important || task_matches_tags(task, tag_selection))
        })
        .map(|(name, _)| name.clone())
        .collect();

    let mut tasks_to_expand: Vec<String> = selected_task_names.iter().cloned().collect();

    while let Some(task_name) = tasks_to_expand.pop() {
        let Some(task) = all_tasks.get(&task_name) else {
            continue;
        };

        for dependency_tag in &task.dependencies {
            if !all_tags.contains(dependency_tag) {
                continue;
            }

            for name in tasks_with_tag(dependency_tag) {
                if selected_task_names.insert(name.clone()) {
                    tasks_to_expand.push(name.clone());
                }
            }
        }
    }

    let mut selected_tasks = all_tasks;
    selected_tasks.retain(|name, _| selected_task_names.contains(name));

    selected_tasks
}

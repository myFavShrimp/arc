use serde::Serialize;
use tabled::{
    Table, Tabled,
    settings::{Padding, Style, object::Columns},
};

use crate::{
    cli::ListItemType,
    engine::Engine,
    error::MutexLockError,
    memory::{
        target_groups::TargetGroups,
        target_systems::{TargetSystemKind, TargetSystems},
        tasks::{Task as MemoryTask, Tasks},
    },
};

#[derive(Debug, thiserror::Error)]
#[error("Failed to list items")]
pub enum ListError {
    Lock(#[from] MutexLockError),
    Serialization(#[from] serde_json::Error),
}

fn format_list(items: &[String]) -> String {
    if items.is_empty() {
        String::new()
    } else {
        items.join(",")
    }
}

#[derive(Serialize, Tabled)]
#[tabled(rename_all = "UPPERCASE")]
struct Task {
    name: String,
    #[tabled(display = "format_list")]
    tags: Vec<String>,
    #[tabled(display = "format_list")]
    targets: Vec<String>,
    #[tabled(display = "format_list")]
    requires: Vec<String>,
    important: bool,
    on_fail: String,
}

#[derive(Serialize, Tabled)]
#[tabled(rename_all = "UPPERCASE")]
struct System {
    name: String,
    kind: String,
    address: String,
    port: String,
    user: String,
    #[tabled(display = "format_list")]
    groups: Vec<String>,
}

#[derive(Serialize, Tabled)]
#[tabled(rename_all = "UPPERCASE")]
struct Group {
    name: String,
    #[tabled(display = "format_list")]
    members: Vec<String>,
}

fn convert_tasks(tasks: &Tasks) -> Vec<Task> {
    let mut result: Vec<Task> = tasks
        .values()
        .map(|task| {
            let mut tags: Vec<String> = task.tags.iter().cloned().collect();
            tags.sort();
            let mut targets: Vec<String> = task.targets.iter().cloned().collect();
            targets.sort();
            let mut requires: Vec<String> = task.requires.iter().cloned().collect();
            requires.sort();

            Task {
                name: task.name.clone(),
                tags,
                targets,
                requires,
                important: task.important,
                on_fail: task.on_fail.to_string(),
            }
        })
        .collect();

    result.sort_by(|a, b| a.name.cmp(&b.name));

    result
}

fn convert_groups(groups: &TargetGroups) -> Vec<Group> {
    let mut result: Vec<Group> = groups
        .values()
        .map(|group| {
            let mut members = group.members.clone();
            members.sort();

            Group {
                name: group.name.clone(),
                members,
            }
        })
        .collect();

    result.sort_by(|a, b| a.name.cmp(&b.name));

    result
}

fn convert_systems(systems: &TargetSystems, groups: &TargetGroups) -> Vec<System> {
    let mut result: Vec<System> = systems
        .values()
        .map(|system| {
            let mut system_groups: Vec<String> = groups
                .values()
                .filter(|g| g.members.contains(&system.name))
                .map(|g| g.name.clone())
                .collect();

            system_groups.sort();

            match &system.kind {
                TargetSystemKind::Remote(remote) => System {
                    name: system.name.clone(),
                    kind: "remote".to_string(),
                    address: remote.address.to_string(),
                    port: remote.port.to_string(),
                    user: remote.user.clone(),
                    groups: system_groups,
                },
                TargetSystemKind::Local => System {
                    name: system.name.clone(),
                    kind: "local".to_string(),
                    address: String::new(),
                    port: String::new(),
                    user: String::new(),
                    groups: system_groups,
                },
            }
        })
        .collect();

    result.sort_by(|a, b| a.name.cmp(&b.name));

    result
}

fn print_json<T: Serialize>(value: &T) -> Result<(), ListError> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn print_table<T: Tabled>(items: Vec<T>) {
    let mut table = Table::new(items);
    table
        .with(Style::blank())
        .modify(Columns::first(), Padding::new(0, 1, 0, 0))
        .modify(Columns::last(), Padding::new(1, 0, 0, 0));

    println!("{}", table);
}

pub(crate) fn list_system_tasks(tasks: &[MemoryTask]) {
    let display_tasks: Vec<Task> = tasks
        .iter()
        .map(|task| Task {
            name: task.name.clone(),
            tags: task.tags.iter().cloned().collect(),
            targets: task.targets.iter().cloned().collect(),
            requires: task.requires.iter().cloned().collect(),
            important: task.important,
            on_fail: task.on_fail.to_string(),
        })
        .collect();

    print_table(display_tasks);
}

pub fn list(engine: &Engine, item_type: ListItemType, json: bool) -> Result<(), ListError> {
    let state = engine.state();
    let tasks = state.all_tasks()?;
    let groups = state.all_groups()?;
    let systems = state.all_systems()?;

    match item_type {
        ListItemType::Tasks => {
            let tasks = convert_tasks(&tasks);

            if json {
                print_json(&tasks)?;
            } else {
                print_table(tasks);
            }
        }
        ListItemType::Groups => {
            let groups = convert_groups(&groups);

            if json {
                print_json(&groups)?;
            } else {
                print_table(groups);
            }
        }
        ListItemType::Systems => {
            let systems = convert_systems(&systems, &groups);

            if json {
                print_json(&systems)?;
            } else {
                print_table(systems);
            }
        }
    }

    Ok(())
}

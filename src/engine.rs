use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use delegator::{
    executor::{ExecutionTargetSetError, Executor},
    operator::{FileSystemOperator, OperationTargetSetError},
};
use mlua::{Lua, LuaOptions, StdLib};
use modules::{Modules, MountToGlobals};
use objects::system::System;
use state::{
    State, TasksErrorStateSetError, TasksExecutionStateResetError, TasksResultStateSetError,
    TasksStateStateSetError,
};

use crate::{
    engine::objects::system::SystemKind,
    error::MutexLockError,
    logger::{Logger, SharedLogger},
    memory::{
        target_groups::TargetGroupsMemory,
        target_systems::{TargetSystemKind, TargetSystemsMemory},
        tasks::{OnFailBehavior, TaskState, TasksMemory},
    },
};

pub mod delegator;
pub mod modules;
pub mod objects;
mod readonly;
pub mod state;

pub struct Engine {
    lua: Lua,
    state: State,
    is_dry_run: bool,
    logger: SharedLogger,
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to create engine")]
pub enum EngineBuilderCreationError {
    Lua(#[from] mlua::Error),
}

static ENTRY_POINT_SCRIPT: &str = "arc.lua";

#[derive(thiserror::Error, Debug)]
#[error("Failed to run scripts")]
pub enum EngineExecutionError {
    Io(#[from] std::io::Error),
    Lua(#[from] mlua::Error),
    ExecutionTargetSet(#[from] ExecutionTargetSetError),
    OperationTargetSet(#[from] OperationTargetSetError),
    FilteredGroupDoesNotExistError(#[from] FilteredGroupDoesNotExistError),
    Lock(#[from] MutexLockError),
    TasksExecutionStateReset(#[from] TasksExecutionStateResetError),
    TasksResultSet(#[from] TasksResultStateSetError),
    TasksStateSet(#[from] TasksStateStateSetError),
    TasksErrorSet(#[from] TasksErrorStateSetError),
    #[error("Task '{task}' aborted execution: {error}")]
    TaskAborted {
        task: String,
        error: String,
    },
}

#[derive(Debug, thiserror::Error)]
#[error("The filtered group {0:?} does not exist")]
pub struct FilteredGroupDoesNotExistError(Vec<String>);

impl Engine {
    pub fn new(logger: Logger, is_dry_run: bool) -> Result<Self, EngineBuilderCreationError> {
        let logger = Arc::new(Mutex::new(logger));
        let mut lua = Lua::new_with(StdLib::ALL_SAFE, LuaOptions::new().catch_rust_panics(true))?;

        let target_systems_memory = Arc::new(Mutex::new(TargetSystemsMemory::default()));
        let target_groups_memory = Arc::new(Mutex::new(TargetGroupsMemory::default()));
        #[allow(clippy::arc_with_non_send_sync)]
        let tasks_memory = Arc::new(Mutex::new(TasksMemory::default()));

        Modules::new(
            target_systems_memory.clone(),
            target_groups_memory.clone(),
            tasks_memory.clone(),
            logger.clone(),
        )
        .mount_to_globals(&mut lua)?;

        Ok(Self {
            lua,
            state: State::new(target_systems_memory, target_groups_memory, tasks_memory),
            is_dry_run,
            logger,
        })
    }

    pub fn execute(
        &self,
        tags: HashSet<String>,
        groups: HashSet<String>,
        no_deps: bool,
    ) -> Result<(), EngineExecutionError> {
        let entry_point_script_path = PathBuf::from(ENTRY_POINT_SCRIPT);
        let entry_point_script = std::fs::read_to_string(&entry_point_script_path)?;

        self.lua
            .load(entry_point_script)
            .set_name(entry_point_script_path.to_string_lossy())
            .exec()?;

        let systems = self.state.systems_for_selected_groups(&groups)?;
        let tasks = if no_deps {
            self.state
                .tasks_for_selected_groups_and_tags(&groups, &tags)?
        } else {
            let (resolved_tasks, undefined_dependencies) = self
                .state
                .tasks_with_resolved_dependencies(&groups, &tags)?;

            for undefined_dependency in undefined_dependencies {
                let logger = self.logger.lock().unwrap();
                logger.warn(&format!(
                    "Task {:?} depends on tag {:?} but no tasks have that tag",
                    undefined_dependency.task_name, undefined_dependency.tag
                ));
            }

            resolved_tasks
        };

        let tasks_to_execute: Vec<_> = tasks.into_values().collect();

        let missing_selected_groups = self.state.missing_selected_groups(&groups)?;
        if !missing_selected_groups.is_empty() {
            Err(FilteredGroupDoesNotExistError(
                missing_selected_groups.clone(),
            ))?
        }

        let selected_groups = self.state.selected_groups(&groups)?;

        for (system_name, system_config) in systems {
            let system_groups = selected_groups
                .iter()
                .filter(|(_, config)| config.members.contains(&system_name))
                .map(|(name, _)| name)
                .collect::<Vec<&String>>();
            let system_tasks = tasks_to_execute
                .iter()
                .filter(|task| {
                    system_groups.is_empty()
                        || task.groups.is_empty()
                        || task
                            .groups
                            .iter()
                            .any(|group| system_groups.contains(&group))
                })
                .collect::<Vec<_>>();

            let mut logger = self.logger.lock().unwrap();
            logger.current_system(&system_name);
            drop(logger);

            if system_tasks.is_empty() {
                continue;
            }

            if self.is_dry_run {
                let mut logger = self.logger.lock().unwrap();

                for task in &system_tasks {
                    logger.info(&format!(
                        "{} {}",
                        task.name,
                        task.tags
                            .iter()
                            .map(|t| format!("#{t}"))
                            .collect::<Vec<_>>()
                            .join(" ")
                    ));
                }

                logger.reset_system();

                drop(logger);

                continue;
            }

            self.state.reset_execution_state()?;

            let system = System {
                name: system_config.name.clone(),
                kind: match &system_config.kind {
                    TargetSystemKind::Remote(remote_target_system) => {
                        SystemKind::Remote(objects::system::RemoteSystem {
                            address: remote_target_system.address,
                            port: remote_target_system.port,
                            user: remote_target_system.user.clone(),
                            executor: Executor::new_for_system(&system_config)?,
                            file_system_operator: FileSystemOperator::new_for_system(
                                &system_config,
                            )?,
                        })
                    }
                    TargetSystemKind::Local => {
                        SystemKind::Local(Executor::new_local(), FileSystemOperator::new_local())
                    }
                },
            };

            let mut skip_system = false;

            for task_config in system_tasks {
                if skip_system && !task_config.important {
                    self.state
                        .set_task_state(&task_config.name, TaskState::Skipped)?;
                    continue;
                }

                if let Some(when_handler) = &task_config.when {
                    let should_run: bool = when_handler.call(())?;
                    if !should_run {
                        self.state
                            .set_task_state(&task_config.name, TaskState::Skipped)?;
                        continue;
                    }
                }

                match task_config.handler.call::<mlua::Value>(system.clone()) {
                    Ok(result) => {
                        self.state.set_task_result(&task_config.name, result)?;
                        self.state
                            .set_task_state(&task_config.name, TaskState::Success)?;
                    }
                    Err(e) => {
                        let error_msg = e.to_string();
                        self.state
                            .set_task_state(&task_config.name, TaskState::Failed)?;
                        self.state
                            .set_task_error(&task_config.name, error_msg.clone())?;

                        match task_config.on_fail {
                            OnFailBehavior::Continue => {}
                            OnFailBehavior::SkipSystem => {
                                skip_system = true;
                            }
                            OnFailBehavior::Abort => {
                                return Err(EngineExecutionError::TaskAborted {
                                    task: task_config.name.clone(),
                                    error: error_msg,
                                });
                            }
                        }
                    }
                }
            }

            let mut logger = self.logger.lock().unwrap();
            logger.reset_system();
        }

        Ok(())
    }
}

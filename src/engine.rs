use std::{
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
use selection::{
    GroupSelection, SystemSelection, TagSelection, select_groups, select_systems, select_tasks,
    select_tasks_with_dependencies,
};
use state::{
    State, TasksErrorStateSetError, TasksExecutionStateResetError, TasksResultStateSetError,
    TasksStateStateSetError,
};
use validation::{
    MissingSelectedGroupError, MissingSelectedSystemError, MissingSelectedTagError,
    UndefinedDependenciesError, validate_selected_groups, validate_selected_systems,
    validate_selected_tags, validate_task_dependencies,
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
pub mod selection;
pub mod state;
pub mod validation;

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
#[error("Runtime error")]
pub enum EngineExecutionError {
    EntrypointExecution(#[from] EntrypointExecutionError),
    Lua(#[from] mlua::Error),
    ExecutionTargetSet(#[from] ExecutionTargetSetError),
    OperationTargetSet(#[from] OperationTargetSetError),
    MissingSelectedGroup(#[from] MissingSelectedGroupError),
    MissingSelectedSystem(#[from] MissingSelectedSystemError),
    MissingSelectedTag(#[from] MissingSelectedTagError),
    UndefinedDependencies(#[from] UndefinedDependenciesError),
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

#[derive(thiserror::Error, Debug)]
#[error("Failed to execute arc entrypoint")]
pub enum EntrypointExecutionError {
    Lua(#[from] mlua::Error),
    Io(#[from] std::io::Error),
}

impl Engine {
    pub fn new(logger: Logger, is_dry_run: bool) -> Result<Self, EngineBuilderCreationError> {
        let logger = Arc::new(Mutex::new(logger));
        let mut lua = Lua::new_with(
            StdLib::TABLE | StdLib::STRING | StdLib::PACKAGE | StdLib::BIT | StdLib::MATH,
            LuaOptions::new().catch_rust_panics(true),
        )?;

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

    pub fn execute_entrypoint(&self) -> Result<(), EntrypointExecutionError> {
        let entry_point_script_path = PathBuf::from(ENTRY_POINT_SCRIPT);
        let entry_point_script = std::fs::read_to_string(&entry_point_script_path)?;

        self.lua
            .load(entry_point_script)
            .set_name(entry_point_script_path.to_string_lossy())
            .exec()?;

        Ok(())
    }

    pub fn execute(
        &self,
        tags_selection: TagSelection,
        groups_selection: GroupSelection,
        systems_selection: SystemSelection,
        no_deps: bool,
    ) -> Result<(), EngineExecutionError> {
        self.execute_entrypoint()?;

        let all_groups = self.state.all_groups()?;
        let all_systems = self.state.all_systems()?;
        let all_tasks = self.state.all_tasks()?;

        validate_selected_groups(&all_groups, &groups_selection)?;
        validate_selected_systems(&all_systems, &systems_selection)?;
        validate_selected_tags(&all_tasks, &tags_selection)?;
        validate_task_dependencies(&all_tasks)?;

        let selected_groups = select_groups(all_groups, &groups_selection);
        let selected_systems = select_systems(
            all_systems,
            &selected_groups,
            &systems_selection,
            &groups_selection,
        );

        let tasks_to_execute = if no_deps {
            select_tasks(all_tasks, &groups_selection, &tags_selection)
        } else {
            select_tasks_with_dependencies(all_tasks, &groups_selection, &tags_selection)
        };

        for (system_name, system_config) in selected_systems {
            let system_groups: Vec<&String> = selected_groups
                .iter()
                .filter(|(_, config)| config.members.contains(&system_name))
                .map(|(name, _)| name)
                .collect();

            let system_tasks: Vec<_> = tasks_to_execute
                .iter()
                .filter(|(_task_name, task)| {
                    system_groups.is_empty()
                        || task.groups.is_empty()
                        || task
                            .groups
                            .iter()
                            .any(|group| system_groups.contains(&group))
                })
                .collect();

            let mut logger = self.logger.lock().unwrap();
            logger.current_system(&system_name);
            drop(logger);

            if system_tasks.is_empty() {
                continue;
            }

            if self.is_dry_run {
                let mut logger = self.logger.lock().unwrap();

                for (_task_name, task) in &system_tasks {
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

            for (_task_name, task_config) in system_tasks {
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
                        let error_message = e.to_string();

                        {
                            let logger = self.logger.lock().unwrap();
                            logger.error(&format!(
                                "Task '{}' failed: {}",
                                task_config.name, error_message
                            ));
                        }

                        self.state
                            .set_task_state(&task_config.name, TaskState::Failed)?;
                        self.state
                            .set_task_error(&task_config.name, error_message.clone())?;

                        match task_config.on_fail {
                            OnFailBehavior::Continue => {}
                            OnFailBehavior::SkipSystem => {
                                skip_system = true;
                            }
                            OnFailBehavior::Abort => {
                                return Err(EngineExecutionError::TaskAborted {
                                    task: task_config.name.clone(),
                                    error: error_message,
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

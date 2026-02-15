use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use indexmap::IndexMap;

use delegator::{
    executor::{ExecutionTargetSetError, Executor},
    operator::{FileSystemOperator, OperationTargetSetError},
};
use mlua::{Lua, LuaOptions, StdLib};
use modules::{Modules, MountToGlobals};
use objects::system::System;
use selection::{
    GroupSelection, SystemSelection, TagSelection, select_groups, select_groups_for_system,
    select_systems, select_tasks, select_tasks_for_system, select_tasks_with_requires,
};
use state::{
    State, TasksErrorStateSetError, TasksExecutionStateResetError, TasksResultStateSetError,
    TasksStateStateSetError,
};
use validation::{
    GroupSystemNameConflictError, MissingSelectedGroupError, MissingSelectedSystemError,
    MissingSelectedTagError, UndefinedGroupMembersError, UndefinedRequiresError,
    UndefinedTaskTargetsError, validate_group_members, validate_group_system_names,
    validate_selected_groups, validate_selected_systems, validate_selected_tags,
    validate_task_requires, validate_task_targets,
};

use crate::{
    engine::{delegator::error::FfiPanicError, objects::system::SystemKind},
    error::MutexLockError,
    logger::{LogLevel, Logger},
    memory::{
        target_groups::TargetGroupsMemory,
        target_systems::{TargetSystem, TargetSystemKind, TargetSystemsMemory},
        tasks::{OnFailBehavior, Task, TaskState, TasksMemory},
    },
    progress::{ProgressContext, SystemLogger, SystemLoggerCreationError, TaskLoggerCreationError},
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
    logger: Logger,
    progress: ProgressContext,
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
    Validation(#[from] ValidationError),
    TaskExecution(#[from] TaskExecutionError),
    ExecutionTargetSet(#[from] ExecutionTargetSetError),
    OperationTargetSet(#[from] OperationTargetSetError),
    TasksExecutionStateReset(#[from] TasksExecutionStateResetError),
    SystemLoggerCreation(#[from] SystemLoggerCreationError),
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to execute arc entrypoint")]
pub enum EntrypointExecutionError {
    Lua(#[from] mlua::Error),
    Io(#[from] std::io::Error),
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to validate and filter selection")]
pub enum ValidationError {
    MissingSelectedGroup(#[from] MissingSelectedGroupError),
    MissingSelectedSystem(#[from] MissingSelectedSystemError),
    MissingSelectedTag(#[from] MissingSelectedTagError),
    GroupSystemNameConflict(#[from] GroupSystemNameConflictError),
    UndefinedGroupMembers(#[from] UndefinedGroupMembersError),
    UndefinedTaskTargets(#[from] UndefinedTaskTargetsError),
    UndefinedRequires(#[from] UndefinedRequiresError),
    Lock(#[from] MutexLockError),
}

#[derive(Debug, thiserror::Error)]
#[error("Task {task:?} aborted execution: {error}")]
pub struct TaskAbortedError {
    pub task: String,
    pub error: String,
}

#[derive(thiserror::Error, Debug)]
#[error("Failed to run tasks on system")]
pub enum TaskExecutionError {
    Lua(#[from] mlua::Error),
    TasksResultSet(#[from] TasksResultStateSetError),
    TasksStateSet(#[from] TasksStateStateSetError),
    TasksErrorSet(#[from] TasksErrorStateSetError),
    TaskAborted(#[from] TaskAbortedError),
    TaskLoggerCreation(#[from] TaskLoggerCreationError),
    Ffi(#[from] FfiPanicError),
}

impl Engine {
    pub fn new(logger: Logger) -> Result<Self, EngineBuilderCreationError> {
        let mut lua = Lua::new_with(
            StdLib::TABLE | StdLib::STRING | StdLib::PACKAGE | StdLib::BIT | StdLib::MATH,
            LuaOptions::new().catch_rust_panics(false),
        )?;

        let target_systems_memory = Arc::new(Mutex::new(TargetSystemsMemory::default()));
        let target_groups_memory = Arc::new(Mutex::new(TargetGroupsMemory::default()));
        #[allow(clippy::arc_with_non_send_sync)]
        let tasks_memory = Arc::new(Mutex::new(TasksMemory::default()));

        let progress = ProgressContext::new(logger.clone());

        Modules::new(
            target_systems_memory.clone(),
            target_groups_memory.clone(),
            tasks_memory.clone(),
            progress.clone(),
        )
        .mount_to_globals(&mut lua)?;

        Ok(Self {
            lua,
            state: State::new(target_systems_memory, target_groups_memory, tasks_memory),
            logger,
            progress,
        })
    }

    pub fn state(&self) -> &State {
        &self.state
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

    pub fn validate_and_filter_by_selection(
        &self,
        tags_selection: &TagSelection,
        groups_selection: &GroupSelection,
        systems_selection: &SystemSelection,
        no_reqs: bool,
    ) -> Result<IndexMap<TargetSystem, Vec<Task>>, ValidationError> {
        let all_groups = self.state.all_groups()?;
        let all_systems = self.state.all_systems()?;
        let all_tasks = self.state.all_tasks()?;

        validate_group_system_names(&all_groups, &all_systems)?;
        validate_group_members(&all_groups, &all_systems)?;
        validate_task_targets(&all_tasks, &all_groups, &all_systems)?;
        validate_task_requires(&all_tasks)?;
        validate_selected_groups(&all_groups, groups_selection)?;
        validate_selected_systems(&all_systems, systems_selection)?;
        validate_selected_tags(&all_tasks, tags_selection)?;

        let selected_groups = select_groups(all_groups.clone(), groups_selection);
        let filtered_systems = select_systems(all_systems, &selected_groups, systems_selection);
        let filtered_tasks = if no_reqs {
            select_tasks(all_tasks, groups_selection, tags_selection)
        } else {
            select_tasks_with_requires(all_tasks, groups_selection, tags_selection)
        };

        let mut result = IndexMap::new();

        for (system_name, system_config) in filtered_systems {
            let system_groups = select_groups_for_system(&all_groups, &system_name);
            let system_tasks: Vec<Task> =
                select_tasks_for_system(&filtered_tasks, &system_name, &system_groups)
                    .into_iter()
                    .cloned()
                    .collect();

            result.insert(system_config, system_tasks);
        }

        Ok(result)
    }

    // TODO: do not propagate immediately, summarize instead
    fn run_tasks_on_system(
        &self,
        system: System,
        tasks: Vec<Task>,
        system_logger: &SystemLogger,
    ) -> Result<(), TaskExecutionError> {
        let mut skip_system = false;

        for task_config in tasks {
            let task_logger = system_logger.task(&task_config.name)?;

            if skip_system && !task_config.important {
                self.state
                    .set_task_state(&task_config.name, TaskState::Skipped)?;
                task_logger.skip();
                continue;
            }

            if let Some(when_handler) = &task_config.when {
                let should_run: bool = when_handler.call(())?;
                if !should_run {
                    self.state
                        .set_task_state(&task_config.name, TaskState::Skipped)?;
                    task_logger.skip();
                    continue;
                }
            }

            task_logger.start();

            self.progress.activate(task_logger.clone());

            let panic_result = catch_unwind(AssertUnwindSafe(|| {
                task_config.handler.call::<mlua::Value>(system.clone())
            }));

            let handler_result = match panic_result {
                Ok(handler_result) => handler_result,
                Err(panic_payload) => {
                    self.progress.deactivate();
                    return Err(panic_payload
                        .downcast::<FfiPanicError>()
                        .map(|error| TaskExecutionError::Ffi(*error))
                        .unwrap_or(TaskExecutionError::Lua(mlua::Error::RuntimeError(
                            "Unknown panic in task handler".to_string(),
                        ))));
                }
            };

            self.progress.deactivate();

            match handler_result {
                Ok(result) => {
                    self.state.set_task_result(&task_config.name, result)?;
                    self.state
                        .set_task_state(&task_config.name, TaskState::Success)?;

                    task_logger.finish(TaskState::Success);
                }
                Err(error) => {
                    let error_message = error.to_string();

                    task_logger.log(
                        LogLevel::Error,
                        &format!("Task '{}' failed: {}", task_config.name, error_message),
                    );

                    self.state
                        .set_task_state(&task_config.name, TaskState::Failed)?;
                    self.state
                        .set_task_error(&task_config.name, error_message.clone())?;

                    task_logger.finish(TaskState::Failed);

                    match task_config.on_fail {
                        OnFailBehavior::Continue => {}
                        OnFailBehavior::SkipSystem => {
                            skip_system = true;
                        }
                        OnFailBehavior::Abort => {
                            return Err(TaskAbortedError {
                                task: task_config.name.clone(),
                                error: error_message,
                            }
                            .into());
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn execute(
        &self,
        tags_selection: TagSelection,
        groups_selection: GroupSelection,
        systems_selection: SystemSelection,
        no_reqs: bool,
    ) -> Result<(), EngineExecutionError> {
        self.execute_entrypoint()?;

        let system_tasks = self.validate_and_filter_by_selection(
            &tags_selection,
            &groups_selection,
            &systems_selection,
            no_reqs,
        )?;

        for (system, tasks) in system_tasks {
            let system_logger = self.logger.system(&system.name)?;

            if tasks.is_empty() {
                // TODO: use system logger
                self.logger.info("No tasks to execute.");
                continue;
            }

            self.state.reset_execution_state()?;

            let system = System {
                name: system.name.clone(),
                kind: match &system.kind {
                    TargetSystemKind::Remote(remote_target_system) => {
                        SystemKind::Remote(objects::system::RemoteSystem {
                            address: remote_target_system.address,
                            port: remote_target_system.port,
                            user: remote_target_system.user.clone(),
                            executor: Executor::new_for_system(&system, self.progress.clone())?,
                            file_system_operator: FileSystemOperator::new_for_system(
                                &system,
                                self.progress.clone(),
                            )?,
                        })
                    }
                    TargetSystemKind::Local => SystemKind::Local(
                        Executor::new_local(self.progress.clone()),
                        FileSystemOperator::new_local(self.progress.clone()),
                    ),
                },
            };

            // TODO: no immediate propagation, end system for summary instead
            self.run_tasks_on_system(system, tasks, &system_logger)?;

            system_logger.finish();
        }

        Ok(())
    }
}

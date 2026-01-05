use std::{collections::HashSet, path::PathBuf, str::FromStr};

use mlua::{FromLua, IntoLua, Lua, MetaMethod, UserData};

use crate::{
    engine::readonly::set_readonly,
    error::{ErrorReport, MutexLockError},
    logger::SharedLogger,
    memory::{
        SharedMemory,
        target_groups::TargetGroupsMemory,
        tasks::{OnFailBehavior, Task, TaskAdditionError, TaskRetrievalError, TasksMemory},
    },
};

#[derive(Debug, Clone, PartialEq)]
pub struct TaskConfig {
    pub handler: mlua::Function,
    pub when: Option<mlua::Function>,
    pub on_fail: OnFailBehavior,
    pub tags: HashSet<String>,
    pub groups: HashSet<String>,
    pub requires: HashSet<String>,
    pub important: bool,
}

impl FromLua for TaskConfig {
    fn from_lua(value: mlua::Value, _: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::Table(table) => {
                let handler_field = table
                    .get::<mlua::Value>("handler")
                    .or(Err(mlua::Error::runtime("\"handler\" is missing")))?;
                let handler = if let mlua::Value::Function(handler_func) = handler_field {
                    handler_func
                } else {
                    Err(mlua::Error::runtime("\"handler\" is invalid"))?
                };

                let when: Option<mlua::Function> = table
                    .get("when")
                    .or(Err(mlua::Error::runtime("\"when\" is invalid")))?;

                let on_fail_str: Option<String> = table
                    .get("on_fail")
                    .or(Err(mlua::Error::runtime("\"on_fail\" is invalid")))?;
                let on_fail = match on_fail_str {
                    Some(s) => OnFailBehavior::from_str(&s).or(Err(mlua::Error::runtime(format!(
                        "Invalid on_fail value: \"{}\". Expected \"continue\", \"skip_system\", or \"abort\"",
                        s
                    ))))?,
                    None => OnFailBehavior::default(),
                };

                let tags: HashSet<String> = table
                    .get::<Option<Vec<String>>>("tags")
                    .or(Err(mlua::Error::runtime("\"tags\" is invalid")))?
                    .unwrap_or_default()
                    .into_iter()
                    .collect();
                let groups: HashSet<String> = table
                    .get::<Option<Vec<String>>>("groups")
                    .or(Err(mlua::Error::runtime("\"groups\" is invalid")))?
                    .unwrap_or_default()
                    .into_iter()
                    .collect();
                let requires: HashSet<String> = table
                    .get::<Option<Vec<String>>>("requires")
                    .or(Err(mlua::Error::runtime("\"requires\" is invalid")))?
                    .unwrap_or_default()
                    .into_iter()
                    .collect();
                let important: bool = table
                    .get::<Option<bool>>("important")
                    .or(Err(mlua::Error::runtime("\"important\" is invalid")))?
                    .unwrap_or(false);

                Ok(TaskConfig {
                    handler,
                    when,
                    on_fail,
                    tags,
                    groups,
                    requires,
                    important,
                })
            }
            mlua::Value::Function(_)
            | mlua::Value::Nil
            | mlua::Value::Boolean(_)
            | mlua::Value::LightUserData(_)
            | mlua::Value::Integer(_)
            | mlua::Value::Number(_)
            | mlua::Value::String(_)
            | mlua::Value::Thread(_)
            | mlua::Value::UserData(_)
            | mlua::Value::Error(_)
            | mlua::Value::Other(_) => Err(mlua::Error::runtime(format!(
                "{:?} is not a valid task config",
                value.type_name()
            ))),
        }
    }
}

impl IntoLua for Task {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        let task_table = lua.create_table()?;

        task_table.set("name", self.name)?;
        task_table.set("tags", self.tags.into_iter().collect::<Vec<_>>())?;
        task_table.set("requires", self.requires.into_iter().collect::<Vec<_>>())?;
        task_table.set("important", self.important)?;
        task_table.set("result", self.result)?;
        task_table.set("handler", self.handler)?;

        task_table.set("on_fail", self.on_fail.to_string())?;
        task_table.set("state", self.state.map(|state| state.to_string()))?;
        task_table.set("error", self.error)?;

        let task_table = set_readonly(lua, task_table)
            .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))?;

        Ok(mlua::Value::Table(task_table))
    }
}

#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub enum TaskConfigAdditionError {
    Lock(#[from] MutexLockError),
    TaskAddition(#[from] TaskAdditionError),
    GroupFilterNotDefined(#[from] GroupFilterNotDefinedError),
    Lua(#[from] mlua::Error),
}

#[derive(Debug, thiserror::Error)]
#[error("Group filter {1:?} of task {0:?} is not defined")]
pub struct GroupFilterNotDefinedError(String, pub Vec<String>);

#[derive(Debug, thiserror::Error)]
#[error("Failed to retrieve tasks configuration")]
pub enum TasksModuleRetrievalError {
    Lock(#[from] MutexLockError),
    TaskRetrieval(#[from] TaskRetrievalError),
}

pub struct TasksTable {
    pub groups_memory: SharedMemory<TargetGroupsMemory>,
    pub tasks_memory: SharedMemory<TasksMemory>,
    pub logger: SharedLogger,
}

impl TasksTable {
    pub fn new(
        groups_memory: SharedMemory<TargetGroupsMemory>,
        tasks_memory: SharedMemory<TasksMemory>,
        logger: SharedLogger,
    ) -> Self {
        Self {
            groups_memory,
            tasks_memory,
            logger,
        }
    }

    pub fn add(
        &self,
        lua: &Lua,
        name: String,
        config: TaskConfig,
    ) -> Result<(), TaskConfigAdditionError> {
        let mut tasks = self.tasks_memory.lock().map_err(|_| MutexLockError)?;
        let groups = self.groups_memory.lock().map_err(|_| MutexLockError)?.all();

        {
            let undefined_groups: Vec<String> = config
                .groups
                .iter()
                .filter(|name| !groups.contains_key(*name))
                .cloned()
                .collect();

            if !undefined_groups.is_empty() {
                Err(GroupFilterNotDefinedError(name.clone(), undefined_groups))?
            }
        }

        let wrapped_handler = {
            let logger = self.logger.clone();
            let task_name = name.clone();
            let handler = config.handler.clone();

            lua.create_function(move |_, value: mlua::Value| {
                let mut guard = logger.lock().unwrap();
                guard.enter_task(&task_name);
                drop(guard);

                let result = handler.clone().call::<mlua::Value>(value);

                let mut guard = logger.lock().unwrap();
                guard.pop_task();

                result
            })?
        };

        tasks.add(Task {
            name,
            handler: wrapped_handler,
            when: config.when,
            on_fail: config.on_fail,
            tags: config.tags,
            groups: config.groups,
            requires: config.requires,
            important: config.important,
            result: None,
            state: None,
            error: None,
        })?;

        Ok(())
    }

    fn get(&self, name: String) -> Result<Task, TasksModuleRetrievalError> {
        let guard = self.tasks_memory.lock().map_err(|_| MutexLockError)?;

        Ok(guard.get(&name)?)
    }
}

impl UserData for TasksTable {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_meta_method(
            MetaMethod::NewIndex,
            |lua, this, (name, mut config): (String, TaskConfig)| {
                let additional_tags = lua.inspect_stack(1, |debug| {
                    let source = debug.source().source?;

                    if !source.starts_with('@') {
                        return None;
                    }

                    let source_path = PathBuf::from(source.trim_start_matches('@'));

                    let source_file_stem = source_path.file_stem()?.to_string_lossy().to_string();
                    let initial_additional_tags = vec![source_file_stem];

                    let additional_tags = if let Some(parent_path) = source_path.parent() {
                        parent_path.components().fold(
                            initial_additional_tags,
                            |mut acc, component| {
                                match component {
                                    std::path::Component::Prefix(..)
                                    | std::path::Component::RootDir
                                    | std::path::Component::CurDir
                                    | std::path::Component::ParentDir => {}
                                    std::path::Component::Normal(component) => {
                                        acc.push(component.to_string_lossy().to_string());
                                    }
                                };

                                acc
                            },
                        )
                    } else {
                        initial_additional_tags
                    };

                    Some(additional_tags)
                });

                if let Some(Some(additional_tags)) = additional_tags {
                    config.tags.extend(additional_tags);
                }

                this.add(lua, name, config)
                    .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
            },
        );

        methods.add_meta_method(MetaMethod::Index, |_, this, (name,): (String,)| {
            this.get(name)
                .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))
        });
    }
}

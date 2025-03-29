use std::path::PathBuf;

use crate::{
    logger::SharedLogger,
    memory::{
        target_groups::TargetGroupsMemory, target_systems::TargetSystemsMemory, tasks::TasksMemory,
        SharedMemory,
    },
};

mod env;
mod file_system;
mod format;
mod log;
mod targets;
mod tasks;
mod templates;

pub struct Modules {
    templates: templates::Templates,
    format: format::Format,
    targets: targets::TargetsTable,
    tasks: tasks::TasksTable,
    file_system: file_system::FileSystem,
    log: log::Log,
    env: env::Env,
}

impl Modules {
    pub fn new(
        target_systems: SharedMemory<TargetSystemsMemory>,
        target_groups: SharedMemory<TargetGroupsMemory>,
        tasks: SharedMemory<TasksMemory>,
        logger: SharedLogger,
        root_directory: PathBuf,
    ) -> Self {
        let file_system = file_system::FileSystem::new(root_directory);
        let format = format::Format;
        let targets = targets::TargetsTable::new(target_groups.clone(), target_systems.clone());
        let tasks = tasks::TasksTable::new(target_groups, tasks, logger);
        let templates = templates::Templates::new();
        let log = log::Log;
        let env = env::Env;

        Self {
            file_system,
            format,
            targets,
            tasks,
            templates,
            log,
            env,
        }
    }
}

impl MountToGlobals for Modules {
    fn mount_to_globals(self, lua: &mut mlua::Lua) -> Result<(), mlua::Error> {
        let globals = lua.globals();

        globals.set("fs", self.file_system)?;
        globals.set("format", self.format)?;
        globals.set("targets", self.targets)?;
        globals.set("tasks", self.tasks)?;
        globals.set("template", self.templates)?;
        globals.set("env", self.env)?;

        self.log.mount_to_globals(lua)?;

        Ok(())
    }
}

pub trait MountToGlobals {
    fn mount_to_globals(self, lua: &mut mlua::Lua) -> Result<(), mlua::Error>;
}

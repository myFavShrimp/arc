use crate::memory::{
    SharedMemory, target_groups::TargetGroupsMemory, target_systems::TargetSystemsMemory,
    tasks::TasksMemory,
};

mod env;
mod format;
mod host;
mod log;
mod targets;
mod tasks;
mod templates;

pub use log::{LogModuleLogger, SharedLogger};

pub struct Modules {
    templates: templates::Templates,
    format: format::Format,
    targets: targets::TargetsTable,
    tasks: tasks::TasksTable,
    log: log::Log,
    env: env::Env,
    host: host::Host,
}

impl Modules {
    pub fn new(
        target_systems: SharedMemory<TargetSystemsMemory>,
        target_groups: SharedMemory<TargetGroupsMemory>,
        tasks: SharedMemory<TasksMemory>,
        shared_logger: SharedLogger,
    ) -> Self {
        let format = format::Format;
        let targets = targets::TargetsTable::new(target_groups, target_systems.clone());
        let tasks = tasks::TasksTable::new(tasks);
        let templates = templates::Templates::new();
        let env = env::Env;
        let host = host::Host::new();
        let log = log::Log::new(shared_logger);

        Self {
            format,
            targets,
            tasks,
            templates,
            log,
            env,
            host,
        }
    }
}

impl MountToGlobals for Modules {
    fn mount_to_globals(self, lua: &mut mlua::Lua) -> Result<(), mlua::Error> {
        let globals = lua.globals();

        globals.set("format", self.format)?;
        globals.set("targets", self.targets)?;
        globals.set("tasks", self.tasks)?;
        globals.set("template", self.templates)?;
        globals.set("env", self.env)?;
        globals.set("host", self.host)?;

        self.log.mount_to_globals(lua)?;

        Ok(())
    }
}

pub trait MountToGlobals {
    fn mount_to_globals(self, lua: &mut mlua::Lua) -> Result<(), mlua::Error>;
}

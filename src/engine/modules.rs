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
mod template;

pub use log::{LogModuleLogger, SharedLogger};

pub struct Modules {
    template: template::Template,
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
        let template = template::Template::new();
        let env = env::Env;
        let host = host::Host::new();
        let log = log::Log::new(shared_logger);

        Self {
            format,
            targets,
            tasks,
            template,
            log,
            env,
            host,
        }
    }
}

impl MountToGlobals for Modules {
    fn mount_to_globals(self, lua: &mut mlua::Lua) -> Result<(), mlua::Error> {
        self.format.mount_to_globals(lua)?;
        self.targets.mount_to_globals(lua)?;
        self.tasks.mount_to_globals(lua)?;
        self.env.mount_to_globals(lua)?;
        self.template.mount_to_globals(lua)?;
        self.log.mount_to_globals(lua)?;

        self.host.mount_to_globals(lua)?;

        Ok(())
    }
}

pub trait MountToGlobals {
    fn mount_to_globals(self, lua: &mut mlua::Lua) -> Result<(), mlua::Error>;
}

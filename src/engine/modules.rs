use std::path::PathBuf;

use crate::{
    memory::{SharedMemory, groups::GroupsMemory, systems::SystemsMemory, tasks::TasksMemory},
    progress::ProgressContext,
};

mod arc;
mod env;
mod format;
mod group;
mod host;
mod log;
mod system;
mod task;
mod template;

pub struct Modules {
    template: template::Template,
    format: format::Format,
    system: system::SystemsTable,
    group: group::GroupsTable,
    task: task::TaskTable,
    log: log::Log,
    env: env::Env,
    host: host::Host,
    arc: arc::Arc,
}

impl Modules {
    pub fn new(
        systems: SharedMemory<SystemsMemory>,
        groups: SharedMemory<GroupsMemory>,
        tasks: SharedMemory<TasksMemory>,
        progress: ProgressContext,
        root_path: PathBuf,
        home_path: PathBuf,
    ) -> Self {
        let format = format::Format;
        let system = system::SystemsTable::new(systems);
        let group = group::GroupsTable::new(groups);
        let task = task::TaskTable::new(tasks);
        let template = template::Template::new();
        let env = env::Env;
        let host = host::Host::new(progress.clone());
        let log = log::Log::new(progress);
        let arc = arc::Arc::new(root_path, home_path);

        Self {
            format,
            system,
            group,
            task,
            template,
            log,
            env,
            host,
            arc,
        }
    }
}

impl MountToGlobals for Modules {
    fn mount_to_globals(self, lua: &mut mlua::Lua) -> Result<(), mlua::Error> {
        self.format.mount_to_globals(lua)?;
        self.system.mount_to_globals(lua)?;
        self.group.mount_to_globals(lua)?;
        self.task.mount_to_globals(lua)?;
        self.env.mount_to_globals(lua)?;
        self.template.mount_to_globals(lua)?;
        self.log.mount_to_globals(lua)?;

        self.host.mount_to_globals(lua)?;
        self.arc.mount_to_globals(lua)?;

        Ok(())
    }
}

pub trait MountToGlobals {
    fn mount_to_globals(self, lua: &mut mlua::Lua) -> Result<(), mlua::Error>;
}

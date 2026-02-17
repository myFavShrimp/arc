use std::path::PathBuf;

use mlua::UserData;

use crate::engine::modules::MountToGlobals;

pub struct Arc {
    root_path: PathBuf,
    home_path: PathBuf,
}

impl Arc {
    pub fn new(root_path: PathBuf, home_path: PathBuf) -> Self {
        Self {
            root_path,
            home_path,
        }
    }
}

impl UserData for Arc {
    fn add_fields<F: mlua::UserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("project_root_path", |_, this| {
            Ok(this.root_path.to_string_lossy().to_string())
        });
        fields.add_field_method_get("home_path", |_, this| {
            Ok(this.home_path.to_string_lossy().to_string())
        });
    }
}

impl MountToGlobals for Arc {
    fn mount_to_globals(self, lua: &mut mlua::Lua) -> Result<(), mlua::Error> {
        let globals = lua.globals();
        globals.set("arc", self)?;

        Ok(())
    }
}

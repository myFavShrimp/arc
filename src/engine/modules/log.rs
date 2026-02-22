use mlua::UserData;

use crate::logger::LogLevel;
use crate::progress::ProgressContext;

use super::MountToGlobals;

#[derive(Clone)]
pub struct Log {
    progress: ProgressContext,
}

impl Log {
    pub fn new(progress: ProgressContext) -> Self {
        Self { progress }
    }

    fn log(&self, level: LogLevel, msg: &str) {
        self.progress.log(level, msg);
    }
}

fn lua_value_to_string(lua: &mlua::Lua, value: mlua::Value) -> Result<String, mlua::Error> {
    let lua_to_string: mlua::Function = lua.globals().get("tostring")?;
    let lua_string: mlua::String = lua_to_string.call(value)?;

    match lua_string.to_str() {
        Ok(utf8_str) => Ok(utf8_str.to_string()),
        Err(_) => Ok(format!("[binary data] {:X?}", lua_string.as_bytes())),
    }
}

impl UserData for Log {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_function("debug", |lua, value: mlua::Value| {
            let log = lua
                .app_data_ref::<Self>()
                .expect("logger unavailable in app data");
            log.log(LogLevel::Debug, &lua_value_to_string(lua, value)?);

            Ok(())
        });

        methods.add_function("info", |lua, value: mlua::Value| {
            let log = lua
                .app_data_ref::<Self>()
                .expect("logger unavailable in app data");
            log.log(LogLevel::Info, &lua_value_to_string(lua, value)?);

            Ok(())
        });

        methods.add_function("warn", |lua, value: mlua::Value| {
            let log = lua
                .app_data_ref::<Self>()
                .expect("logger unavailable in app data");
            log.log(LogLevel::Warn, &lua_value_to_string(lua, value)?);

            Ok(())
        });

        methods.add_function("error", |lua, value: mlua::Value| {
            let log = lua
                .app_data_ref::<Self>()
                .expect("logger unavailable in app data");
            log.log(LogLevel::Error, &lua_value_to_string(lua, value)?);

            Ok(())
        });
    }
}

impl MountToGlobals for Log {
    fn mount_to_globals(self, lua: &mut mlua::Lua) -> Result<(), mlua::Error> {
        lua.set_app_data(self.clone());

        let globals = lua.globals();
        globals.set("log", self)?;

        globals.set(
            "print",
            lua.create_function(|lua, value: mlua::Value| {
                let logger = lua
                    .app_data_ref::<Self>()
                    .expect("logger unavailable in app data");

                logger.log(LogLevel::Info, &lua_value_to_string(lua, value)?);

                Ok(())
            })?,
        )?;

        Ok(())
    }
}

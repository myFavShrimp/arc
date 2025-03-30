use colored::Colorize;
use mlua::UserData;

use super::MountToGlobals;

pub struct Log;

impl Log {
    fn info(value: mlua::Value) {
        let message = match value.to_string() {
            Ok(utf8_str) => utf8_str,
            Err(_) => {
                format!("[binary data] {:X?}", value)
            }
        };

        println!(
            "{:.3} {}{}: {}",
            jiff::Timestamp::now(),
            "INFO".blue(),
            "".clear(),
            message,
        );
    }
}

impl UserData for Log {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_function("info", |_, value: mlua::Value| {
            Log::info(value);

            Ok(())
        });
    }
}

impl MountToGlobals for Log {
    fn mount_to_globals(self, lua: &mut mlua::Lua) -> Result<(), mlua::Error> {
        let globals = lua.globals();

        globals.set(
            "print",
            lua.create_function(|_, value: mlua::Value| {
                Log::info(value);

                Ok(())
            })?,
        )?;

        Ok(())
    }
}

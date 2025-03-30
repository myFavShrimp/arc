use colored::Colorize;
use mlua::UserData;

use super::MountToGlobals;

#[derive(Clone)]
pub struct Log;

impl Log {
    fn debug(message: &str) {
        println!(
            "{:.3} {}{}: {}",
            jiff::Timestamp::now(),
            "DEBUG".green(),
            "".clear(),
            message,
        );
    }

    fn info(message: &str) {
        println!(
            "{:.3} {}{}: {}",
            jiff::Timestamp::now(),
            "INFO".blue(),
            "".clear(),
            message,
        );
    }

    fn warn(message: &str) {
        println!(
            "{:.3} {}{}: {}",
            jiff::Timestamp::now(),
            "WARN".yellow(),
            "".clear(),
            message,
        );
    }

    fn error(message: &str) {
        println!(
            "{:.3} {}{}: {}",
            jiff::Timestamp::now(),
            "ERROR".red(),
            "".clear(),
            message,
        );
    }
}

fn lua_value_to_string(value: mlua::Value) -> String {
    match value.to_string() {
        Ok(utf8_str) => utf8_str,
        Err(_) => {
            format!("[binary data] {:X?}", value)
        }
    }
}

impl UserData for Log {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_function("debug", |_, value: mlua::Value| {
            Log::debug(&lua_value_to_string(value));

            Ok(())
        });
        methods.add_function("info", |_, value: mlua::Value| {
            Log::info(&lua_value_to_string(value));

            Ok(())
        });
        methods.add_function("warn", |_, value: mlua::Value| {
            Log::warn(&lua_value_to_string(value));

            Ok(())
        });
        methods.add_function("error", |_, value: mlua::Value| {
            Log::error(&lua_value_to_string(value));

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
                Log::info(&lua_value_to_string(value));

                Ok(())
            })?,
        )?;

        Ok(())
    }
}

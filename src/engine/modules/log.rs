use mlua::UserData;

use crate::error::ErrorReport;

use super::MountToGlobals;

pub struct Log;

impl Log {
    fn info(message: &str) {
        println!("{:.3} INFO: {}", jiff::Timestamp::now(), message);
    }
}

impl UserData for Log {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_function("to_json", |_, value: mlua::Value| {
            Self::info(
                &value
                    .to_string()
                    .map_err(|e| mlua::Error::RuntimeError(ErrorReport::boxed_from(e).report()))?,
            );

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
                Log::info(&value.to_string()?);

                Ok(())
            })?,
        )?;

        Ok(())
    }
}

use std::sync::{Arc, Mutex};

use crate::logger::{LogLevel, Logger};
use crate::progress::TaskLogger;

use super::MountToGlobals;

pub enum LogModuleLogger {
    Global(Logger),
    Task(TaskLogger),
}

#[derive(Clone)]
pub struct SharedLogger {
    inner: Arc<Mutex<LogModuleLogger>>,
}

impl SharedLogger {
    pub fn new(logger: Logger) -> Self {
        Self {
            inner: Arc::new(Mutex::new(LogModuleLogger::Global(logger))),
        }
    }

    #[must_use]
    pub fn change_logger(&self, logger: LogModuleLogger) -> LogModuleLogger {
        std::mem::replace(&mut *self.inner.lock().unwrap(), logger)
    }

    fn log(&self, level: LogLevel, msg: &str) {
        match &*self.inner.lock().unwrap() {
            LogModuleLogger::Global(logger) => logger.lua_log(level, msg),
            LogModuleLogger::Task(task_logger) => task_logger.log(level, msg),
        }
    }
}

pub struct Log {
    logger: SharedLogger,
}

impl Log {
    pub fn new(logger: SharedLogger) -> Self {
        Self { logger }
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

impl MountToGlobals for Log {
    fn mount_to_globals(self, lua: &mut mlua::Lua) -> Result<(), mlua::Error> {
        lua.set_app_data(self.logger);

        let globals = lua.globals();

        let log_table = lua.create_table()?;

        log_table.set(
            "debug",
            lua.create_function(|lua, value: mlua::Value| {
                if let Some(logger) = lua.app_data_ref::<SharedLogger>() {
                    logger.log(LogLevel::Debug, &lua_value_to_string(value));
                }
                Ok(())
            })?,
        )?;

        log_table.set(
            "info",
            lua.create_function(|lua, value: mlua::Value| {
                if let Some(logger) = lua.app_data_ref::<SharedLogger>() {
                    logger.log(LogLevel::Info, &lua_value_to_string(value));
                }
                Ok(())
            })?,
        )?;

        log_table.set(
            "warn",
            lua.create_function(|lua, value: mlua::Value| {
                if let Some(logger) = lua.app_data_ref::<SharedLogger>() {
                    logger.log(LogLevel::Warn, &lua_value_to_string(value));
                }
                Ok(())
            })?,
        )?;

        log_table.set(
            "error",
            lua.create_function(|lua, value: mlua::Value| {
                if let Some(logger) = lua.app_data_ref::<SharedLogger>() {
                    logger.log(LogLevel::Error, &lua_value_to_string(value));
                }
                Ok(())
            })?,
        )?;

        globals.set("log", log_table)?;

        globals.set(
            "print",
            lua.create_function(|lua, value: mlua::Value| {
                if let Some(logger) = lua.app_data_ref::<SharedLogger>() {
                    logger.log(LogLevel::Info, &lua_value_to_string(value));
                }
                Ok(())
            })?,
        )?;

        Ok(())
    }
}

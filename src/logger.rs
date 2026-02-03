use colored::Colorize;

use crate::progress::{SystemLogger, SystemLoggerCreationError};

#[derive(Clone)]
pub struct Logger;

impl Logger {
    pub fn new() -> Self {
        Self
    }

    pub fn info(&self, message: &str) {
        println!("ARC | {}{} : {}", "INFO".blue(), "".clear(), message);
    }

    pub fn warn(&self, message: &str) {
        println!("ARC | {}{} : {}", "WARN".yellow(), "".clear(), message);
    }
    pub fn lua_log(&self, level: LogLevel, message: &str) {
        let level_colored = match level {
            LogLevel::Debug => "DEBG".green(),
            LogLevel::Info => "INFO".blue(),
            LogLevel::Warn => "WARN".yellow(),
            LogLevel::Error => "ERRO".red(),
        };

        println!(
            " {}  {}{} {}",
            level_colored,
            format!("{:.3}", jiff::Timestamp::now()).bright_black(),
            ":".bright_black(),
            message.bright_black(),
        );
    }

    pub fn system(&self, name: &str) -> Result<SystemLogger, SystemLoggerCreationError> {
        SystemLogger::new(name)
    }
}

impl Default for Logger {
    fn default() -> Self {
        Self::new()
    }
}

pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

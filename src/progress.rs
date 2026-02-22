mod command;
mod system;
mod task;
mod transfer;

use std::{cell::RefCell, rc::Rc};

use crate::logger::{LogLevel, Logger};

pub use command::{CommandProgress, CommandProgressCreationError};
pub use system::{SystemLogger, SystemLoggerCreationError, TaskLoggerCreationError};
pub use task::TaskLogger;
pub use transfer::{
    ProgressWriter, TransferDirection, TransferProgress, TransferProgressCreationError,
};

static SYSTEM_TICK_CHARS: &str = r"|/-\ ";
static TASK_TICK_STRINGS: &[&str] = &["●   ", " ●  ", "  ● ", "   ●", "  ● ", " ●  ", "    "];
static TICK_DURATION_MS_TASK: u64 = 120;
static TICK_DURATION_MS_SYSTEM: u64 = TICK_DURATION_MS_TASK * 2;

#[derive(Clone)]
pub struct ProgressContext {
    active_task: Rc<RefCell<Option<TaskLogger>>>,
    logger: Logger,
}

impl ProgressContext {
    pub fn new(logger: Logger) -> Self {
        Self {
            active_task: Rc::new(RefCell::new(None)),
            logger,
        }
    }

    pub fn activate(&self, task_logger: TaskLogger) {
        *self.active_task.borrow_mut() = Some(task_logger);
    }

    pub fn deactivate(&self) {
        *self.active_task.borrow_mut() = None;
    }

    pub fn command(&self, command: &str) -> Result<CommandProgress, CommandProgressCreationError> {
        match &*self.active_task.borrow() {
            Some(task_logger) => task_logger.command_progress(command),
            None => Ok(CommandProgress::noop()),
        }
    }

    pub fn transfer(
        &self,
        direction: TransferDirection,
        total: u64,
    ) -> Result<TransferProgress, TransferProgressCreationError> {
        match &*self.active_task.borrow() {
            Some(task_logger) => task_logger.transfer_progress(direction, total),
            None => Ok(TransferProgress::noop()),
        }
    }

    pub fn log(&self, level: LogLevel, msg: &str) {
        match &*self.active_task.borrow() {
            Some(task_logger) => task_logger.log(level, msg),
            None => self.logger.lua_log(level, msg),
        }
    }
}

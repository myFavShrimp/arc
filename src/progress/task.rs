use colored::Colorize;
use indicatif::{MultiProgress, ProgressBar};

use crate::{logger::LogLevel, memory::tasks::TaskState};

use super::{
    command::{CommandProgress, CommandProgressCreationError},
    system::TaskSummary,
    transfer::{TransferDirection, TransferProgress, TransferProgressCreationError},
};

#[derive(Clone)]
pub struct TaskLogger {
    multi_progress: MultiProgress,
    task_bar: ProgressBar,
    task_name: String,
    summary: TaskSummary,
}

impl TaskLogger {
    pub(super) fn new(
        multi_progress: MultiProgress,
        task_bar: ProgressBar,
        task_name: String,
        summary: TaskSummary,
    ) -> Self {
        Self {
            multi_progress,
            task_bar,
            task_name,
            summary,
        }
    }

    fn println(&self, msg: &str) {
        self.task_bar.suspend(|| {
            println!("{}", msg);
        });
    }

    pub fn start(&self) {
        self.println(&format!("[{}] {}", "STRT".bright_blue(), self.task_name));
    }

    pub fn skip(self) {
        self.summary.increment(TaskState::Skipped);
        self.println(&format!("[{}] {}\n", "SKIP".yellow(), self.task_name));
        self.task_bar.finish_and_clear();
    }

    pub fn log(&self, level: LogLevel, message: &str) {
        let level_colored = match level {
            LogLevel::Debug => "DEBG".green(),
            LogLevel::Info => "INFO".blue(),
            LogLevel::Warn => "WARN".yellow(),
            LogLevel::Error => "ERRO".red(),
        };

        self.println(&format!(
            " {}  {}{} {}",
            level_colored,
            format!("{:.3}", jiff::Timestamp::now()).bright_black(),
            ":".bright_black(),
            message.bright_black(),
        ));
    }

    pub(super) fn transfer_progress(
        &self,
        direction: TransferDirection,
        total: u64,
    ) -> Result<TransferProgress, TransferProgressCreationError> {
        TransferProgress::new(&self.multi_progress, direction, total)
    }

    pub(super) fn command_progress(
        &self,
        cmd: &str,
    ) -> Result<CommandProgress, CommandProgressCreationError> {
        CommandProgress::new(&self.multi_progress, cmd)
    }

    pub fn abort(self) {
        self.summary.increment(TaskState::Failed);
        self.println(&format!("[{}] {}\n", "ABRT".red(), self.task_name));
        self.task_bar.finish_and_clear();
    }

    pub fn finish(self, state: TaskState) {
        self.summary.increment(state);

        let status = match state {
            TaskState::Success => format!(" {} ", "OK".green()),
            TaskState::Failed => format!("{}", "FAIL".red()),
            TaskState::Skipped => format!("{}", "SKIP".yellow()),
        };

        self.println(&format!("[{}] {}\n", status, self.task_name));

        self.task_bar.finish_and_clear();
    }
}

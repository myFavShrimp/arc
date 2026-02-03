use colored::Colorize;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use crate::{logger::LogLevel, memory::tasks::TaskState};

#[derive(Debug, thiserror::Error)]
#[error("Failed to create system logger")]
pub struct SystemLoggerCreationError(#[from] indicatif::style::TemplateError);

#[derive(Debug, thiserror::Error)]
#[error("Failed to create task logger")]
pub struct TaskLoggerCreationError(#[from] indicatif::style::TemplateError);

static SYSTEM_TICK_CHARS: &str = r"|/-\ ";
static TASK_TICK_STRINGS: &[&str] = &["●   ", " ●  ", "  ● ", "   ●", "  ● ", " ●  ", "    "];
static TICK_DURATION_MS_TASK: u64 = 120;
static TICK_DURATION_MS_SYSTEM: u64 = TICK_DURATION_MS_TASK * 2;

pub struct SystemLogger {
    multi_progress: MultiProgress,
    system_bar: ProgressBar,
    system_name: String,
}

impl SystemLogger {
    pub fn new(system_name: &str) -> Result<Self, SystemLoggerCreationError> {
        let multi_progress = MultiProgress::new();
        let bar = multi_progress.add(ProgressBar::new_spinner());

        bar.set_style(
            ProgressStyle::default_spinner()
                .tick_chars(SYSTEM_TICK_CHARS)
                .template("\n{msg} {spinner:.cyan}")?,
        );

        bar.set_message(format!("SYSTEM: {}", system_name));
        bar.enable_steady_tick(std::time::Duration::from_millis(TICK_DURATION_MS_SYSTEM));

        let system_logger = Self {
            multi_progress,
            system_bar: bar,
            system_name: system_name.to_string(),
        };

        system_logger.println(&format!("\nSYSTEM: {}\n", system_name));

        Ok(system_logger)
    }

    pub fn println(&self, msg: &str) {
        self.system_bar.suspend(|| {
            println!("{}", msg);
        });
    }

    pub fn task(&self, task_name: &str) -> Result<TaskLogger, TaskLoggerCreationError> {
        let bar = self.multi_progress.insert(0, ProgressBar::new_spinner());

        bar.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(TASK_TICK_STRINGS)
                .template("[{spinner:.cyan}] {msg}")?,
        );

        bar.set_message(task_name.to_string());
        bar.enable_steady_tick(std::time::Duration::from_millis(TICK_DURATION_MS_TASK));

        Ok(TaskLogger {
            task_bar: bar,
            task_name: task_name.to_string(),
        })
    }

    pub fn finish(self, success: bool) {
        let status = if success {
            "ok".green()
        } else {
            "failed".red()
        };

        self.println(&format!("SYSTEM : {} | {}\n", self.system_name, status));

        self.system_bar.finish_and_clear();
    }
}

#[derive(Clone)]
pub struct TaskLogger {
    task_bar: ProgressBar,
    task_name: String,
}

impl TaskLogger {
    pub fn println(&self, msg: &str) {
        self.task_bar.suspend(|| {
            println!("{}", msg);
        });
    }

    pub fn start(&self) {
        self.println(&format!("[{}] {}", "STRT".bright_blue(), self.task_name));
    }

    pub fn skip(self) {
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

    pub fn finish(self, state: TaskState) {
        let status = match state {
            TaskState::Success => format!(" {} ", "OK".green()),
            TaskState::Failed => format!("{}", "FAIL".red()),
            TaskState::Skipped => format!("{}", "SKIP".yellow()),
        };

        self.println(&format!("[{}] {}\n", status, self.task_name));

        self.task_bar.finish_and_clear();
    }
}

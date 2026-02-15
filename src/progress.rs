use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use colored::Colorize;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use crate::{
    logger::{LogLevel, Logger},
    memory::tasks::TaskState,
};

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

    pub fn command(&self, cmd: &str) -> Result<CommandProgress, CommandProgressCreationError> {
        match &*self.active_task.borrow() {
            Some(task_logger) => task_logger.command_progress(cmd),
            None => Ok(CommandProgress::noop()),
        }
    }

    pub fn upload(
        &self,
        path: &str,
        total: u64,
    ) -> Result<TransferProgress, TransferProgressCreationError> {
        match &*self.active_task.borrow() {
            Some(task_logger) => {
                task_logger.transfer_progress(TransferDirection::Upload, path, total)
            }
            None => Ok(TransferProgress::noop()),
        }
    }

    pub fn download(
        &self,
        path: &str,
        total: u64,
    ) -> Result<TransferProgress, TransferProgressCreationError> {
        match &*self.active_task.borrow() {
            Some(task_logger) => {
                task_logger.transfer_progress(TransferDirection::Download, path, total)
            }
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

#[derive(Debug, thiserror::Error)]
#[error("Failed to create transfer progress")]
pub struct TransferProgressCreationError(#[from] indicatif::style::TemplateError);

#[derive(Debug, thiserror::Error)]
#[error("Failed to create command progress")]
pub struct CommandProgressCreationError(#[from] indicatif::style::TemplateError);

pub struct TransferProgress {
    bar: ProgressBar,
    header: String,
    active: bool,
}

impl TransferProgress {
    fn noop() -> Self {
        Self {
            bar: ProgressBar::hidden(),
            header: String::new(),
            active: false,
        }
    }

    pub fn update(&self, bytes: u64) {
        if self.active {
            self.bar.set_position(bytes);
        }
    }

    pub fn finish(&self) {
        if self.active {
            self.bar.println(&self.header);
            self.bar.finish_and_clear();
        }
    }
}

static MAX_OUTPUT_LINES: usize = 4;

pub struct CommandProgress {
    bar: ProgressBar,
    header: String,
    command: String,
    active: bool,
}

impl CommandProgress {
    fn noop() -> Self {
        Self {
            bar: ProgressBar::hidden(),
            header: String::new(),
            command: String::new(),
            active: false,
        }
    }

    pub fn update_output(&self, output: &str) {
        if !self.active {
            return;
        }

        let lines: Vec<&str> = output.lines().collect();
        let start = lines.len().saturating_sub(MAX_OUTPUT_LINES);
        let tail: String = lines[start..]
            .iter()
            .map(|line| format!("       {}", line.bright_black()))
            .collect::<Vec<_>>()
            .join("\n");

        if tail.is_empty() {
            self.bar
                .set_message(format!("{}", self.command.bright_black()));
        } else {
            self.bar
                .set_message(format!("{}\n{}", self.command.bright_black(), tail,));
        }
    }

    pub fn finish(&self) {
        if self.active {
            self.bar.println(&self.header);
            self.bar.finish_and_clear();
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TaskSummary {
    success: Arc<AtomicUsize>,
    failed: Arc<AtomicUsize>,
    skipped: Arc<AtomicUsize>,
}

impl TaskSummary {
    fn increment(&self, state: TaskState) {
        match state {
            TaskState::Success => self.success.fetch_add(1, Ordering::Relaxed),
            TaskState::Failed => self.failed.fetch_add(1, Ordering::Relaxed),
            TaskState::Skipped => self.skipped.fetch_add(1, Ordering::Relaxed),
        };
    }

    fn success(&self) -> usize {
        self.success.load(Ordering::Relaxed)
    }

    fn failed(&self) -> usize {
        self.failed.load(Ordering::Relaxed)
    }

    fn skipped(&self) -> usize {
        self.skipped.load(Ordering::Relaxed)
    }
}

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
    summary: TaskSummary,
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
            summary: TaskSummary::default(),
        };

        system_logger.println(&format!("\nSYSTEM: {}\n", system_name));

        Ok(system_logger)
    }

    fn println(&self, msg: &str) {
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
            multi_progress: self.multi_progress.clone(),
            task_bar: bar,
            task_name: task_name.to_string(),
            summary: self.summary.clone(),
        })
    }

    pub fn finish(self) {
        let ok_part = format!("{} OK", self.summary.success()).green();

        let failed_part = if self.summary.failed() > 0 {
            format!("{} FAILED", self.summary.failed()).red()
        } else {
            format!("{} FAILED", self.summary.failed()).normal()
        };

        let skipped_part = if self.summary.skipped() > 0 {
            format!("{} SKIPPED", self.summary.skipped()).yellow()
        } else {
            format!("{} SKIPPED", self.summary.skipped()).normal()
        };

        self.println(&format!(
            "SYSTEM : {} | {} | {} | {}\n",
            self.system_name, ok_part, failed_part, skipped_part
        ));

        self.system_bar.finish_and_clear();
    }
}

#[derive(Clone)]
pub struct TaskLogger {
    multi_progress: MultiProgress,
    task_bar: ProgressBar,
    task_name: String,
    summary: TaskSummary,
}

enum TransferDirection {
    Upload,
    Download,
}

impl TransferDirection {
    fn label(&self) -> &'static str {
        match self {
            TransferDirection::Upload => "UPLD",
            TransferDirection::Download => "DWLD",
        }
    }
}

impl TaskLogger {
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

    fn transfer_progress(
        &self,
        direction: TransferDirection,
        path: &str,
        total: u64,
    ) -> Result<TransferProgress, TransferProgressCreationError> {
        let bar = self.multi_progress.insert(0, ProgressBar::new(total));

        bar.set_style(
            ProgressStyle::default_bar()
                .template(" {prefix}  {msg}\n       [{bar:30.dim}] {bytes}/{total_bytes}")?
                .progress_chars("█░ "),
        );

        let header = format!(" {}  {}", direction.label().cyan(), path.bright_black());

        bar.set_prefix(format!("{}", direction.label().cyan()));
        bar.set_message(format!("{}", path.bright_black()));

        Ok(TransferProgress {
            bar,
            header,
            active: true,
        })
    }

    fn command_progress(&self, cmd: &str) -> Result<CommandProgress, CommandProgressCreationError> {
        let bar = self.multi_progress.insert(0, ProgressBar::new_spinner());

        bar.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(TASK_TICK_STRINGS)
                .template(" {prefix}  {msg}")?,
        );

        let header = format!(" {}  {}", "CMND".cyan(), cmd.bright_black());

        bar.set_prefix(format!("{}", "CMND".cyan()));
        bar.set_message(format!("{}", cmd.bright_black()));
        bar.enable_steady_tick(std::time::Duration::from_millis(TICK_DURATION_MS_TASK));

        Ok(CommandProgress {
            bar,
            header,
            command: cmd.to_string(),
            active: true,
        })
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

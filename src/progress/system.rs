use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use colored::Colorize;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use crate::memory::tasks::TaskState;

use super::{
    SYSTEM_TICK_CHARS, TASK_TICK_STRINGS, TICK_DURATION_MS_SYSTEM, TICK_DURATION_MS_TASK,
    task::TaskLogger,
};

#[derive(Debug, thiserror::Error)]
#[error("Failed to create system logger")]
pub struct SystemLoggerCreationError(#[from] indicatif::style::TemplateError);

#[derive(Debug, Clone, Default)]
pub(super) struct TaskSummary {
    success: Arc<AtomicUsize>,
    failed: Arc<AtomicUsize>,
    skipped: Arc<AtomicUsize>,
}

impl TaskSummary {
    pub(super) fn increment(&self, state: TaskState) {
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

        Ok(TaskLogger::new(
            self.multi_progress.clone(),
            bar,
            task_name.to_string(),
            self.summary.clone(),
        ))
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

#[derive(Debug, thiserror::Error)]
#[error("Failed to create task logger")]
pub struct TaskLoggerCreationError(#[from] indicatif::style::TemplateError);

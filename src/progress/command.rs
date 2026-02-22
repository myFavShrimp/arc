use colored::Colorize;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use super::{TASK_TICK_STRINGS, TICK_DURATION_MS_TASK};

#[derive(Debug, thiserror::Error)]
#[error("Failed to create command progress")]
pub struct CommandProgressCreationError(#[from] indicatif::style::TemplateError);

static MAX_OUTPUT_LINES: usize = 4;
static MAX_OUTPUT_LINE_WIDTH: usize = 42;

fn truncate_line(line: &str, max_width: usize) -> String {
    if line.len() <= max_width {
        line.to_string()
    } else {
        format!("{}...", &line[..max_width - 3])
    }
}

pub struct CommandProgress {
    bar: ProgressBar,
    header: String,
    command: String,
    active: bool,
}

impl CommandProgress {
    pub(crate) fn noop() -> Self {
        Self {
            bar: ProgressBar::hidden(),
            header: String::new(),
            command: String::new(),
            active: false,
        }
    }

    pub(crate) fn new(
        multi_progress: &MultiProgress,
        cmd: &str,
    ) -> Result<Self, CommandProgressCreationError> {
        let bar = multi_progress.insert(0, ProgressBar::new_spinner());

        bar.set_style(
            ProgressStyle::default_spinner()
                .tick_strings(TASK_TICK_STRINGS)
                .template(" {prefix}  {msg}")?,
        );

        let header = format!(" {}  {}", "CMND".cyan(), cmd.bright_black());

        bar.set_prefix(format!("{}", "CMND".cyan()));
        bar.set_message(format!("{}", cmd.bright_black()));
        bar.enable_steady_tick(std::time::Duration::from_millis(TICK_DURATION_MS_TASK));

        Ok(Self {
            bar,
            header,
            command: cmd.to_string(),
            active: true,
        })
    }

    pub fn update_output(&self, output: &str) {
        if !self.active {
            return;
        }

        let lines: Vec<&str> = output.lines().collect();
        let start = lines.len().saturating_sub(MAX_OUTPUT_LINES);
        let tail: String = lines[start..]
            .iter()
            .map(|line| {
                format!(
                    "       {}",
                    truncate_line(line, MAX_OUTPUT_LINE_WIDTH).bright_black()
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        if tail.is_empty() {
            self.bar
                .set_message(format!("{}", self.command.bright_black()));
        } else {
            self.bar
                .set_message(format!("{}\n{}", self.command.bright_black(), tail));
        }
    }

    pub fn finish(&self) {
        if self.active {
            self.bar.println(&self.header);
            self.bar.finish_and_clear();
        }
    }
}

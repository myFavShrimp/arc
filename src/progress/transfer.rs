use colored::Colorize;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

pub enum TransferDirection {
    Upload {
        source_file_path: Option<String>,
        target_file_path: String,
    },
    Download {
        source_file_path: String,
        target_file_path: Option<String>,
    },
    Copy {
        source_file_path: String,
        target_file_path: String,
    },
}

impl TransferDirection {
    fn label(&self) -> &'static str {
        match self {
            TransferDirection::Upload { .. } => "UPLD",
            TransferDirection::Download { .. } => "DWLD",
            TransferDirection::Copy { .. } => "COPY",
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Failed to create transfer progress")]
pub struct TransferProgressCreationError(#[from] indicatif::style::TemplateError);

pub struct TransferProgress {
    bar: ProgressBar,
    header: String,
    active: bool,
}

impl TransferProgress {
    pub(crate) fn noop() -> Self {
        Self {
            bar: ProgressBar::hidden(),
            header: String::new(),
            active: false,
        }
    }

    pub(crate) fn new(
        multi_progress: &MultiProgress,
        direction: TransferDirection,
        total: u64,
    ) -> Result<Self, TransferProgressCreationError> {
        let bar = multi_progress.insert(0, ProgressBar::new(total));

        bar.set_style(
            ProgressStyle::default_bar()
                .template(" {prefix}  {msg}\n       [{bar:30.dim}] {bytes}/{total_bytes}")?
                .progress_chars("█░ "),
        );

        let label = direction.label().cyan();

        let (header, message) = match &direction {
            TransferDirection::Upload {
                source_file_path: Some(source),
                target_file_path: target,
            } => {
                let from = format!("from {}", source).bright_black();
                let to = format!("to   {}", target).bright_black();

                (
                    format!(" {}  {}\n       {}", label, from, to),
                    format!("{}\n       {}", from, to),
                )
            }
            TransferDirection::Upload {
                source_file_path: None,
                target_file_path: target,
            } => {
                let path_colored = format!("to {}", target).bright_black();

                (
                    format!(" {}  {}", label, path_colored),
                    format!("{}", path_colored),
                )
            }
            TransferDirection::Download {
                source_file_path: source,
                target_file_path: Some(target),
            } => {
                let from = format!("from {}", source).bright_black();
                let to = format!("to   {}", target).bright_black();

                (
                    format!(" {}  {}\n       {}", label, from, to),
                    format!("{}\n       {}", from, to),
                )
            }
            TransferDirection::Download {
                source_file_path: source,
                target_file_path: None,
            } => {
                let path_colored = format!("from {}", source).bright_black();

                (
                    format!(" {}  {}", label, path_colored),
                    format!("{}", path_colored),
                )
            }
            TransferDirection::Copy {
                source_file_path: source,
                target_file_path: target,
            } => {
                let from = format!("from {}", source).bright_black();
                let to = format!("to   {}", target).bright_black();

                (
                    format!(" {}  {}\n       {}", label, from, to),
                    format!("{}\n       {}", from, to),
                )
            }
        };

        bar.set_prefix(format!("{}", label));
        bar.set_message(message);

        Ok(Self {
            bar,
            header,
            active: true,
        })
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

pub struct ProgressWriter<'a, W> {
    inner: W,
    progress: &'a TransferProgress,
    bytes_written: u64,
}

impl<'a, W> ProgressWriter<'a, W> {
    pub fn new(inner: W, progress: &'a TransferProgress) -> Self {
        Self {
            inner,
            progress,
            bytes_written: 0,
        }
    }
}

impl<W: std::io::Write> std::io::Write for ProgressWriter<'_, W> {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        let bytes_written = self.inner.write(buffer)?;

        self.bytes_written += bytes_written as u64;
        self.progress.update(self.bytes_written);

        Ok(bytes_written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

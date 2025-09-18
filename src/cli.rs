use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "arc")]
#[command(about = "A scriptable automation tool.", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Init {
        // Initialize project with type definitions for luau-lsp
        project_root: PathBuf,
    },
    Run {
        /// Filter tasks by tag
        #[arg(short, long)]
        tag: Vec<String>,
        /// Run tasks only on specific groups
        #[arg(short, long)]
        group: Vec<String>,
        /// Perform a dry run without executing commands or modifying the file system
        #[arg(short, long)]
        dry_run: bool,
    },
}

impl Default for Command {
    fn default() -> Self {
        Self::Run {
            tag: Vec::new(),
            group: Vec::new(),
            dry_run: false,
        }
    }
}

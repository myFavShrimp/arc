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
    /// Initialize project with type definitions for luau-lsp
    Init { project_root: PathBuf },
    /// Execute tasks
    Run {
        /// Select tasks by tag
        #[arg(short, long)]
        tag: Vec<String>,
        /// Run tasks only on specific groups
        #[arg(short, long)]
        group: Vec<String>,
        /// Print tasks that would be executed without running them
        #[arg(short, long)]
        dry_run: bool,
        /// Skip dependency resolution and only run explicitly selected tasks
        #[arg(long)]
        no_deps: bool,
        /// Run all tasks
        #[arg(long)]
        all_tags: bool,
    },
}

impl Default for Command {
    fn default() -> Self {
        Self::Run {
            tag: Vec::new(),
            group: Vec::new(),
            dry_run: false,
            no_deps: false,
            all_tags: false,
        }
    }
}

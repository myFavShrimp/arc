use std::path::PathBuf;

use clap::{ArgGroup, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "arc")]
#[command(version, about = "A scriptable automation tool.", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Initialize project with type definitions
    Init { project_root: PathBuf },
    /// Execute tasks
    #[command(group = ArgGroup::new("tags").required(true).args(["tag", "all_tags"]))]
    #[command(group = ArgGroup::new("targets").required(true).args(["group", "system", "all_systems"]))]
    Run {
        /// Select tasks by tag
        #[arg(short, long)]
        tag: Vec<String>,
        /// Run tasks only on specific groups
        #[arg(short, long)]
        group: Vec<String>,
        /// Run tasks only on specific systems
        #[arg(short, long)]
        system: Vec<String>,
        /// Print tasks that would be executed without running them
        #[arg(short, long)]
        dry_run: bool,
        /// Skip resolution of requires and only run explicitly selected tasks
        #[arg(long)]
        no_reqs: bool,
        /// Run all tasks
        #[arg(long)]
        all_tags: bool,
        /// Run on all systems
        #[arg(long)]
        all_systems: bool,
    },
    /// List registered items
    List {
        /// Item type to list
        #[arg(value_enum)]
        item_type: ListItemType,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum ListItemType {
    Tasks,
    Groups,
    Systems,
}

impl Default for Command {
    fn default() -> Self {
        Self::Run {
            tag: Vec::new(),
            group: Vec::new(),
            system: Vec::new(),
            dry_run: false,
            no_reqs: false,
            all_tags: false,
            all_systems: false,
        }
    }
}

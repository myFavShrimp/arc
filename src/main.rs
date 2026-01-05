use clap::Parser;
use cli::Cli;
use engine::Engine;
use logger::Logger;

use crate::engine::selection::{GroupSelection, SystemSelection, TagSelection};

mod cli;
mod engine;
mod error;
mod init;
mod logger;
mod memory;

fn main() -> Result<(), error::ErrorReport> {
    let cli_args = Cli::parse();
    let logger = Logger::new();

    match cli_args.command {
        cli::Command::Init { project_root } => {
            init::init_project(project_root).map_err(error::ErrorReport::boxed_from)?
        }
        cli::Command::Run {
            tag,
            group,
            system,
            dry_run,
            no_reqs,
            all_tags,
            all_systems,
        } => {
            let tags = if all_tags {
                TagSelection::All
            } else {
                TagSelection::Set(tag.into_iter().collect())
            };

            let groups = if group.is_empty() {
                GroupSelection::All
            } else {
                GroupSelection::Set(group.into_iter().collect())
            };

            let systems = if all_systems || system.is_empty() {
                SystemSelection::All
            } else {
                SystemSelection::Set(system.into_iter().collect())
            };

            if let Err(error) = dotenvy::dotenv_override() {
                logger.warn(&format!("Failed to load .env: {}", error));
            };

            Engine::new(logger, dry_run)
                .map_err(error::ErrorReport::boxed_from)?
                .execute(tags, groups, systems, no_reqs)
                .map_err(error::ErrorReport::boxed_from)?;
        }
    }

    Ok(())
}

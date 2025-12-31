use clap::Parser;
use cli::Cli;
use engine::Engine;
use logger::Logger;

use crate::engine::state::{GroupSelection, TagSelection};

mod cli;
mod engine;
mod error;
mod init;
mod logger;
mod memory;

#[derive(thiserror::Error, Debug)]
#[error("No tags specified. Use -t/--tag to select tasks or --all-tags.")]
struct NoTagsError;

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
            dry_run,
            no_deps,
            all_tags,
        } => {
            let tags = if all_tags {
                TagSelection::All
            } else if !tag.is_empty() {
                TagSelection::Set(tag.into_iter().collect())
            } else {
                return Err(error::ErrorReport::boxed_from(NoTagsError));
            };

            let groups = if group.is_empty() {
                GroupSelection::All
            } else {
                GroupSelection::Set(group.into_iter().collect())
            };

            if let Err(error) = dotenvy::dotenv_override() {
                logger.warn(&format!("Failed to load .env: {}", error));
            };

            Engine::new(logger, dry_run)
                .map_err(error::ErrorReport::boxed_from)?
                .execute(tags, groups, no_deps)
                .map_err(error::ErrorReport::boxed_from)?;
        }
    }

    Ok(())
}

use std::collections::HashSet;

use clap::Parser;
use cli::Cli;
use engine::Engine;
use logger::Logger;

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
            dry_run,
            no_deps,
            all_tags,
        } => {
            if tag.is_empty() && !all_tags {
                logger.error("No tags specified. Use -t/--tag to select tasks or --all-tags.");

                std::process::exit(1);
            }

            if let Err(error) = dotenvy::dotenv_override() {
                logger.warn(&format!("Failed to load .env: {}", error));
            };

            let tags: HashSet<String> = tag.into_iter().collect();
            let groups: HashSet<String> = group.into_iter().collect();

            Engine::new(logger, dry_run)
                .map_err(error::ErrorReport::boxed_from)?
                .execute(tags, groups, no_deps)
                .map_err(error::ErrorReport::boxed_from)?;
        }
    }

    Ok(())
}

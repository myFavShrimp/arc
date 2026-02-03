use clap::Parser;
use cli::Cli;
use engine::Engine;

use crate::{
    engine::selection::{GroupSelection, SystemSelection, TagSelection},
    logger::Logger,
};

mod cli;
mod engine;
mod error;
mod init;
mod list;
mod logger;
mod memory;
mod progress;

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
            list,
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
                GroupSelection::None
            } else {
                GroupSelection::Set(group.into_iter().collect())
            };

            let systems = if all_systems {
                SystemSelection::All
            } else if system.is_empty() {
                SystemSelection::None
            } else {
                SystemSelection::Set(system.into_iter().collect())
            };

            if let Err(error) = dotenvy::dotenv_override() {
                logger.warn(&format!("Failed to load .env: {}", error));
            };

            let engine = Engine::new(logger).map_err(error::ErrorReport::boxed_from)?;

            if list {
                engine
                    .execute_entrypoint()
                    .map_err(error::ErrorReport::boxed_from)?;

                let system_tasks = engine
                    .validate_and_filter_by_selection(&tags, &groups, &systems, no_reqs)
                    .map_err(error::ErrorReport::boxed_from)?;

                for (system, tasks) in &system_tasks {
                    if !tasks.is_empty() {
                        println!("\nSYSTEM : {}\n", system.name);

                        list::list_system_tasks(tasks);
                    }
                }
            } else {
                engine
                    .execute(tags, groups, systems, no_reqs)
                    .map_err(error::ErrorReport::boxed_from)?;
            }
        }
        cli::Command::List { item_type, json } => {
            if let Err(error) = dotenvy::dotenv_override() {
                logger.warn(&format!("Failed to load .env: {}", error));
            };

            let engine = Engine::new(logger).map_err(error::ErrorReport::boxed_from)?;

            engine
                .execute_entrypoint()
                .map_err(error::ErrorReport::boxed_from)?;

            list::list(&engine, item_type, json).map_err(error::ErrorReport::boxed_from)?;
        }
    }

    Ok(())
}

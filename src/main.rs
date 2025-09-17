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
            verbose,
            tag,
            group,
            dry_run,
        } => {
            if let Err(error) = dotenvy::dotenv_override() {
                logger.warn(&format!("Failed to load .env: {}", error));
            };

            Engine::new(logger, verbose, dry_run)
                .map_err(error::ErrorReport::boxed_from)?
                .execute(tag, group)
                .map_err(error::ErrorReport::boxed_from)?;
        }
    }

    Ok(())
}

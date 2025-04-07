use clap::Parser;
use cli::Cli;
use engine::Engine;
use logger::Logger;

mod cli;
mod engine;
mod error;
mod logger;
mod memory;

fn main() -> Result<(), error::ErrorReport> {
    let cli_args = Cli::parse();
    let logger = Logger::new();

    if let Err(error) = dotenvy::dotenv_override() {
        logger.warn(&format!("Failed to load .env: {}", error));
    };

    Engine::new(logger, cli_args.verbose, cli_args.dry_run)
        .map_err(error::ErrorReport::boxed_from)?
        .execute(cli_args.tag, cli_args.group)
        .map_err(error::ErrorReport::boxed_from)?;

    Ok(())
}

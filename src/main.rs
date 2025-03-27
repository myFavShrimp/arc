use clap::Parser;
use cli::Cli;
use engine::Engine;

mod cli;
mod engine;
mod error;
mod logger;
mod memory;
mod ssh;

fn main() -> Result<(), error::ErrorReport> {
    let cli_args = Cli::parse();

    let current_dir = std::env::current_dir().map_err(error::ErrorReport::boxed_from)?;

    Engine::new(current_dir, cli_args.verbose, cli_args.dry_run)
        .map_err(error::ErrorReport::boxed_from)?
        .execute(cli_args.tag, cli_args.group)
        .map_err(error::ErrorReport::boxed_from)?;

    Ok(())
}

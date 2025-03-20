use clap::Parser;
use cli::Cli;
use engine::Engine;
use log::LevelFilter;

mod cli;
mod engine;
mod error;
mod operations;
mod ssh;

fn main() -> Result<(), error::ErrorReport> {
    let cli_args = Cli::parse();

    let log_level = match cli_args.verbose {
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        2.. => LevelFilter::Trace,
    };

    env_logger::Builder::new().filter_level(log_level).init();

    Engine::new()
        .map_err(error::ErrorReport::boxed_from)?
        .execute(cli_args.tag)
        .map_err(error::ErrorReport::boxed_from)?;

    Ok(())
}

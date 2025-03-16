use clap::Parser;
use cli::Cli;
use engine::Engine;
use log::LevelFilter;

mod cli;
mod engine;
mod error;
mod inventory;
mod operations;
mod ssh;
mod tasks;

fn main() -> Result<(), error::ErrorReport> {
    let _ = Cli::parse();

    env_logger::Builder::new()
        .filter_level(LevelFilter::Trace)
        .init();

    Engine::new()
        .map_err(error::ErrorReport::boxed_from)?
        .execute()
        .map_err(error::ErrorReport::boxed_from)?;

    Ok(())
}

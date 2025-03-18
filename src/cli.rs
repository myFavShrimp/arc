use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "arc")]
#[command(about = "Written in Rust, using Luau", long_about = None)]
pub struct Cli {
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
    #[arg(short, long)]
    pub tag: Vec<String>,
}

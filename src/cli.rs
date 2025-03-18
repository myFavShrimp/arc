use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "arc")]
#[command(about = "Written in Rust, using Luau", long_about = None)]
pub struct Cli {
    // #[arg(short, long)]
    // pub verbose: bool,
    #[arg(short, long)]
    pub tag: Vec<String>,
}

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "arc")]
#[command(about = "An infrastructure automation tool.", long_about = None)]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
    /// Filter tasks by tag
    #[arg(short, long)]
    pub tag: Vec<String>,
    /// Run tasks only on specific groups
    #[arg(short, long)]
    pub group: Vec<String>,
    /// Perform a dry run without executing commands or modifying the file system
    #[arg(short, long)]
    pub dry_run: bool,
}

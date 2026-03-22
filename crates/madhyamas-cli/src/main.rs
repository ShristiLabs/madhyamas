//! Madhyamas CLI - Command-line interface for interacting with Madhyamas proxy server

use anyhow::Result;
use clap::Parser;

mod commands;

use commands::Commands;

use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[derive(Parser, Debug)]
#[command(name = "madhyamas-cli")]
#[command(author, version, about)]
#[command(about = "CLI tool for interacting with Madhyamas proxy server")]
struct Args {
    /// API server URL
    #[arg(short, long, default_value = "http://127.0.0.1:3001", env = "MADHYAMAS_API_URL")]
    api_url: String,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    let level = if args.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };
    let subscriber = FmtSubscriber::builder().with_max_level(level).finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    // Execute the command
    args.command.execute(args.api_url).await
}

//! Madhyamas CLI - Command-line interface for interacting with Madhyamas proxy server

use anyhow::Result;
use clap::Parser;

mod commands;

use commands::{CliAuth, Commands};

use tracing::Level;
use tracing_subscriber::FmtSubscriber;

#[derive(Parser, Debug)]
#[command(name = "madhyamas-cli")]
#[command(author = "Madhyamas Team", version, about, long_about = None)]
#[command(about = "CLI tool for interacting with Madhyamas proxy server")]
struct Args {
    /// API server URL
    #[arg(
        short,
        long,
        default_value = "http://127.0.0.1:3001",
        env = "MADHYAMAS_API_URL"
    )]
    api_url: String,

    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// API key for authenticating against an enterprise API server with
    /// `--enable-auth`. Sent as the `X-API-Key` header. Overrides the
    /// `MADHYAMAS_API_KEY` environment variable. When both `--api-key` and
    /// `--token` are provided, the API key takes precedence.
    #[arg(long, env = "MADHYAMAS_API_KEY")]
    api_key: Option<String>,

    /// JWT token for authenticating against an enterprise API server with
    /// `--enable-auth`. Sent as the `Authorization: Bearer <token>` header.
    /// Overrides the `MADHYAMAS_TOKEN` environment variable. When both
    /// `--api-key` and `--token` are provided, the API key takes precedence.
    #[arg(long, env = "MADHYAMAS_TOKEN")]
    token: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

/// Resolve CLI auth from flags / env vars. API key takes precedence over JWT.
fn resolve_auth(api_key: &Option<String>, token: &Option<String>) -> CliAuth {
    if let Some(key) = api_key {
        return CliAuth::ApiKey(key.clone());
    }
    if let Some(t) = token {
        return CliAuth::Jwt(t.clone());
    }
    CliAuth::None
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
    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stdout()))
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    // Execute the command
    let auth = resolve_auth(&args.api_key, &args.token);
    args.command.execute(args.api_url, auth).await
}

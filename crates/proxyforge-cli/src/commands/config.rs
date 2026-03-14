//! Configuration commands

use anyhow::Result;
use clap::Subcommand;
use serde_json::Value;

use super::ApiClient;
#[derive(Debug, Args)]
pub struct ConfigGetArgs {
    /// Output as JSON
    #[arg(long)]
    json: bool,
}
#[derive(Debug, Args)]
pub struct ConfigSetArgs {
    /// Configuration key
    key: String,

    /// Configuration value
        value: String,
}
#[derive(Debug, Subcommand)]
pub enum ConfigCommands {
    /// Get current configuration
    get,

    /// Set configuration value
    set(ConfigSetArgs),
}

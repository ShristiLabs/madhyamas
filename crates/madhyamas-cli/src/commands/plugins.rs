//! Plugin commands

use anyhow::Result;
use clap::{Args, Subcommand};
use serde_json::json;

use super::ApiClient;

#[derive(Debug, Args)]
pub struct PluginIdArgs {
    /// Plugin ID
    pub id: String,
}

#[derive(Debug, Subcommand)]
pub enum PluginCommands {
    /// List all plugins
    List,
    /// Get a specific plugin
    Get(PluginIdArgs),
    /// Enable a plugin
    Enable(PluginIdArgs),
    /// Disable a plugin
    Disable(PluginIdArgs),
    /// Get statistics for a plugin
    Stats(PluginIdArgs),
    /// Reload all plugins
    Reload,
}

impl PluginCommands {
    pub async fn execute(&self, api_url: String) -> Result<()> {
        let client = ApiClient::new(api_url);

        match self {
            PluginCommands::List => {
                let result = client.get("plugins").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            PluginCommands::Get(args) => {
                let result = client.get(&format!("plugins/{}", args.id)).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            PluginCommands::Enable(args) => {
                let result = client
                    .post(&format!("plugins/{}/enable", args.id), json!({}))
                    .await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            PluginCommands::Disable(args) => {
                let result = client
                    .post(&format!("plugins/{}/disable", args.id), json!({}))
                    .await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            PluginCommands::Stats(args) => {
                let result = client.get(&format!("plugins/{}/stats", args.id)).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            PluginCommands::Reload => {
                let result = client.post("plugins/reload", json!({})).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }

        Ok(())
    }
}

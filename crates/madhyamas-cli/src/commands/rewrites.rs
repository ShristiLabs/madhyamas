//! Rewrite rule commands

use anyhow::Result;
use clap::{Args, Subcommand};
use serde_json::json;

use super::ApiClient;

#[derive(Debug, Args)]
pub struct RewriteCreateArgs {
    /// Name of the rewrite rule
    #[arg(short, long)]
    pub name: String,

    /// URL pattern to match
    #[arg(short, long)]
    pub pattern: String,

    /// Replacement action (e.g. the new URL or body)
    #[arg(short, long)]
    pub action: String,
}

#[derive(Debug, Args)]
pub struct RewriteIdArgs {
    /// Rewrite rule ID
    pub id: String,
}

#[derive(Debug, Subcommand)]
pub enum RewriteCommands {
    /// List all rewrite rules
    List,
    /// Create a rewrite rule
    Create(RewriteCreateArgs),
    /// Delete a rewrite rule
    Delete(RewriteIdArgs),
    /// Toggle a rewrite rule on/off
    Toggle(RewriteIdArgs),
    /// List available rewrite templates
    Templates,
}

impl RewriteCommands {
    pub async fn execute(&self, api_url: String) -> Result<()> {
        let client = ApiClient::new(api_url);

        match self {
            RewriteCommands::List => {
                let result = client.get("rewrites").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            RewriteCommands::Create(args) => {
                let body = json!({
                    "name": args.name,
                    "pattern": args.pattern,
                    "action": args.action,
                });
                let result = client.post("rewrites", body).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            RewriteCommands::Delete(args) => {
                let result = client.delete(&format!("rewrites/{}", args.id)).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            RewriteCommands::Toggle(args) => {
                let result = client
                    .post(&format!("rewrites/{}/toggle", args.id), json!({}))
                    .await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            RewriteCommands::Templates => {
                let result = client.get("rewrites/templates").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }

        Ok(())
    }
}

//! Rewrite rule commands

use anyhow::Result;
use clap::{Args, Subcommand};
use serde_json::{json, Value};

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

#[derive(Debug, Args)]
pub struct RewriteUpdateArgs {
    /// Rewrite rule ID
    pub id: String,

    /// Name of the rewrite rule
    #[arg(short, long)]
    pub name: Option<String>,

    /// URL pattern to match
    #[arg(short, long)]
    pub pattern: Option<String>,

    /// Replacement action (e.g. the new URL or body)
    #[arg(short, long)]
    pub action: Option<String>,
}

#[derive(Debug, Args)]
pub struct RewriteBatchToggleArgs {
    /// Comma-separated list of rewrite rule IDs
    #[arg(short, long)]
    pub ids: String,

    /// Enable or disable
    #[arg(short, long)]
    pub enabled: bool,
}

#[derive(Debug, Subcommand)]
pub enum RewriteCommands {
    /// List all rewrite rules
    List,
    /// Get a specific rewrite rule
    Get(RewriteIdArgs),
    /// Create a rewrite rule
    Create(RewriteCreateArgs),
    /// Update a rewrite rule
    Update(RewriteUpdateArgs),
    /// Delete a rewrite rule
    Delete(RewriteIdArgs),
    /// Toggle a rewrite rule on/off
    Toggle(RewriteIdArgs),
    /// Batch toggle multiple rewrite rules
    BatchToggle(RewriteBatchToggleArgs),
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
            RewriteCommands::Get(args) => {
                let result = client.get(&format!("rewrites/{}", args.id)).await?;
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
            RewriteCommands::Update(args) => {
                let mut body = json!({});
                if let Some(ref name) = args.name {
                    body["name"] = Value::String(name.clone());
                }
                if let Some(ref pattern) = args.pattern {
                    body["pattern"] = Value::String(pattern.clone());
                }
                if let Some(ref action) = args.action {
                    body["action"] = Value::String(action.clone());
                }
                let result = client.put(&format!("rewrites/{}", args.id), body).await?;
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
            RewriteCommands::BatchToggle(args) => {
                let ids: Vec<&str> = args.ids.split(',').map(|s| s.trim()).collect();
                let body = json!({ "ids": ids, "enabled": args.enabled });
                let result = client.post("rewrites/batch-toggle", body).await?;
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

//! Focus host commands

use anyhow::Result;
use clap::{Args, Subcommand};
use serde_json::json;

use super::ApiClient;

#[derive(Debug, Args)]
pub struct FocusAddArgs {
    /// Host pattern to focus (e.g. `api.example.com`, `*.example.com`, `*api*`)
    pub pattern: String,
}

#[derive(Debug, Args)]
pub struct FocusRemoveArgs {
    /// Focus host ID
    pub id: String,
}

#[derive(Debug, Subcommand)]
pub enum FocusCommands {
    /// List all focus host patterns
    List,
    /// Add a focus host pattern
    Add(FocusAddArgs),
    /// Remove a focus host by ID
    Remove(FocusRemoveArgs),
    /// Clear all focus hosts
    Clear,
}

impl FocusCommands {
    pub async fn execute(&self, api_url: String, auth: super::CliAuth) -> Result<()> {
        let client = ApiClient::new(api_url, auth.clone());

        match self {
            FocusCommands::List => {
                let result = client.get("focus").await?;
                if let Some(hosts) = result.as_array() {
                    if hosts.is_empty() {
                        println!("No focus hosts.");
                        return Ok(());
                    }
                    println!("{:<36}  PATTERN", "ID");
                    println!("{}", "-".repeat(60));
                    for host in hosts {
                        let id = host.get("id").and_then(|v| v.as_str()).unwrap_or("-");
                        let pattern = host.get("pattern").and_then(|v| v.as_str()).unwrap_or("-");
                        println!("{:<36}  {}", id, pattern);
                    }
                } else {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
            }
            FocusCommands::Add(args) => {
                let body = json!({ "pattern": args.pattern });
                let result = client.post("focus", body).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            FocusCommands::Remove(args) => {
                client.delete(&format!("focus/{}", args.id)).await?;
                println!("Removed focus host {}", args.id);
            }
            FocusCommands::Clear => {
                client.delete("focus").await?;
                println!("Cleared all focus hosts.");
            }
        }

        Ok(())
    }
}

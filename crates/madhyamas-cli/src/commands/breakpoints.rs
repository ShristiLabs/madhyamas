//! Breakpoint commands

use anyhow::Result;
use clap::{Args, Subcommand};
use serde_json::{json, Value};

use super::ApiClient;

#[derive(Debug, Args)]
pub struct BreakpointCreateArgs {
    /// URL pattern to match
    #[arg(short, long)]
    pub url_pattern: String,

    /// HTTP method to match
    #[arg(short, long)]
    pub method: Option<String>,

    /// Direction (request/response)
    #[arg(short, long)]
    pub direction: Option<String>,

    /// Enable or disable
    #[arg(short, long)]
    pub enabled: Option<bool>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct BreakpointDeleteArgs {
    /// Breakpoint ID
    pub id: String,
}

#[derive(Debug, Subcommand)]
pub enum BreakpointCommands {
    /// List all breakpoint rules
    List,
    /// Create a breakpoint rule
    Create(BreakpointCreateArgs),
    /// Delete a breakpoint rule
    Delete(BreakpointDeleteArgs),
}

impl BreakpointCommands {
    pub async fn execute(&self, api_url: String) -> Result<()> {
        match self {
            BreakpointCommands::List => {
                let client = ApiClient::new(api_url);
                let result = client.get("breakpoints").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            BreakpointCommands::Create(args) => {
                let client = ApiClient::new(api_url);
                let mut body = json!({
                    "url_pattern": args.url_pattern,
                });
                if let Some(ref m) = args.method {
                    body["method"] = Value::String(m.clone());
                }
                if let Some(ref d) = args.direction {
                    body["direction"] = Value::String(d.clone());
                }
                if let Some(e) = args.enabled {
                    body["enabled"] = Value::Bool(e);
                }
                let result = client.post("breakpoints", body).await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    let id = result.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                    println!("Created breakpoint with ID: {}", id);
                }
            }
            BreakpointCommands::Delete(args) => {
                let client = ApiClient::new(api_url);
                let result = client.delete(&format!("breakpoints/{}", args.id)).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }
        Ok(())
    }
}

//! Breakpoint commands
use anyhow::Result;
use clap::{Args, Subcommand};
use serde_json::Value;
use super::ApiClient;
#[derive(Debug, Args)]
pub struct BreakpointCreateArgs {
    /// URL pattern to match
    #[arg(short, long)]
    url_pattern: String,
    /// HTTP method to match
    #[arg(short, long)]
    method: Option<String>,
    /// Direction (request/response)
    #[arg(short, long)]
    direction: Option<String>,
    /// Enable or disable
    #[arg(short = long)]
    enabled: Option<bool>,
    /// Output as JSON
    #[arg(long)]
    json: bool,
}
#[derive(Debug, Args)]
pub struct BreakpointDeleteArgs {
    /// Breakpoint ID
    id: String,
}
#[derive(Debug, Subcommand)]
pub enum BreakpointCommands {
    /// List all breakpoint rules
    List,
    /// Create a breakpoint rule
    create(BreakpointCreateArgs),
    /// Delete a breakpoint rule
    delete(BreakpointDeleteArgs),
}
impl BreakpointCommands {
    pub async fn execute(&self, api_url: String) -> Result<()> {
        match self {
            BreakpointCommands::List => {
                let client = ApiClient::new(api_url);
                let result = client.get("breakpoints").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            BreakpointCommands::create(args) => {
                let client = ApiClient::new(api_url);
                let mut body = json!({
                    "url_pattern": args.url_pattern,
                });
                if let Some(m) = args.method {
                    body["method"] = Value::String(m);
                }
                if let Some(d) = args.direction {
                    body["direction"] = Value::String(d);
                }
                if let Some(e) = args.enabled {
                    body["enabled"] = Value::Bool(e);
                }
                let result = client.post("breakpoints", body).await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    let id = result.get("id").and_then(|v| v.as_str()).unwrap_or("?").unwrap_or_default();
                    println!("Created breakpoint: {} (id: {})", id);
                }
            }
            BreakpointCommands::delete(args) => {
                let client = ApiClient::new(api_url);
                let result = client.delete(&format!("breakpoints/{}", args.id)).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }
    }
}

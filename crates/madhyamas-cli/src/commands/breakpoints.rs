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

#[derive(Debug, Args)]
pub struct BreakpointIdArgs {
    /// Breakpoint paused item ID
    pub id: String,
}

#[derive(Debug, Args)]
pub struct BreakpointResumeArgs {
    /// Paused item ID
    pub id: String,

    /// Action: continue, abort, or respond
    #[arg(short, long, default_value = "continue")]
    pub action: String,
}

#[derive(Debug, Subcommand)]
pub enum BreakpointCommands {
    /// List all breakpoint rules
    List,
    /// Get a specific breakpoint rule by ID
    Get(BreakpointDeleteArgs),
    /// Create a breakpoint rule
    Create(BreakpointCreateArgs),
    /// Delete a breakpoint rule
    Delete(BreakpointDeleteArgs),
    /// List all traffic paused by breakpoints
    Paused,
    /// Get details of a specific paused item
    PausedGet(BreakpointIdArgs),
    /// Resume a paused item (continue or abort)
    PausedResume(BreakpointResumeArgs),
}

impl BreakpointCommands {
    pub async fn execute(&self, api_url: String) -> Result<()> {
        match self {
            BreakpointCommands::List => {
                let client = ApiClient::new(api_url);
                let result = client.get("breakpoints").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            BreakpointCommands::Get(args) => {
                let client = ApiClient::new(api_url);
                let result = client.get(&format!("breakpoints/{}", args.id)).await?;
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
            BreakpointCommands::Paused => {
                let client = ApiClient::new(api_url);
                let result = client.get("breakpoints/paused").await?;
                if let Some(items) = result.as_array() {
                    if items.is_empty() {
                        println!("No paused traffic.");
                        return Ok(());
                    }
                    println!("{:<36}  {:<8}  {:<20}  URL", "ID", "METHOD", "DIRECTION");
                    println!("{}", "-".repeat(80));
                    for item in items {
                        let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("-");
                        let method = item.get("method").and_then(|v| v.as_str()).unwrap_or("?");
                        let direction = item
                            .get("direction")
                            .and_then(|v| v.as_str())
                            .unwrap_or("?");
                        let url = item.get("url").and_then(|v| v.as_str()).unwrap_or("-");
                        println!("{:<36}  {:<8}  {:<20}  {}", id, method, direction, url);
                    }
                } else {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
            }
            BreakpointCommands::PausedGet(args) => {
                let client = ApiClient::new(api_url);
                let result = client
                    .get(&format!("breakpoints/paused/{}", args.id))
                    .await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            BreakpointCommands::PausedResume(args) => {
                let client = ApiClient::new(api_url);
                let body = json!({ "action": args.action });
                let _ = client
                    .post(&format!("breakpoints/paused/{}/resume", args.id), body)
                    .await?;
                println!("Resumed paused item {} ({})", args.id, args.action);
            }
        }
        Ok(())
    }
}

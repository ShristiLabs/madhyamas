//! Block list commands

use anyhow::Result;
use clap::{Args, Subcommand};
use serde_json::{json, Value};

use super::ApiClient;

#[derive(Debug, Args)]
pub struct BlockListCreateArgs {
    /// Domain or wildcard pattern to block (e.g. `ads.example.com`, `*.tracker.com`, `*ads*`)
    #[arg(short, long)]
    pub pattern: String,

    /// Optional note describing why this entry exists
    #[arg(short, long)]
    pub note: Option<String>,

    /// Whether the entry is enabled immediately (default: true)
    #[arg(short, long)]
    pub enabled: Option<bool>,

    /// HTTP status code to return (default: 403)
    #[arg(long)]
    pub status_code: Option<u16>,

    /// Response body to return when blocked
    #[arg(long)]
    pub response_body: Option<String>,

    /// Content-Type header for the block response
    #[arg(long)]
    pub content_type: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct BlockListUpdateArgs {
    /// Block list entry ID
    pub id: String,

    /// Domain or wildcard pattern
    #[arg(short, long)]
    pub pattern: Option<String>,

    /// Optional note
    #[arg(short, long)]
    pub note: Option<String>,

    /// Enable or disable
    #[arg(short, long)]
    pub enabled: Option<bool>,

    /// HTTP status code
    #[arg(long)]
    pub status_code: Option<u16>,

    /// Response body
    #[arg(long)]
    pub response_body: Option<String>,

    /// Content-Type header
    #[arg(long)]
    pub content_type: Option<String>,
}

#[derive(Debug, Args)]
pub struct BlockListIdArgs {
    /// Block list entry ID
    pub id: String,
}

#[derive(Debug, Args)]
pub struct BlockListToggleArgs {
    /// Block list entry ID
    pub id: String,

    /// Enable (true) or disable (false)
    pub enabled: bool,
}

#[derive(Debug, Subcommand)]
pub enum BlockListCommands {
    /// List all block list entries
    List,
    /// View block list summary statistics
    Stats,
    /// Get a specific block list entry by ID
    Get(BlockListIdArgs),
    /// Create a block list entry
    Create(BlockListCreateArgs),
    /// Update a block list entry
    Update(BlockListUpdateArgs),
    /// Delete a block list entry
    Delete(BlockListIdArgs),
    /// Enable or disable a block list entry
    Toggle(BlockListToggleArgs),
}

impl BlockListCommands {
    pub async fn execute(&self, api_url: String) -> Result<()> {
        let client = ApiClient::new(api_url);

        match self {
            BlockListCommands::List => {
                let result = client.get("blocklist").await?;
                if let Some(entries) = result.as_array() {
                    if entries.is_empty() {
                        println!("No block list entries.");
                        return Ok(());
                    }
                    println!(
                        "{:<36}  {:<6}  {:<6}  {:<6}  PATTERN",
                        "ID", "ENABLED", "HITS", "STATUS"
                    );
                    println!("{}", "-".repeat(80));
                    for entry in entries {
                        let id = entry.get("id").and_then(|v| v.as_str()).unwrap_or("-");
                        let pattern = entry.get("pattern").and_then(|v| v.as_str()).unwrap_or("-");
                        let enabled = entry
                            .get("enabled")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        let hits = entry.get("hit_count").and_then(|v| v.as_u64()).unwrap_or(0);
                        let status = entry
                            .get("status_code")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(403);
                        println!(
                            "{:<36}  {:<6}  {:<6}  {:<6}  {}",
                            id,
                            if enabled { "yes" } else { "no" },
                            hits,
                            status,
                            pattern
                        );
                    }
                } else {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
            }
            BlockListCommands::Stats => {
                let result = client.get("blocklist/stats").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            BlockListCommands::Get(args) => {
                let result = client.get(&format!("blocklist/{}", args.id)).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            BlockListCommands::Create(args) => {
                let mut body = json!({ "pattern": args.pattern });
                if let Some(ref n) = args.note {
                    body["note"] = Value::String(n.clone());
                }
                if let Some(e) = args.enabled {
                    body["enabled"] = Value::Bool(e);
                }
                if let Some(sc) = args.status_code {
                    body["status_code"] = json!(sc);
                }
                if let Some(ref rb) = args.response_body {
                    body["response_body"] = Value::String(rb.clone());
                }
                if let Some(ref ct) = args.content_type {
                    body["content_type"] = Value::String(ct.clone());
                }
                let result = client.post("blocklist", body).await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    let id = result.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                    println!("Created block list entry with ID: {}", id);
                }
            }
            BlockListCommands::Update(args) => {
                // Fetch current entry to merge updates
                let current = client.get(&format!("blocklist/{}", args.id)).await?;
                let mut body = current.clone();
                if let Some(ref p) = args.pattern {
                    body["pattern"] = Value::String(p.clone());
                }
                if let Some(ref n) = args.note {
                    body["note"] = Value::String(n.clone());
                }
                if let Some(e) = args.enabled {
                    body["enabled"] = Value::Bool(e);
                }
                if let Some(sc) = args.status_code {
                    body["status_code"] = json!(sc);
                }
                if let Some(ref rb) = args.response_body {
                    body["response_body"] = Value::String(rb.clone());
                }
                if let Some(ref ct) = args.content_type {
                    body["content_type"] = Value::String(ct.clone());
                }
                client.put(&format!("blocklist/{}", args.id), body).await?;
                println!("Updated block list entry {}", args.id);
            }
            BlockListCommands::Delete(args) => {
                client
                    .delete_void(&format!("blocklist/{}", args.id))
                    .await?;
                println!("Deleted block list entry {}", args.id);
            }
            BlockListCommands::Toggle(args) => {
                let body = json!({ "enabled": args.enabled });
                client
                    .post(&format!("blocklist/{}/toggle", args.id), body)
                    .await?;
                println!(
                    "Block list entry {} {}",
                    args.id,
                    if args.enabled { "enabled" } else { "disabled" }
                );
            }
        }

        Ok(())
    }
}

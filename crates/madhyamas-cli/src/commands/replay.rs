//! Replay commands

use anyhow::Result;
use clap::{Args, Subcommand};
use serde_json::{json, Value};

use super::ApiClient;

#[derive(Debug, Args)]
pub struct ReplayRequestArgs {
    /// Traffic entry ID to replay
    pub id: String,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct ReplaySaveArgs {
    /// Traffic entry ID
    pub traffic_id: String,

    /// Optional name for the saved request
    #[arg(short, long)]
    pub name: Option<String>,

    /// Optional description
    #[arg(short, long)]
    pub description: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct ReplayListArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct ReplayDeleteArgs {
    /// Saved request ID to delete
    pub id: String,
}

#[derive(Debug, Args)]
pub struct ReplayExportArgs {
    /// Traffic entry ID
    pub id: String,

    /// Export format (curl, har)
    #[arg(short, long, default_value = "curl")]
    pub format: String,
}

#[derive(Debug, Subcommand)]
pub enum ReplayCommands {
    /// Replay a captured request
    Run(ReplayRequestArgs),
    /// Save a request for later replay
    Save(ReplaySaveArgs),
    /// List saved requests
    List(ReplayListArgs),
    /// Delete a saved request
    Delete(ReplayDeleteArgs),
    /// Export request as cURL or HAR
    Export(ReplayExportArgs),
    /// Show replay history
    History(ReplayListArgs),
}

impl ReplayCommands {
    pub async fn execute(&self, api_url: String) -> Result<()> {
        match self {
            ReplayCommands::Run(args) => {
                let client = ApiClient::new(api_url);
                let body = json!({ "traffic_id": args.id });
                let result = client.post("replay", body).await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("Replay completed successfully");
                    if let Some(status) = result.get("status") {
                        println!("Status: {}", status);
                    }
                }
            }
            ReplayCommands::Save(args) => {
                let client = ApiClient::new(api_url);
                let mut body = json!({
                    "traffic_id": args.traffic_id,
                });
                if let Some(ref n) = args.name {
                    body["name"] = Value::String(n.clone());
                }
                if let Some(ref d) = args.description {
                    body["description"] = Value::String(d.clone());
                }
                let result = client.post("replay/saved", body).await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    let id = result.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                    println!("Saved request with ID: {}", id);
                }
            }
            ReplayCommands::List(args) => {
                let client = ApiClient::new(api_url);
                let result = client.get("replay/saved").await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    print_saved_requests(&result);
                }
            }
            ReplayCommands::Delete(args) => {
                let client = ApiClient::new(api_url);
                let result = client.delete(&format!("replay/saved/{}", args.id)).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            ReplayCommands::Export(args) => {
                let client = ApiClient::new(api_url);
                let result = client
                    .get(&format!(
                        "traffic/{}/export?format={}",
                        args.id, args.format
                    ))
                    .await?;
                if let Some(export) = result.get("export").and_then(|v| v.as_str()) {
                    println!("{}", export);
                } else {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
            }
            ReplayCommands::History(args) => {
                let client = ApiClient::new(api_url);
                let result = client.get("replay/history").await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    print_replay_history(&result);
                }
            }
        }
        Ok(())
    }
}

fn print_saved_requests(result: &Value) {
    if let Some(requests) = result.as_array() {
        if requests.is_empty() {
            println!("No saved requests found.");
            return;
        }
        println!("{:<36} {:<10} {:<50}", "ID", "METHOD", "URL");
        println!("{}", "-".repeat(96));
        for req in requests {
            let id = req.get("id").and_then(|v| v.as_str()).unwrap_or("-");
            let method = req.get("method").and_then(|v| v.as_str()).unwrap_or("-");
            let url = req.get("url").and_then(|v| v.as_str()).unwrap_or("-");
            let url_truncated = if url.len() > 50 {
                format!("{}...", &url[..47])
            } else {
                url.to_string()
            };
            println!("{:<36} {:<10} {:<50}", id, method, url_truncated);
        }
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(result).unwrap_or_default()
        );
    }
}

fn print_replay_history(result: &Value) {
    if let Some(history) = result.as_array() {
        if history.is_empty() {
            println!("No replay history found.");
            return;
        }
        println!(
            "{:<36} {:<10} {:<6} {:<20}",
            "ID", "METHOD", "STATUS", "TIMESTAMP"
        );
        println!("{}", "-".repeat(80));
        for entry in history {
            let id = entry.get("id").and_then(|v| v.as_str()).unwrap_or("-");
            let method = entry.get("method").and_then(|v| v.as_str()).unwrap_or("-");
            let status = entry.get("status").and_then(|v| v.as_u64()).unwrap_or(0);
            let timestamp = entry
                .get("timestamp")
                .and_then(|v| v.as_str())
                .unwrap_or("-");
            println!("{:<36} {:<10} {:<6} {:<20}", id, method, status, timestamp);
        }
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(result).unwrap_or_default()
        );
    }
}

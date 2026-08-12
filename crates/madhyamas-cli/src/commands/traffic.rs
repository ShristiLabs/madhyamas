//! Traffic inspection commands

use anyhow::Result;
use clap::{Args, Subcommand};
use serde_json::Value;

use super::ApiClient;

#[derive(Debug, Args)]
pub struct TrafficListArgs {
    /// Filter by URL pattern
    #[arg(short, long)]
    pub filter: Option<String>,

    /// Filter by HTTP method
    #[arg(short, long)]
    pub method: Option<String>,

    /// Filter by HTTP status code
    #[arg(short, long)]
    pub status: Option<u16>,

    /// Maximum number of results
    #[arg(short, long, default_value = "100")]
    pub limit: usize,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct TrafficGetArgs {
    /// Traffic entry ID
    pub id: String,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct TrafficSearchArgs {
    /// Search query
    pub query: String,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct TrafficImportHarArgs {
    /// Path to the HAR file to import
    pub file: String,

    /// Optional name for the newly created session
    #[arg(short, long)]
    pub name: Option<String>,

    /// Switch to the newly created session after import
    #[arg(long)]
    pub switch: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Subcommand)]
pub enum TrafficCommands {
    /// List captured traffic
    List(TrafficListArgs),
    /// Get a specific traffic entry
    Get(TrafficGetArgs),
    /// Search traffic by query
    Search(TrafficSearchArgs),
    /// Get traffic count
    Count,
    /// Clear all traffic
    Clear,
    /// Import traffic from a HAR file into a new session
    ImportHar(TrafficImportHarArgs),
    /// Show script execution traces for a traffic entry
    ScriptTraces(TrafficGetArgs),
}

impl TrafficCommands {
    pub async fn execute(&self, api_url: String) -> Result<()> {
        match self {
            TrafficCommands::List(args) => {
                let client = ApiClient::new(api_url);
                let mut query_parts: Vec<String> = Vec::new();

                if let Some(ref filter) = args.filter {
                    query_parts.push(format!("filter={}", filter));
                }
                if let Some(ref method) = args.method {
                    query_parts.push(format!("method={}", method));
                }
                if let Some(status) = args.status {
                    query_parts.push(format!("status={}", status));
                }
                query_parts.push(format!("limit={}", args.limit));

                let path = if query_parts.is_empty() {
                    "traffic".to_string()
                } else {
                    format!("traffic?{}", query_parts.join("&"))
                };

                let result = client.get(&path).await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    print_traffic_list(&result);
                }
            }
            TrafficCommands::Get(args) => {
                let client = ApiClient::new(api_url);
                let result = client.get(&format!("traffic/{}", args.id)).await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    print_traffic_detail(&result);
                }
            }
            TrafficCommands::Search(args) => {
                let client = ApiClient::new(api_url);
                let result = client
                    .get(&format!("traffic/search?q={}", args.query))
                    .await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    print_traffic_list(&result);
                }
            }
            TrafficCommands::Count => {
                let client = ApiClient::new(api_url);
                let result = client.get("traffic/count").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            TrafficCommands::Clear => {
                let client = ApiClient::new(api_url);
                let result = client.delete("traffic").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            TrafficCommands::ImportHar(args) => {
                let file_content = std::fs::read_to_string(&args.file).map_err(|e| {
                    anyhow::anyhow!("Failed to read HAR file '{}': {}", args.file, e)
                })?;
                let har: serde_json::Value = serde_json::from_str(&file_content)
                    .map_err(|e| anyhow::anyhow!("Failed to parse HAR JSON: {}", e))?;

                let body = serde_json::json!({
                    "har": har,
                    "session_name": args.name,
                    "switch_session": args.switch,
                });

                let client = ApiClient::new(api_url);
                let result = client.post("traffic/import/har", body).await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    let imported = result
                        .get("imported_count")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    let skipped = result
                        .get("skipped_count")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0);
                    let session_id = result
                        .get("session_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("-");
                    println!(
                        "Imported {} entries ({} skipped) into session {}",
                        imported, skipped, session_id
                    );
                    if let Some(errors) = result.get("errors").and_then(|v| v.as_array()) {
                        for err in errors {
                            if let Some(msg) = err.as_str() {
                                eprintln!("  - {}", msg);
                            }
                        }
                    }
                }
            }
            TrafficCommands::ScriptTraces(args) => {
                let client = ApiClient::new(api_url);
                let result = client
                    .get(&format!("traffic/{}/script-traces", args.id))
                    .await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }
        Ok(())
    }
}

fn print_traffic_list(result: &Value) {
    if let Some(entries) = result.as_array() {
        if entries.is_empty() {
            println!("No traffic entries found.");
            return;
        }
        println!("{:<36} {:<8} {:<6} {:<50}", "ID", "METHOD", "STATUS", "URL");
        println!("{}", "-".repeat(100));
        for entry in entries {
            let id = entry.get("id").and_then(|v| v.as_str()).unwrap_or("-");
            let method = entry.get("method").and_then(|v| v.as_str()).unwrap_or("-");
            let status = entry.get("status").and_then(|v| v.as_u64()).unwrap_or(0);
            let url = entry.get("url").and_then(|v| v.as_str()).unwrap_or("-");
            let url_truncated = if url.len() > 50 {
                format!("{}...", &url[..47])
            } else {
                url.to_string()
            };
            println!(
                "{:<36} {:<8} {:<6} {:<50}",
                id, method, status, url_truncated
            );
        }
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(result).unwrap_or_default()
        );
    }
}

fn print_traffic_detail(result: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(result).unwrap_or_default()
    );
}

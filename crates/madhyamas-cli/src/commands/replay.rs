//! Replay commands

use anyhow::Result;
use clap::{Args, Subcommand};
use serde_json::{json, Value};
use std::collections::HashMap;

use super::ApiClient;

#[derive(Debug, Args)]
pub struct ReplayRequestArgs {
    /// Saved request ID to replay
    pub id: String,

    /// Override the URL
    #[arg(long)]
    pub url: Option<String>,

    /// Override the HTTP method
    #[arg(long)]
    pub method: Option<String>,

    /// Header to add/replace (repeatable). Format: "Key: Value"
    #[arg(long = "header", value_name = "KEY: VALUE")]
    pub headers: Vec<String>,

    /// New request body (raw text)
    #[arg(long)]
    pub body: Option<String>,

    /// Read request body from a file
    #[arg(long, value_name = "PATH")]
    pub body_file: Option<String>,

    /// Follow redirect responses (3xx)
    #[arg(long)]
    pub follow_redirects: bool,

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

#[derive(Debug, Args)]
pub struct ReplayAdvancedArgs {
    /// Saved request ID to replay
    pub id: String,

    /// Total number of requests to send (max 10,000)
    #[arg(long, default_value_t = 1)]
    pub iterations: usize,

    /// Number of simultaneous in-flight requests (max 100)
    #[arg(long, default_value_t = 1)]
    pub concurrency: usize,

    /// Delay between requests in milliseconds
    #[arg(long)]
    pub delay_ms: Option<u64>,

    /// Override the URL
    #[arg(long)]
    pub url: Option<String>,

    /// Override the HTTP method
    #[arg(long)]
    pub method: Option<String>,

    /// Header to add/replace (repeatable). Format: "Key: Value"
    #[arg(long = "header", value_name = "KEY: VALUE")]
    pub headers: Vec<String>,

    /// New request body (raw text)
    #[arg(long)]
    pub body: Option<String>,

    /// Read request body from a file
    #[arg(long, value_name = "PATH")]
    pub body_file: Option<String>,

    /// Follow redirect responses (3xx)
    #[arg(long)]
    pub follow_redirects: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Subcommand)]
pub enum ReplayCommands {
    /// Replay a captured request
    Run(ReplayRequestArgs),
    /// Replay a captured request multiple times with concurrency and delay
    RunAdvanced(ReplayAdvancedArgs),
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
    /// Clear all replay history
    HistoryClear,
}

impl ReplayCommands {
    pub async fn execute(&self, api_url: String) -> Result<()> {
        match self {
            ReplayCommands::Run(args) => {
                let client = ApiClient::new(api_url);

                let mut modifications = json!({});
                let mut has_modifications = false;

                if let Some(ref url) = args.url {
                    modifications["url"] = Value::String(url.clone());
                    has_modifications = true;
                }

                if let Some(ref method) = args.method {
                    modifications["method"] = Value::String(method.to_uppercase());
                    has_modifications = true;
                }

                if !args.headers.is_empty() {
                    let mut headers = HashMap::new();
                    for header in &args.headers {
                        if let Some((name, value)) = header.split_once(':') {
                            headers.insert(name.trim().to_string(), value.trim().to_string());
                        }
                    }
                    modifications["headers"] = serde_json::to_value(&headers)?;
                    has_modifications = true;
                }

                if let Some(ref body) = args.body {
                    modifications["body"] = Value::String(body.clone());
                    has_modifications = true;
                }

                if let Some(ref path) = args.body_file {
                    let content = std::fs::read_to_string(path)
                        .map_err(|e| anyhow::anyhow!("Failed to read body file: {}", e))?;
                    modifications["body"] = Value::String(content);
                    has_modifications = true;
                }

                if args.follow_redirects {
                    modifications["follow_redirects"] = Value::Bool(true);
                    has_modifications = true;
                }

                let request_body = if has_modifications {
                    json!({ "modifications": modifications })
                } else {
                    json!({})
                };

                let result = client
                    .post(&format!("replay/execute/{}", args.id), request_body)
                    .await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    if let Some(error) = result.get("error").and_then(|v| v.as_str()) {
                        println!("Replay failed: {}", error);
                    } else if let Some(response) = result.get("response") {
                        let status = response
                            .get("status_code")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        let duration = result
                            .get("duration_ms")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        println!("Replay completed: {} ({}ms)", status, duration);
                    } else {
                        println!("Replay completed successfully");
                    }
                }
            }
            ReplayCommands::RunAdvanced(args) => {
                let client = ApiClient::new(api_url);

                let mut modifications = json!({});
                let mut has_modifications = false;

                if let Some(ref url) = args.url {
                    modifications["url"] = Value::String(url.clone());
                    has_modifications = true;
                }

                if let Some(ref method) = args.method {
                    modifications["method"] = Value::String(method.to_uppercase());
                    has_modifications = true;
                }

                if !args.headers.is_empty() {
                    let mut headers = HashMap::new();
                    for header in &args.headers {
                        if let Some((name, value)) = header.split_once(':') {
                            headers.insert(name.trim().to_string(), value.trim().to_string());
                        }
                    }
                    modifications["headers"] = serde_json::to_value(&headers)?;
                    has_modifications = true;
                }

                if let Some(ref body) = args.body {
                    modifications["body"] = Value::String(body.clone());
                    has_modifications = true;
                }

                if let Some(ref path) = args.body_file {
                    let content = std::fs::read_to_string(path)
                        .map_err(|e| anyhow::anyhow!("Failed to read body file: {}", e))?;
                    modifications["body"] = Value::String(content);
                    has_modifications = true;
                }

                if args.follow_redirects {
                    modifications["follow_redirects"] = Value::Bool(true);
                    has_modifications = true;
                }

                let mut request_body = json!({
                    "config": {
                        "iterations": args.iterations,
                        "concurrency": args.concurrency,
                    }
                });
                if let Some(delay) = args.delay_ms {
                    request_body["config"]["delay_ms"] = Value::Number(delay.into());
                }
                if has_modifications {
                    request_body["modifications"] = modifications;
                }

                let result = client
                    .post(&format!("replay/execute/{}/batch", args.id), request_body)
                    .await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    print_batch_result(&result);
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
            ReplayCommands::HistoryClear => {
                let client = ApiClient::new(api_url);
                client.delete_void("replay/history").await?;
                println!("Cleared all replay history.");
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

fn print_batch_result(result: &Value) {
    let total = result.get("total").and_then(|v| v.as_u64()).unwrap_or(0);
    let succeeded = result
        .get("succeeded")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let failed = result.get("failed").and_then(|v| v.as_u64()).unwrap_or(0);
    let min_ms = result.get("min_ms").and_then(|v| v.as_u64()).unwrap_or(0);
    let max_ms = result.get("max_ms").and_then(|v| v.as_u64()).unwrap_or(0);
    let avg_ms = result.get("avg_ms").and_then(|v| v.as_u64()).unwrap_or(0);
    let p95_ms = result.get("p95_ms").and_then(|v| v.as_u64()).unwrap_or(0);

    println!("Batch replay completed");
    println!("{}", "-".repeat(40));
    println!("Total:     {}", total);
    println!("Succeeded: {}", succeeded);
    println!("Failed:    {}", failed);
    println!("Latency (ms):");
    println!("  min: {}", min_ms);
    println!("  avg: {}", avg_ms);
    println!("  max: {}", max_ms);
    println!("  p95: {}", p95_ms);
}

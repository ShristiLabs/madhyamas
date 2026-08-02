//! Mirror tool commands

use anyhow::Result;
use clap::{Args, Subcommand};

use super::ApiClient;

#[derive(Debug, Args)]
pub struct MirrorStatusArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct MirrorConfigArgs {
    /// Enable or disable mirroring
    #[arg(long)]
    pub enabled: Option<bool>,

    /// Output directory for mirrored files
    #[arg(long)]
    pub output_dir: Option<String>,

    /// Comma-separated host filter patterns (use "none" to clear)
    #[arg(long)]
    pub host_filter: Option<String>,

    /// Whether to also save request bodies
    #[arg(long)]
    pub save_request_bodies: Option<bool>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Subcommand)]
pub enum MirrorCommands {
    /// Show current mirror status and statistics
    Status(MirrorStatusArgs),

    /// Start mirroring (enable the mirror tool)
    Start,

    /// Stop mirroring (disable the mirror tool)
    Stop,

    /// Update mirror configuration
    Config(MirrorConfigArgs),
}

impl MirrorCommands {
    pub async fn execute(&self, api_url: String) -> Result<()> {
        let client = ApiClient::new(api_url);

        match self {
            MirrorCommands::Status(args) => {
                let result = client.get("mirror").await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("Mirror Status");
                    println!("=============");
                    println!(
                        "Enabled:            {}",
                        if result["enabled"].as_bool().unwrap_or(false) {
                            "yes"
                        } else {
                            "no"
                        }
                    );
                    println!(
                        "Output Dir:         {}",
                        result["output_dir"].as_str().unwrap_or("(default)")
                    );
                    println!(
                        "Save Request Bodies:{}",
                        if result["save_request_bodies"].as_bool().unwrap_or(false) {
                            " yes"
                        } else {
                            " no"
                        }
                    );
                    if let Some(filter) = result["host_filter"].as_array() {
                        if filter.is_empty() {
                            println!("Host Filter:        (none — all hosts)");
                        } else {
                            let patterns: Vec<&str> =
                                filter.iter().filter_map(|v| v.as_str()).collect();
                            println!("Host Filter:        {}", patterns.join(", "));
                        }
                    } else {
                        println!("Host Filter:        (none — all hosts)");
                    }
                    println!(
                        "Files Written:      {}",
                        result["files_written"].as_u64().unwrap_or(0)
                    );
                    println!(
                        "Bytes Written:      {}",
                        format_bytes(result["bytes_written"].as_u64().unwrap_or(0))
                    );
                }
            }
            MirrorCommands::Start => {
                let result = client
                    .post("mirror/toggle", serde_json::json!({ "enabled": true }))
                    .await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            MirrorCommands::Stop => {
                let result = client
                    .post("mirror/toggle", serde_json::json!({ "enabled": false }))
                    .await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            MirrorCommands::Config(args) => {
                let mut payload = serde_json::Map::new();

                if let Some(enabled) = args.enabled {
                    payload.insert("enabled".to_string(), serde_json::Value::Bool(enabled));
                }
                if let Some(ref dir) = args.output_dir {
                    payload.insert(
                        "output_dir".to_string(),
                        serde_json::Value::String(dir.clone()),
                    );
                }
                if let Some(ref filter) = args.host_filter {
                    if filter.to_lowercase() == "none" {
                        payload.insert("host_filter".to_string(), serde_json::Value::Null);
                    } else {
                        let patterns: Vec<serde_json::Value> = filter
                            .split(',')
                            .map(|s| s.trim())
                            .filter(|s| !s.is_empty())
                            .map(|s| serde_json::Value::String(s.to_string()))
                            .collect();
                        payload.insert(
                            "host_filter".to_string(),
                            serde_json::Value::Array(patterns),
                        );
                    }
                }
                if let Some(save) = args.save_request_bodies {
                    payload.insert(
                        "save_request_bodies".to_string(),
                        serde_json::Value::Bool(save),
                    );
                }

                if payload.is_empty() {
                    println!("No mirror changes specified.");
                    println!(
                        "Use --enabled, --output-dir, --host-filter, or --save-request-bodies"
                    );
                    return Ok(());
                }

                let result = client
                    .patch("mirror/config", serde_json::Value::Object(payload))
                    .await?;

                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("Mirror configuration updated successfully");
                    println!(
                        "  Enabled:        {}",
                        if result["enabled"].as_bool().unwrap_or(false) {
                            "yes"
                        } else {
                            "no"
                        }
                    );
                    println!(
                        "  Output Dir:     {}",
                        result["output_dir"].as_str().unwrap_or("(default)")
                    );
                    if let Some(filter) = result["host_filter"].as_array() {
                        if filter.is_empty() {
                            println!("  Host Filter:    (none — all hosts)");
                        } else {
                            let patterns: Vec<&str> =
                                filter.iter().filter_map(|v| v.as_str()).collect();
                            println!("  Host Filter:    {}", patterns.join(", "));
                        }
                    } else {
                        println!("  Host Filter:    (none — all hosts)");
                    }
                    println!(
                        "  Save Requests:  {}",
                        if result["save_request_bodies"].as_bool().unwrap_or(false) {
                            "yes"
                        } else {
                            "no"
                        }
                    );
                }
            }
        }

        Ok(())
    }
}

/// Format a byte count as a human-readable string.
fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

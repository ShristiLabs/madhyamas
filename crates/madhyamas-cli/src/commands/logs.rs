//! Log rotation commands.

use anyhow::Result;
use clap::{Args, Subcommand};

use super::ApiClient;

#[derive(Debug, Args)]
pub struct LogsStatusArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct LogsConfigArgs {
    /// Enable or disable file logging
    #[arg(long)]
    pub enabled: Option<bool>,

    /// Rotation mode: never, hourly, daily, or size
    #[arg(long)]
    pub rotation: Option<String>,

    /// Size in MB (only used with --rotation size)
    #[arg(long)]
    pub size_mb: Option<u64>,

    /// Maximum number of archived log files to keep
    #[arg(long)]
    pub max_files: Option<usize>,

    /// Hard per-file size cap in MB (safety net for time-based rotation)
    #[arg(long)]
    pub max_file_size_mb: Option<u64>,

    /// Use structured JSON log format
    #[arg(long)]
    pub json_format: Option<bool>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Subcommand)]
pub enum LogsCommands {
    /// Show current log rotation status and archived files
    Status(LogsStatusArgs),

    /// Rotate the current log file immediately (on-demand)
    Rotate,

    /// Update log rotation configuration
    Config(LogsConfigArgs),
}

impl LogsCommands {
    pub async fn execute(&self, api_url: String) -> Result<()> {
        let client = ApiClient::new(api_url);

        match self {
            LogsCommands::Status(args) => {
                let result = client.get("logs").await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    print_log_status(&result);
                }
            }
            LogsCommands::Rotate => {
                let result = client.post("logs/rotate", serde_json::json!({})).await?;
                if let Some(rotated) = result.get("rotated_to").and_then(|v| v.as_str()) {
                    println!("Log rotated to: {}", rotated);
                } else {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
            }
            LogsCommands::Config(args) => {
                let mut payload = serde_json::Map::new();

                if let Some(v) = args.enabled {
                    payload.insert("enabled".to_string(), serde_json::Value::Bool(v));
                }
                if let Some(ref mode) = args.rotation {
                    let mode = mode.trim().to_lowercase();
                    let rotation = match mode.as_str() {
                        "never" => serde_json::json!({ "mode": "never" }),
                        "hourly" => serde_json::json!({ "mode": "hourly" }),
                        "daily" => serde_json::json!({ "mode": "daily" }),
                        "size" => {
                            let size_mb = args.size_mb.ok_or_else(|| {
                                anyhow::anyhow!("--size-mb is required when --rotation size")
                            })?;
                            serde_json::json!({ "mode": "size", "size_mb": size_mb })
                        }
                        other => anyhow::bail!(
                            "invalid rotation mode: {} (expected never|hourly|daily|size)",
                            other
                        ),
                    };
                    payload.insert("rotation".to_string(), rotation);
                }
                if let Some(v) = args.max_files {
                    payload.insert(
                        "max_files".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(v)),
                    );
                }
                if let Some(v) = args.max_file_size_mb {
                    payload.insert(
                        "max_file_size_mb".to_string(),
                        serde_json::Value::Number(serde_json::Number::from(v)),
                    );
                }
                if let Some(v) = args.json_format {
                    payload.insert("json_format".to_string(), serde_json::Value::Bool(v));
                }

                if payload.is_empty() {
                    println!("No log changes specified.");
                    println!(
                        "Use --enabled, --rotation, --max-files, --max-file-size-mb, or --json-format"
                    );
                    return Ok(());
                }

                let result = client
                    .patch("logs", serde_json::Value::Object(payload))
                    .await?;

                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("Log configuration updated successfully");
                    println!(
                        "  Enabled:         {}",
                        bool_label(result["enabled"].as_bool())
                    );
                    println!(
                        "  Rotation:        {}",
                        result["rotation"].as_str().unwrap_or("(unknown)")
                    );
                    println!(
                        "  Max Files:       {}",
                        result["max_files"].as_u64().unwrap_or(0)
                    );
                    println!(
                        "  Max File Size:   {} MB",
                        result["max_file_size_mb"].as_u64().unwrap_or(0)
                    );
                    println!(
                        "  JSON Format:     {}",
                        bool_label(result["json_format"].as_bool())
                    );
                    if let Some(msg) = result["message"].as_str() {
                        println!();
                        println!("  {}", msg);
                    }
                }
            }
        }

        Ok(())
    }
}

fn bool_label(v: Option<bool>) -> &'static str {
    match v {
        Some(true) => "yes",
        Some(false) => "no",
        None => "?",
    }
}

fn print_log_status(result: &serde_json::Value) {
    println!("Log Rotation Status");
    println!("===================");
    println!(
        "Enabled:          {}",
        bool_label(result["enabled"].as_bool())
    );
    println!(
        "Rotation:         {}",
        result["rotation"].as_str().unwrap_or("(unknown)")
    );
    println!(
        "Max Files:        {}",
        result["max_files"].as_u64().unwrap_or(0)
    );
    println!(
        "Max File Size:    {} MB",
        result["max_file_size_mb"].as_u64().unwrap_or(0)
    );
    println!(
        "JSON Format:      {}",
        bool_label(result["json_format"].as_bool())
    );
    println!(
        "Log Dir:          {}",
        result["log_dir"].as_str().unwrap_or("(default)")
    );
    if let Some(current) = result.get("current_file") {
        println!();
        println!("Current File:");
        println!(
            "  Path:           {}",
            current["path"].as_str().unwrap_or("?")
        );
        println!(
            "  Size:           {}",
            format_bytes(current["size_bytes"].as_u64().unwrap_or(0))
        );
    }
    if let Some(archived) = result.get("archived_files").and_then(|v| v.as_array()) {
        println!();
        println!("Archived Files ({}):", archived.len());
        if archived.is_empty() {
            println!("  (none)");
        } else {
            for f in archived {
                let name = f["name"].as_str().unwrap_or("?");
                let size = f["size_bytes"].as_u64().unwrap_or(0);
                println!("  {} ({})", name, format_bytes(size));
            }
        }
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

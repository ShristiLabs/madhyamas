//! Auto Save commands

use anyhow::Result;
use clap::{Args, Subcommand};

use super::ApiClient;

#[derive(Debug, Args)]
pub struct AutoSaveGetArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct AutoSaveUpdateArgs {
    /// Enable or disable Auto Save
    #[arg(long)]
    pub enabled: Option<bool>,

    /// Interval between snapshots in seconds
    #[arg(long)]
    pub interval_seconds: Option<u64>,

    /// Export format: "har" or "session"
    #[arg(long)]
    pub export_format: Option<String>,

    /// Output directory for backup files
    #[arg(long)]
    pub output_dir: Option<String>,

    /// Maximum number of backup files to keep
    #[arg(long)]
    pub max_backups: Option<usize>,

    /// Rotate session after this many requests (use 0 to disable)
    #[arg(long)]
    pub rotate_after_requests: Option<usize>,

    /// Rotate session after this many minutes (use 0 to disable)
    #[arg(long)]
    pub rotate_after_minutes: Option<u64>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct AutoSaveSnapshotArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Subcommand)]
pub enum AutoSaveCommands {
    /// Get current Auto Save configuration
    Get(AutoSaveGetArgs),

    /// Update Auto Save configuration
    Update(AutoSaveUpdateArgs),

    /// Trigger an immediate snapshot (save now)
    Snapshot(AutoSaveSnapshotArgs),
}

impl AutoSaveCommands {
    pub async fn execute(&self, api_url: String) -> Result<()> {
        let client = ApiClient::new(api_url);

        match self {
            AutoSaveCommands::Get(args) => {
                let result = client.get("autosave").await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("Auto Save Configuration");
                    println!("========================");
                    println!(
                        "Enabled:          {}",
                        if result["enabled"].as_bool().unwrap_or(false) {
                            "yes"
                        } else {
                            "no"
                        }
                    );
                    println!(
                        "Interval:         {}s",
                        result["interval_seconds"].as_u64().unwrap_or(300)
                    );
                    println!(
                        "Format:           {}",
                        result["export_format"].as_str().unwrap_or("har")
                    );
                    println!(
                        "Output Dir:       {}",
                        result["output_dir"].as_str().unwrap_or("(default)")
                    );
                    println!(
                        "Max Backups:      {}",
                        result["max_backups"].as_u64().unwrap_or(10)
                    );
                    println!(
                        "Rotate Requests:  {}",
                        result["rotate_after_requests"]
                            .as_u64()
                            .map(|v| v.to_string())
                            .unwrap_or_else(|| "(disabled)".to_string())
                    );
                    println!(
                        "Rotate Minutes:   {}",
                        result["rotate_after_minutes"]
                            .as_u64()
                            .map(|v| v.to_string())
                            .unwrap_or_else(|| "(disabled)".to_string())
                    );
                }
            }
            AutoSaveCommands::Update(args) => {
                let mut payload = serde_json::Map::new();

                if let Some(enabled) = args.enabled {
                    payload.insert("enabled".to_string(), serde_json::Value::Bool(enabled));
                }
                if let Some(interval) = args.interval_seconds {
                    payload.insert(
                        "interval_seconds".to_string(),
                        serde_json::Value::Number(interval.into()),
                    );
                }
                if let Some(ref format) = args.export_format {
                    payload.insert(
                        "export_format".to_string(),
                        serde_json::Value::String(format.clone()),
                    );
                }
                if let Some(ref dir) = args.output_dir {
                    payload.insert(
                        "output_dir".to_string(),
                        serde_json::Value::String(dir.clone()),
                    );
                }
                if let Some(max) = args.max_backups {
                    payload.insert(
                        "max_backups".to_string(),
                        serde_json::Value::Number(max.into()),
                    );
                }
                if let Some(reqs) = args.rotate_after_requests {
                    if reqs == 0 {
                        payload
                            .insert("rotate_after_requests".to_string(), serde_json::Value::Null);
                    } else {
                        payload.insert(
                            "rotate_after_requests".to_string(),
                            serde_json::Value::Number(reqs.into()),
                        );
                    }
                }
                if let Some(mins) = args.rotate_after_minutes {
                    if mins == 0 {
                        payload.insert("rotate_after_minutes".to_string(), serde_json::Value::Null);
                    } else {
                        payload.insert(
                            "rotate_after_minutes".to_string(),
                            serde_json::Value::Number(mins.into()),
                        );
                    }
                }

                if payload.is_empty() {
                    println!("No Auto Save changes specified.");
                    println!("Use --enabled, --interval-seconds, --export-format, --output-dir, --max-backups, --rotate-after-requests, or --rotate-after-minutes");
                    return Ok(());
                }

                let result = client
                    .patch("autosave", serde_json::Value::Object(payload))
                    .await?;

                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("✅ Auto Save configuration updated successfully");
                    println!();
                    println!("Current Auto Save Configuration:");
                    println!(
                        "  Enabled:        {}",
                        if result["enabled"].as_bool().unwrap_or(false) {
                            "yes"
                        } else {
                            "no"
                        }
                    );
                    println!(
                        "  Interval:       {}s",
                        result["interval_seconds"].as_u64().unwrap_or(300)
                    );
                    println!(
                        "  Format:         {}",
                        result["export_format"].as_str().unwrap_or("har")
                    );
                    println!(
                        "  Output Dir:     {}",
                        result["output_dir"].as_str().unwrap_or("(default)")
                    );
                    println!(
                        "  Max Backups:    {}",
                        result["max_backups"].as_u64().unwrap_or(10)
                    );
                }
            }
            AutoSaveCommands::Snapshot(args) => {
                let result = client
                    .post("autosave/snapshot", serde_json::Value::Null)
                    .await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("✅ Snapshot saved successfully");
                    if let Some(dir) = result["output_dir"].as_str() {
                        println!("  Output directory: {}", dir);
                    }
                }
            }
        }

        Ok(())
    }
}

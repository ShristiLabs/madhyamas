//! Capture mode commands (recording vs passthrough)

use anyhow::Result;
use clap::{Args, Subcommand};

use super::ApiClient;

#[derive(Debug, Args)]
pub struct CaptureStatusArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Subcommand)]
pub enum CaptureCommands {
    /// Get current capture status (recording or passthrough)
    Status(CaptureStatusArgs),
    /// Toggle capture mode (recording <-> passthrough)
    Toggle,
    /// Enable traffic recording
    Enable,
    /// Disable traffic recording (passthrough mode)
    Disable,
}

impl CaptureCommands {
    pub async fn execute(&self, api_url: String, auth: super::CliAuth) -> Result<()> {
        let client = ApiClient::new(api_url, auth.clone());

        match self {
            CaptureCommands::Status(args) => {
                let result = client.get("capture").await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    let enabled = result["capture_enabled"].as_bool().unwrap_or(true);
                    let mode = result["mode"].as_str().unwrap_or("unknown");
                    if enabled {
                        println!("📹 Capture Mode: {} (traffic is being recorded)", mode);
                    } else {
                        println!("⏸️  Capture Mode: {} (traffic is NOT being recorded)", mode);
                    }
                }
            }
            CaptureCommands::Toggle => {
                let result = client.post("capture/toggle", serde_json::json!({})).await?;
                let enabled = result["capture_enabled"].as_bool().unwrap_or(true);
                let mode = result["mode"].as_str().unwrap_or("unknown");
                if enabled {
                    println!("✅ Capture enabled: {} (traffic will be recorded)", mode);
                } else {
                    println!("⏸️  Capture disabled: {} (passthrough mode)", mode);
                }
            }
            CaptureCommands::Enable => {
                // First check current status
                let status = client.get("capture").await?;
                let currently_enabled = status["capture_enabled"].as_bool().unwrap_or(true);

                if currently_enabled {
                    println!("ℹ️  Capture is already enabled");
                } else {
                    // Toggle to enable
                    let result = client.post("capture/toggle", serde_json::json!({})).await?;
                    let mode = result["mode"].as_str().unwrap_or("recording");
                    println!("✅ Capture enabled: {} (traffic will be recorded)", mode);
                }
            }
            CaptureCommands::Disable => {
                // First check current status
                let status = client.get("capture").await?;
                let currently_enabled = status["capture_enabled"].as_bool().unwrap_or(true);

                if !currently_enabled {
                    println!("ℹ️  Capture is already disabled (passthrough mode)");
                } else {
                    // Toggle to disable
                    let result = client.post("capture/toggle", serde_json::json!({})).await?;
                    let mode = result["mode"].as_str().unwrap_or("passthrough");
                    println!("⏸️  Capture disabled: {} (passthrough mode)", mode);
                }
            }
        }

        Ok(())
    }
}

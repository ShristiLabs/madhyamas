//! Throttle commands (bandwidth/latency limiting)

use anyhow::Result;
use clap::{Args, Subcommand};
use serde_json::{json, Value};

use super::ApiClient;

#[derive(Debug, Args)]
pub struct ThrottleSetArgs {
    /// Download bandwidth limit in bytes per second
    #[arg(long)]
    pub download_bps: Option<u64>,

    /// Upload bandwidth limit in bytes per second
    #[arg(long)]
    pub upload_bps: Option<u64>,

    /// Added latency in milliseconds
    #[arg(long)]
    pub delay_ms: Option<u64>,

    /// Optional name to save/load as a preset
    #[arg(short, long)]
    pub name: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum ThrottleCommands {
    /// Get the current throttle profile
    Get,
    /// Set throttle parameters
    Set(ThrottleSetArgs),
    /// Enable throttling
    Enable,
    /// Disable throttling
    Disable,
    /// List available throttle presets
    Presets,
}

impl ThrottleCommands {
    pub async fn execute(&self, api_url: String, auth: super::CliAuth) -> Result<()> {
        let client = ApiClient::new(api_url, auth.clone());

        match self {
            ThrottleCommands::Get => {
                let result = client.get("throttle").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            ThrottleCommands::Set(args) => {
                let mut body = json!({});
                if let Some(d) = args.download_bps {
                    body["download_bps"] = Value::Number(d.into());
                }
                if let Some(u) = args.upload_bps {
                    body["upload_bps"] = Value::Number(u.into());
                }
                if let Some(delay) = args.delay_ms {
                    body["delay_ms"] = Value::Number(delay.into());
                }
                if let Some(ref name) = args.name {
                    body["name"] = Value::String(name.clone());
                }
                let result = client.post("throttle", body).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            ThrottleCommands::Enable => {
                let result = client
                    .post("throttle/enabled", json!({ "enabled": true }))
                    .await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            ThrottleCommands::Disable => {
                let result = client
                    .post("throttle/enabled", json!({ "enabled": false }))
                    .await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            ThrottleCommands::Presets => {
                let result = client.get("throttle/presets").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }

        Ok(())
    }
}

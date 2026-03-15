//! Configuration commands

use anyhow::Result;
use clap::{Args, Subcommand};

use super::ApiClient;

#[derive(Debug, Args)]
pub struct ConfigGetArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct ConfigUpdateArgs {
    /// Enable or disable HTTPS interception
    #[arg(long)]
    pub intercept_https: Option<bool>,

    /// Maximum number of requests to keep in memory
    #[arg(long)]
    pub max_requests: Option<usize>,

    /// Enable or disable verbose logging
    #[arg(long)]
    pub verbose: Option<bool>,

    /// Public IP address to display (use "null" to clear)
    #[arg(long)]
    pub public_ip: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommands {
    /// Get current proxy configuration
    Get(ConfigGetArgs),

    /// Update runtime configuration
    Update(ConfigUpdateArgs),
}

impl ConfigCommands {
    pub async fn execute(&self, api_url: String) -> Result<()> {
        let client = ApiClient::new(api_url);

        match self {
            ConfigCommands::Get(args) => {
                let result = client.get("config").await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("Madhyamas Configuration");
                    println!("========================");
                    println!(
                        "Proxy Port:       {}",
                        result["proxy_port"].as_u64().unwrap_or(8888)
                    );
                    println!(
                        "API Port:         {}",
                        result["api_port"].as_u64().unwrap_or(3001)
                    );
                    println!(
                        "Host:             {}",
                        result["host"].as_str().unwrap_or("127.0.0.1")
                    );
                    println!(
                        "Public IP:        {}",
                        result["public_ip"].as_str().unwrap_or("(auto-detect)")
                    );
                    println!(
                        "HTTPS Intercept:  {}",
                        if result["intercept_https"].as_bool().unwrap_or(true) {
                            "enabled"
                        } else {
                            "disabled"
                        }
                    );
                    println!(
                        "Max Requests:     {}",
                        result["max_requests"].as_u64().unwrap_or(10000)
                    );
                }
            }
            ConfigCommands::Update(args) => {
                // Build the update payload
                let mut payload = serde_json::Map::new();

                if let Some(intercept) = args.intercept_https {
                    payload.insert(
                        "intercept_https".to_string(),
                        serde_json::Value::Bool(intercept),
                    );
                }

                if let Some(max_req) = args.max_requests {
                    payload.insert(
                        "max_requests".to_string(),
                        serde_json::Value::Number(max_req.into()),
                    );
                }

                if let Some(verbose) = args.verbose {
                    payload.insert("verbose".to_string(), serde_json::Value::Bool(verbose));
                }

                if let Some(ref ip) = args.public_ip {
                    if ip == "null" || ip.is_empty() {
                        payload.insert("public_ip".to_string(), serde_json::Value::Null);
                    } else {
                        payload.insert(
                            "public_ip".to_string(),
                            serde_json::Value::String(ip.clone()),
                        );
                    }
                }

                if payload.is_empty() {
                    println!("No configuration changes specified.");
                    println!("Use --intercept-https, --max-requests, --verbose, or --public-ip");
                    return Ok(());
                }

                let result = client
                    .patch("config", serde_json::Value::Object(payload))
                    .await?;

                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("✅ Configuration updated successfully");
                    println!();
                    println!("Current Configuration:");
                    println!(
                        "  HTTPS Intercept: {}",
                        if result["intercept_https"].as_bool().unwrap_or(true) {
                            "enabled"
                        } else {
                            "disabled"
                        }
                    );
                    println!(
                        "  Max Requests:    {}",
                        result["max_requests"].as_u64().unwrap_or(10000)
                    );
                    println!(
                        "  Verbose:         {}",
                        if result["verbose"].as_bool().unwrap_or(false) {
                            "enabled"
                        } else {
                            "disabled"
                        }
                    );
                    println!(
                        "  Public IP:       {}",
                        result["public_ip"].as_str().unwrap_or("(auto-detect)")
                    );
                }
            }
        }

        Ok(())
    }
}

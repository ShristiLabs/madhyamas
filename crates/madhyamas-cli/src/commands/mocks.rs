//! Mock response commands

use anyhow::Result;
use clap::{Args, Subcommand};
use serde_json::{json, Value};

use super::ApiClient;

#[derive(Debug, Args)]
pub struct MockCreateArgs {
    /// URL pattern to match (supports wildcards: */api.example.com/*)
    #[arg(short, long)]
    pub url_pattern: String,

    /// HTTP method to match
    #[arg(short, long)]
    pub method: Option<String>,

    /// Response status code
    #[arg(short, long)]
    pub status_code: Option<u16>,

    /// Response body
    #[arg(short, long)]
    pub body: Option<String>,

    /// Response delay in milliseconds
    #[arg(short, long)]
    pub delay_ms: Option<u64>,

    /// Enable or disable
    #[arg(short, long)]
    pub enabled: Option<bool>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct MockDeleteArgs {
    /// Mock ID
    pub id: String,
}

#[derive(Debug, Args)]
pub struct MockToggleArgs {
    /// Mock ID
    pub id: String,

    /// Enable or disable
    pub enabled: bool,
}

#[derive(Debug, Subcommand)]
pub enum MockCommands {
    /// List all mock rules
    List,
    /// Create a mock rule
    Create(MockCreateArgs),
    /// Delete a mock rule
    Delete(MockDeleteArgs),
    /// Toggle mock rule on/off
    Toggle(MockToggleArgs),
}

impl MockCommands {
    pub async fn execute(&self, api_url: String) -> Result<()> {
        match self {
            MockCommands::List => {
                let client = ApiClient::new(api_url);
                let result = client.get("mocks").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            MockCommands::Create(args) => {
                let client = ApiClient::new(api_url);
                let mut body = json!({
                    "url_pattern": args.url_pattern,
                });
                if let Some(ref m) = args.method {
                    body["method"] = Value::String(m.clone());
                }
                if let Some(s) = args.status_code {
                    body["status_code"] = Value::Number(s.into());
                }
                if let Some(ref b) = args.body {
                    body["body"] = Value::String(b.clone());
                }
                if let Some(d) = args.delay_ms {
                    body["delay_ms"] = Value::Number(d.into());
                }
                if let Some(e) = args.enabled {
                    body["enabled"] = Value::Bool(e);
                }
                let result = client.post("mocks", body).await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    let id = result.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                    println!("Created mock with ID: {}", id);
                }
            }
            MockCommands::Delete(args) => {
                let client = ApiClient::new(api_url);
                let result = client.delete(&format!("mocks/{}", args.id)).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            MockCommands::Toggle(args) => {
                let client = ApiClient::new(api_url);
                let body = json!({ "enabled": args.enabled });
                let result = client
                    .post(&format!("mocks/{}/toggle", args.id), body)
                    .await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }
        Ok(())
    }
}

//! Mock response commands
use anyhow::Result;
use clap::{Args, Subcommand};
use serde_json::{json, Value};
use super::ApiClient;
#[derive(Debug, Args)]
pub struct MockCreateArgs {
    /// URL pattern to match (supports wildcards: */api.example.com/*)
    #[arg(short, long)]
    url_pattern: String,
    /// HTTP method to match
    #[arg(short, long)]
    method: Option<String>,
    /// Response status code
    #[arg(short, long)]
    status_code: Option<u16>,
    /// Response headers (JSON)
    #[arg(short = long)]
    headers: Option<Value>,
    /// Response body (JSON)
    #[arg(short, long)]
    body: Option<Value>,
    /// Response delay in milliseconds
    #[arg(short, long)]
    delay_ms: Option<u64>,
    /// Enable or disable
    #[arg(short = long)]
    enabled: Option<bool>,
    /// Output as JSON
    #[arg(long)]
    json: bool,
}
#[derive(Debug, Args)]
pub struct MockDeleteArgs {
    /// Mock ID
    id: String,
}
#[derive(Debug, Args)]
pub struct MockToggleArgs {
    /// Mock ID
    id: String,
    /// Enable or disable
    enabled: bool,
}
#[derive(Debug, Subcommand)]
pub enum MockCommands {
    /// List all mock rules
    List,
    /// Create a mock rule
    create(MockCreateArgs),
    /// Delete a mock rule
    delete(MockDeleteArgs),
    /// Toggle mock rule on/off
    toggle(MockToggleArgs),
}
impl MockCommands {
    pub async fn execute(&self, api_url: String) -> Result<()> {
        match self {
            MockCommands::List => {
                let client = ApiClient::new(api_url);
                let result = client.get("mocks").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            MockCommands::create(args) => {
                let client = ApiClient::new(api_url);
                let mut body = json!({
                    "url_pattern": args.url_pattern,
                });
                if let Some(m) = args.method {
                    body["method"] = Value::String(m);
                }
                if let Some(s) = args.status_code {
                    body["status_code"] = Value::Number(s.into());
                }
                if let Some(h) = args.headers {
                    body["headers"] = h;
                }
                if let Some(b) = args.body {
                    body["body"] = b;
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
                    let id = result.get("id").and_then(|v| v.as_str()).unwrap_or("?").unwrap_or_default();
                    println!("Created mock: {} (id: {})", id);
                }
            }
            MockCommands::delete(args) => {
                let client = ApiClient::new(api_url);
                let result = client.delete(&format!("mocks/{}", args.id)).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            MockCommands::toggle(args) => {
                let client = ApiClient::new(api_url);
                let body = json!({ "enabled": args.enabled });
                let result = client.post(&format!("mocks/{}/toggle", body).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }
    }
}

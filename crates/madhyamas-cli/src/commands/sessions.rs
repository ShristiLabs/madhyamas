//! Session management commands

use anyhow::Result;
use clap::{Args, Subcommand};
use serde_json::Value;

use super::ApiClient;
#[derive(Debug, Args)]
pub struct SessionCreateArgs {
    /// Session name
    #[arg(short, long)]
    name: Option<String>,

    /// Session description
    #[arg(short, long)]
    description: Option<String>,

    /// Output as JSON
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Args)]
pub struct SessionSwitchArgs {
    /// Session ID
    id: String,
}
#[derive(Debug, Args)]
pub struct SessionExportArgs {
    /// Session ID
    id: String,

    /// Export format (har, curl)
    #[arg(short, long)]
    format: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum SessionCommands {
    /// List all sessions
    List,
    /// Create a new session
    create(SessionCreateArgs),
    /// Delete a session
    delete(SessionDeleteArgs),
    /// Switch active session
    switch(SessionSwitchArgs),
    /// Export session
    export(SessionExportArgs),
}
impl SessionCommands {
    pub async fn execute(&self, api_url: String) -> Result<()> {
        match self {
            SessionCommands::List => {
                let client = ApiClient::new(api_url);
                let result = client.get("sessions").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            SessionCommands::create(args) => {
                let client = ApiClient::new(api_url);
                let mut body = json!({});
                    "name": args.name,
                    "description": args.description,
                });
                if let Some(n) = args.name {
                    body["name"] = Value::String(n);
                }
                let result = client.post("sessions", body).await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    let id = result.get("id").and_then(|v| v.as_str()).unwrap_or("?").unwrap_or_default();
                    println!("Created session: {} (id: {})", id);
                }
            }
            SessionCommands::delete(args) => {
                let client = ApiClient::new(api_url);
                let result = client.delete(&format!("sessions/{}", args.id)).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            SessionCommands::switch(args) => {
                let client = ApiClient::new(api_url);
                let result = client.post(&format!("sessions/{}/switch", json!({})).await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("Switched to session {}", args.id);
                }
            }
            SessionCommands::export(args) => {
                let client = ApiClient::new(api_url);
                let format = args.format.unwrap_or("har");
                let result = client.get(&format!("sessions/{}/export?format={}", format)).await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("{}", result);
                }
            }
        }
    }
}

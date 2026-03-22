//! Session management commands

use anyhow::Result;
use clap::{Args, Subcommand};
use serde_json::{json, Value};

use super::ApiClient;

#[derive(Debug, Args)]
pub struct SessionCreateArgs {
    /// Session name
    #[arg(short, long)]
    pub name: Option<String>,

    /// Session description
    #[arg(short, long)]
    pub description: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct SessionDeleteArgs {
    /// Session ID
    pub id: String,
}

#[derive(Debug, Args)]
pub struct SessionSwitchArgs {
    /// Session ID
    pub id: String,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct SessionExportArgs {
    /// Session ID
    pub id: String,

    /// Export format (har, curl)
    #[arg(short, long, default_value = "har")]
    pub format: String,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Subcommand)]
pub enum SessionCommands {
    /// List all sessions
    List,
    /// Create a new session
    Create(SessionCreateArgs),
    /// Delete a session
    Delete(SessionDeleteArgs),
    /// Switch active session
    Switch(SessionSwitchArgs),
    /// Export session
    Export(SessionExportArgs),
}

impl SessionCommands {
    pub async fn execute(&self, api_url: String) -> Result<()> {
        match self {
            SessionCommands::List => {
                let client = ApiClient::new(api_url);
                let result = client.get("sessions").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            SessionCommands::Create(args) => {
                let client = ApiClient::new(api_url);
                let mut body = json!({});
                if let Some(ref n) = args.name {
                    body["name"] = Value::String(n.clone());
                }
                if let Some(ref d) = args.description {
                    body["description"] = Value::String(d.clone());
                }
                let result = client.post("sessions", body).await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    let id = result.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                    println!("Created session with ID: {}", id);
                }
            }
            SessionCommands::Delete(args) => {
                let client = ApiClient::new(api_url);
                let result = client.delete(&format!("sessions/{}", args.id)).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            SessionCommands::Switch(args) => {
                let client = ApiClient::new(api_url);
                let result = client
                    .post(&format!("sessions/{}/switch", args.id), json!({}))
                    .await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("Switched to session {}", args.id);
                }
            }
            SessionCommands::Export(args) => {
                let client = ApiClient::new(api_url);
                let result = client
                    .get(&format!("sessions/{}/export?format={}", args.id, args.format))
                    .await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
            }
        }
        Ok(())
    }
}

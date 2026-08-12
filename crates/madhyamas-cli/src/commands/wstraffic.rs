//! WebSocket traffic inspection commands

use anyhow::Result;
use clap::{Args, Subcommand};
use serde_json::json;

use super::ApiClient;

#[derive(Debug, Args)]
pub struct WsConnectionArgs {
    /// WebSocket connection ID
    pub id: String,
}

#[derive(Debug, Args)]
pub struct WsMessagesArgs {
    /// Filter by connection ID
    #[arg(long)]
    pub connection_id: Option<String>,

    /// Filter by direction (send, receive)
    #[arg(long)]
    pub direction: Option<String>,

    /// Filter by message type (text, binary, ping, pong, close)
    #[arg(long)]
    pub message_type: Option<String>,

    /// Search in message payloads
    #[arg(long)]
    pub search: Option<String>,

    /// Maximum number of results
    #[arg(long)]
    pub limit: Option<usize>,

    /// Offset for pagination
    #[arg(long)]
    pub offset: Option<usize>,
}

#[derive(Debug, Subcommand)]
pub enum WsTrafficCommands {
    /// List all WebSocket connections
    Connections,
    /// Get details of a specific WebSocket connection
    Connection(WsConnectionArgs),
    /// List WebSocket messages with optional filtering
    Messages(WsMessagesArgs),
    /// Clear all WebSocket traffic (messages and closed connections)
    Clear,
}

impl WsTrafficCommands {
    pub async fn execute(&self, api_url: String) -> Result<()> {
        let client = ApiClient::new(api_url);

        match self {
            WsTrafficCommands::Connections => {
                let result = client.get("ws-traffic/connections").await?;
                if let Some(conns) = result.as_array() {
                    if conns.is_empty() {
                        println!("No WebSocket connections.");
                        return Ok(());
                    }
                    println!(
                        "{:<36}  {:<24}  {:<8}  {:<6}  DIRECTION",
                        "ID", "HOST", "STATE", "MSGS"
                    );
                    println!("{}", "-".repeat(90));
                    for conn in conns {
                        let id = conn.get("id").and_then(|v| v.as_str()).unwrap_or("-");
                        let host = conn.get("host").and_then(|v| v.as_str()).unwrap_or("-");
                        let state = conn.get("state").and_then(|v| v.as_str()).unwrap_or("?");
                        let msg_count = conn
                            .get("message_count")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        let direction = conn
                            .get("direction")
                            .and_then(|v| v.as_str())
                            .unwrap_or("?");
                        println!(
                            "{:<36}  {:<24}  {:<8}  {:<6}  {}",
                            id, host, state, msg_count, direction
                        );
                    }
                } else {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
            }
            WsTrafficCommands::Connection(args) => {
                let result = client
                    .get(&format!("ws-traffic/connections/{}", args.id))
                    .await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            WsTrafficCommands::Messages(args) => {
                let mut params = Vec::new();
                if let Some(ref cid) = args.connection_id {
                    params.push(format!("connection_id={}", cid));
                }
                if let Some(ref d) = args.direction {
                    params.push(format!("direction={}", d));
                }
                if let Some(ref mt) = args.message_type {
                    params.push(format!("message_type={}", mt));
                }
                if let Some(ref s) = args.search {
                    params.push(format!("search={}", s));
                }
                if let Some(l) = args.limit {
                    params.push(format!("limit={}", l));
                }
                if let Some(o) = args.offset {
                    params.push(format!("offset={}", o));
                }
                let path = if params.is_empty() {
                    "ws-traffic/messages".to_string()
                } else {
                    format!("ws-traffic/messages?{}", params.join("&"))
                };
                let result = client.get(&path).await?;
                if let Some(msgs) = result.as_array() {
                    if msgs.is_empty() {
                        println!("No WebSocket messages.");
                        return Ok(());
                    }
                    println!(
                        "{:<36}  {:<8}  {:<8}  {:<10}  SIZE",
                        "ID", "DIRECTION", "TYPE", "TIMESTAMP"
                    );
                    println!("{}", "-".repeat(80));
                    for msg in msgs {
                        let id = msg.get("id").and_then(|v| v.as_str()).unwrap_or("-");
                        let direction =
                            msg.get("direction").and_then(|v| v.as_str()).unwrap_or("?");
                        let mtype = msg
                            .get("message_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("?");
                        let ts = msg.get("timestamp").and_then(|v| v.as_str()).unwrap_or("?");
                        let size = msg.get("size").and_then(|v| v.as_u64()).unwrap_or(0);
                        let ts_short = if ts.len() >= 10 { &ts[..10] } else { ts };
                        println!(
                            "{:<36}  {:<8}  {:<8}  {:<10}  {}",
                            id, direction, mtype, ts_short, size
                        );
                    }
                } else {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
            }
            WsTrafficCommands::Clear => {
                let _ = client.post("ws-traffic/clear", json!({})).await?;
                println!("Cleared all WebSocket traffic.");
            }
        }

        Ok(())
    }
}

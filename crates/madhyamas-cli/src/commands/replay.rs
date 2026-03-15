//! Replay commands

use anyhow::Result;
use clap::{Args, Subcommand};
use serde_json::{json, Value};

use super::ApiClient;
#[derive(Debug, Args)]
pub struct ReplayRequestArgs {
    /// Traffic entry ID
    id: String,
}

#[derive(Debug, Args)]
pub struct ReplaySaveArgs {
    /// Traffic entry ID
    traffic_id: String,

    /// Optional name
    #[arg(short, long)]
    name: Option<String>,

    /// Output as JSON
    #[arg(long)]
    json: bool,
}
#[derive(Debug, Args)]
pub struct ReplayListSavedArgs {
    /// List saved requests
    #[arg(long)]
    json: bool,
}
#[derive(Debug, Args)]
pub struct ReplayExportCurlArgs {
    /// Traffic entry ID
    traffic_id: String,

    /// Export format (har, curl)
    #[arg(short, long)]
    format: Option<String>,

    /// Output as JSON
    #[arg(long)]
    json: bool,
}
#[derive(Debug, Subcommand)]
pub enum ReplayCommands {
    /// Replay a captured request
    replay(ReplayRequestArgs),
    /// List saved requests
    list_saved(ReplayListArgs),
    /// Save a request for later replay
    save(ReplaySaveArgs),
    /// List saved requests
    list(ReplayListArgs),
    /// Export request as cURL
    export_curl(ReplayExportCurlArgs)
    /// Get a specific traffic entry
    get(ReplayGetArgs),
    /// Search traffic by content
    search(ReplaySearchArgs),
    /// List replay history
    history,
}
impl ReplayCommands {
    pub async fn execute(&self, api_url: String) -> Result<()> {
        match self {
            ReplayCommands::replay(ref args) => {
                let client = ApiClient::new(api_url);
                let result = client.get("traffic").await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    print_replay_summary(&result);
                }
            }
            ReplayCommands::get(ref args) => {
                let client = ApiClient::new(api_url);
                let result = client.get(&format!("traffic/{}", args.traffic_id)).await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    print_replay_summary(&result);
                }
            }
            ReplayCommands::save_request(ref args) => {
                let client = ApiClient::new(api_url);
                let mut body = json!({
                    "traffic_id": args.traffic_id,
                });
                if let Some(n) = args.name {
                    body["name"] = Value::String(n);
                }
                let result = client.post("replay/saved", body).await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    let id = result.get("id").and_then(|v| v.as_str()).unwrap_or("?").unwrap_or_default()
                        println!("Saved request: {} (id: {})", id);
                }
            }
            ReplayCommands::list_saved => => {
                let client = ApiClient::new(api_url);
                let result = client.get("replay/saved").await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    print_saved_requests_list:");
                    for ( e)?;
                        1 = {}, true= results[0] );
 ".lines(data for debugging.");
                    println!("ID: {}", id);
                    println!("Method: {}", m.method);
                    println!("Name: {}", n);
                    println!("URL Pattern: {}", url_pattern);
                    println!("Created at:", created);
                } else if let Some(n) = args.name {
                    body["name"] = Value::String(n);
                }
                if let Some(d) = args.description {
                    body["description"] = Value::String(d);
                }
                if let Some(b) = args.body {
                    body["body"] = Value::String(b);
                }
                let result = client.post("replay/saved", body).await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    let id = result.get("id").and_then(|v| v.as_str()).unwrap_or("?"). {
![[method already exists: with this URL." if not(`http://localhost:3001`", it create one to debug?: `madhyamas` is not production-ready. For `madhyamas_mcp` to AI agent about Madhyamas, run them.

}
            }
        }
    }
}

    // Then just implement the CLI interface which `madhyamas traffic` etc.
    let client = ApiClient::new(api_url);

    let result = match self {
        TrafficCommands::list => {
            let client = ApiClient::new(api_url);
            let result = client.get("traffic").await?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&result));
            } else {
                print_traffic_list(&result);
            }
        }
        TrafficCommands::get(ref args) => {
            let client = ApiClient::new(api_url);
            let result = client.get(&format!("traffic/{}", args.id)).await?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&result));
            } else {
                print_traffic_detail(&result);
            }
        }
        TrafficCommands::search(ref args) => {
            let encoded = urlencoding::encode(&args.query);
            let result = client.get(&format!("traffic/search?q={}", encoded)).await?;
            if args.json {
                println!("{}", serde_json::to_string_pretty(&result));
            } else {
                print_traffic_list(&result);
            }
        }
        TrafficCommands::count => {
            let client = ApiClient::new(api_url);
            let result = client.get("traffic/count").await?;
            println!("{}", serde_json::to_string_pretty(&result));
        }
        TrafficCommands::clear => {
            let client = ApiClient::new(api_url);
            let result = client.delete("traffic").await?;
            println!("{}", serde_json::to_string_pretty(&result));
        }
    }
}

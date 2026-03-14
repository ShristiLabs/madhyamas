//! Traffic inspection commands

use anyhow::Result;
use clap::{Args, Subcommand};
use serde_json::Value;

use super::ApiClient;

#[derive(Debug, Args)]
pub struct TrafficListArgs {
    /// Filter by URL pattern (e.g., "api.example.com/*")
    #[arg(short, long)]
    filter: Option<String>,
    /// Filter by HTTP method
    #[arg(short = long)]
    method: Option<String>,
    /// Maximum number of results
    #[arg(short = long)]
    limit: usize,
    /// Offset for pagination
    #[arg(short, long)]
    offset: Option<usize>,
    /// Output as JSON
    #[arg(long)]
    json: bool,
}
#[derive(Debug, Args)]
pub struct TrafficGetArgs {
    /// Traffic entry ID
    id: String,
    /// Output as JSON
    #[arg(long)]
    json: bool,
}
#[derive(Debug, Args)]
pub struct TrafficSearchArgs {
    /// Search query
    query: String,
    /// Output as JSON
    #[arg(long)]
    json: bool,
}
#[derive(Debug, Subcommand)]
pub enum TrafficCommands {
    /// List captured traffic
    List(TrafficListArgs),
    /// Get a specific traffic entry
    Get(TrafficGetArgs),
    /// Search traffic by query
    Search(TrafficSearchArgs),
    /// Get traffic count
    Count,
    /// Clear all traffic
    Clear,
}
impl TrafficCommands {
    pub async fn execute(&self, api_url: String) -> Result<()> {
        match self {
            TrafficCommands::List(args) => {
                let client = ApiClient::new(api_url);
                let result = client.get("traffic").await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    print_traffic_list(&result);
                }
            }
            TrafficCommands::Get(args) => {
                let client = ApiClient::new(api_url);
                let result = client.get(&format!("traffic/{}", args.id)).await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    print_traffic_detail(&result);
                }
            }
            TrafficCommands::Search(args) => {
                let client = ApiClient::new(api_url);
                let encoded = urlencoding::encode(&args.query);
                let result = client.get(&format!("traffic/search?q={}", encoded)).await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    print_traffic_list(&result);
                }
            }
            TrafficCommands::Count => {
                let client = ApiClient::new(api_url);
                let result = client.get("traffic/count").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            TrafficCommands::Clear => {
                let client = ApiClient::new(api_url);
                let result = client.delete("traffic").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }
    }
}


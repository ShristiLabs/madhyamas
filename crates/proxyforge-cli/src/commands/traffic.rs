//! Traffic inspection commands

use anyhow::Result;
use clap::{Args, Subcommand};

use super::ApiClient;

#[derive(Debug, Args)]
pub struct TrafficListArgs {
    /// Filter by URL pattern (e.g., "api.example.com/*")
    #[arg(short, long)]
    pub filter: Option<String>,

    /// Filter by HTTP method (GET, POST, PUT, DELETE, etc.)
    #[arg(short, long)]
    pub method: Option<String>,

    /// Filter by HTTP status code (e.g., 200, 404, 500)
    #[arg(short, long)]
    pub status: Option<u16>,

    /// Filter by file type/extension (e.g., json, html, css, js, png)
    #[arg(long)]
    pub file_type: Option<String>,

    /// Filter by header (format: "key:value" or just "key")
    #[arg(long)]
    pub header: Option<String>,

    /// Filter by cookie (format: "name=value" or just "name")
    #[arg(long)]
    pub cookie: Option<String>,

    /// Search in request/response bodies
    #[arg(long)]
    pub search: Option<String>,

    /// Filter by minimum response size in bytes
    #[arg(long)]
    pub min_size: Option<usize>,

    /// Filter by maximum response size in bytes
    #[arg(long)]
    pub max_size: Option<usize>,

    /// Filter by minimum response time in milliseconds
    #[arg(long)]
    pub min_time: Option<u64>,

    /// Filter by maximum response time in milliseconds
    #[arg(long)]
    pub max_time: Option<u64>,

    /// Maximum number of results
    #[arg(short, long, default_value = "100")]
    pub limit: usize,

    /// Offset for pagination
    #[arg(short, long)]
    pub offset: Option<usize>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
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

                // Build query string with all filter parameters
                let mut query_parts: Vec<String> = Vec::new();

                if let Some(ref filter) = args.filter {
                    query_parts.push(format!("filter={}", urlencoding::encode(filter)));
                }
                if let Some(ref method) = args.method {
                    query_parts.push(format!("method={}", urlencoding::encode(method)));
                }
                if let Some(status) = args.status {
                    query_parts.push(format!("status={}", status));
                }
                if let Some(ref file_type) = args.file_type {
                    query_parts.push(format!("file_type={}", urlencoding::encode(file_type)));
                }
                if let Some(ref header) = args.header {
                    query_parts.push(format!("header={}", urlencoding::encode(header)));
                }
                if let Some(ref cookie) = args.cookie {
                    query_parts.push(format!("cookie={}", urlencoding::encode(cookie)));
                }
                if let Some(ref search) = args.search {
                    query_parts.push(format!("search={}", urlencoding::encode(search)));
                }
                if let Some(min_size) = args.min_size {
                    query_parts.push(format!("min_size={}", min_size));
                }
                if let Some(max_size) = args.max_size {
                    query_parts.push(format!("max_size={}", max_size));
                }
                if let Some(min_time) = args.min_time {
                    query_parts.push(format!("min_time={}", min_time));
                }
                if let Some(max_time) = args.max_time {
                    query_parts.push(format!("max_time={}", max_time));
                }
                query_parts.push(format!("limit={}", args.limit));
                if let Some(offset) = args.offset {
                    query_parts.push(format!("offset={}", offset));
                }

                let path = if query_parts.is_empty() {
                    "traffic".to_string()
                } else {
                    format!("traffic?{}", query_parts.join("&"))
                };

                let result = client.get(&path).await?;
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

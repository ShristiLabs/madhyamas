//! gRPC inspection commands

use anyhow::Result;
use clap::{Args, Subcommand};

use super::ApiClient;

#[derive(Debug, Args)]
pub struct GrpcFramesArgs {
    /// Filter by connection ID
    #[arg(long)]
    pub connection_id: Option<String>,

    /// Filter by stream ID
    #[arg(long)]
    pub stream_id: Option<String>,

    /// Maximum number of results
    #[arg(short, long, default_value = "100")]
    pub limit: usize,
}

#[derive(Debug, Subcommand)]
pub enum GrpcCommands {
    /// List gRPC connections
    Connections,
    /// List gRPC streams
    Streams,
    /// List gRPC frames (with optional filters)
    Frames(GrpcFramesArgs),
    /// Get gRPC statistics
    Stats,
    /// Clear all gRPC frames
    Clear,
}

impl GrpcCommands {
    pub async fn execute(&self, api_url: String) -> Result<()> {
        let client = ApiClient::new(api_url);

        match self {
            GrpcCommands::Connections => {
                let result = client.get("grpc/connections").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            GrpcCommands::Streams => {
                let result = client.get("grpc/streams").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            GrpcCommands::Frames(args) => {
                let mut query_parts: Vec<String> = Vec::new();
                if let Some(ref conn) = args.connection_id {
                    query_parts.push(format!("connection_id={}", conn));
                }
                if let Some(ref stream) = args.stream_id {
                    query_parts.push(format!("stream_id={}", stream));
                }
                query_parts.push(format!("limit={}", args.limit));

                let path = format!("grpc/frames?{}", query_parts.join("&"));
                let result = client.get(&path).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            GrpcCommands::Stats => {
                let result = client.get("grpc/stats").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            GrpcCommands::Clear => {
                let result = client.post("grpc/clear", serde_json::json!({})).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }

        Ok(())
    }
}

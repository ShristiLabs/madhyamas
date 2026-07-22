//! Export commands (HAR, cURL)

use anyhow::Result;
use clap::{Args, Subcommand};

use super::ApiClient;

#[derive(Debug, Args)]
pub struct ExportHarArgs {
    /// Write output to the given file instead of stdout
    #[arg(short, long)]
    pub output: Option<String>,
}

#[derive(Debug, Args)]
pub struct ExportCurlArgs {
    /// Traffic entry ID to export as a cURL command
    pub id: String,
}

#[derive(Debug, Subcommand)]
pub enum ExportCommands {
    /// Export captured traffic as a HAR file
    Har(ExportHarArgs),
    /// Export a traffic entry as a cURL command
    Curl(ExportCurlArgs),
}

impl ExportCommands {
    pub async fn execute(&self, api_url: String) -> Result<()> {
        let client = ApiClient::new(api_url);

        match self {
            ExportCommands::Har(args) => {
                let result = client.get("export/har").await?;
                let pretty = serde_json::to_string_pretty(&result)?;
                if let Some(ref path) = args.output {
                    std::fs::write(path, pretty)
                        .map_err(|e| anyhow::anyhow!("Failed to write to '{}': {}", path, e))?;
                    println!("HAR export written to {}", path);
                } else {
                    println!("{}", pretty);
                }
            }
            ExportCommands::Curl(args) => {
                let result = client.get(&format!("export/curl/{}", args.id)).await?;
                // The API may return the curl command as a string or as JSON
                if let Some(curl) = result.as_str() {
                    println!("{}", curl);
                } else if let Some(curl) = result.get("curl").and_then(|v| v.as_str()) {
                    println!("{}", curl);
                } else {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
            }
        }

        Ok(())
    }
}

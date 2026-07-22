//! Script commands

use anyhow::Result;
use clap::{Args, Subcommand};
use serde_json::json;

use super::ApiClient;

#[derive(Debug, Args)]
pub struct ScriptCreateArgs {
    /// Name of the script
    #[arg(short, long)]
    pub name: String,

    /// Path to a file containing the script source
    #[arg(long, conflicts_with = "inline")]
    pub file: Option<String>,

    /// Inline script source
    #[arg(short = 'i', long, conflicts_with = "file")]
    pub inline: Option<String>,

    /// Hook/event the script attaches to (e.g. request, response)
    #[arg(short, long)]
    pub hook: String,
}

#[derive(Debug, Args)]
pub struct ScriptIdArgs {
    /// Script ID
    pub id: String,
}

#[derive(Debug, Subcommand)]
pub enum ScriptCommands {
    /// List all scripts
    List,
    /// Create a script
    Create(ScriptCreateArgs),
    /// Get a specific script
    Get(ScriptIdArgs),
    /// Delete a script
    Delete(ScriptIdArgs),
    /// Toggle a script on/off
    Toggle(ScriptIdArgs),
    /// List available script templates
    Templates,
}

impl ScriptCommands {
    pub async fn execute(&self, api_url: String) -> Result<()> {
        let client = ApiClient::new(api_url);

        match self {
            ScriptCommands::List => {
                let result = client.get("scripts").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            ScriptCommands::Create(args) => {
                let source = if let Some(ref path) = args.file {
                    std::fs::read_to_string(path).map_err(|e| {
                        anyhow::anyhow!("Failed to read script file '{}': {}", path, e)
                    })?
                } else if let Some(ref src) = args.inline {
                    src.clone()
                } else {
                    anyhow::bail!("Either --file or --inline must be provided");
                };

                let body = json!({
                    "name": args.name,
                    "source": source,
                    "hook": args.hook,
                });
                let result = client.post("scripts", body).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            ScriptCommands::Get(args) => {
                let result = client.get(&format!("scripts/{}", args.id)).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            ScriptCommands::Delete(args) => {
                let result = client.delete(&format!("scripts/{}", args.id)).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            ScriptCommands::Toggle(args) => {
                let result = client
                    .post(&format!("scripts/{}/toggle", args.id), json!({}))
                    .await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            ScriptCommands::Templates => {
                let result = client.get("scripts/templates").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }

        Ok(())
    }
}

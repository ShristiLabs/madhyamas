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

    /// Hooks to attach the script to (repeatable: --hook on_request --hook on_response)
    #[arg(short = 'H', long = "hook")]
    pub hooks: Vec<String>,

    /// Optional description
    #[arg(short, long)]
    pub description: Option<String>,
}

#[derive(Debug, Args)]
pub struct ScriptIdArgs {
    /// Script ID
    pub id: String,
}

#[derive(Debug, Args)]
pub struct ScriptToggleArgs {
    /// Script ID
    pub id: String,

    /// Enable (true) or disable (false) the script
    #[arg(short, long)]
    pub enabled: bool,
}

#[derive(Debug, Args)]
pub struct ScriptTestArgs {
    /// Path to a file containing the script source to test
    #[arg(long, conflicts_with = "inline")]
    pub file: Option<String>,

    /// Inline script source to test
    #[arg(short = 'i', long, conflicts_with = "file")]
    pub inline: Option<String>,

    /// Hook to test against (e.g. on_request, on_response)
    #[arg(short = 'H', long)]
    pub hook: String,
}

#[derive(Debug, Args)]
pub struct ScriptValidateArgs {
    /// Path to a file containing the script source to validate
    #[arg(long, conflicts_with = "inline")]
    pub file: Option<String>,

    /// Inline script source to validate
    #[arg(short = 'i', long, conflicts_with = "file")]
    pub inline: Option<String>,
}

#[derive(Debug, Args)]
pub struct ScriptHistoryArgs {
    /// Script ID
    pub id: String,

    /// Maximum number of history entries to show
    #[arg(short, long, default_value = "20")]
    pub limit: usize,
}

#[derive(Debug, Args)]
pub struct ScriptReorderArgs {
    /// Script ID
    pub id: String,

    /// New priority position (lower = earlier in the chain)
    #[arg(short, long)]
    pub priority: i32,
}

#[derive(Debug, Args)]
pub struct ScriptMatchPreviewArgs {
    /// URL to test for script matches
    #[arg(long)]
    pub url: String,

    /// HTTP method
    #[arg(long, default_value = "GET")]
    pub method: String,
}

#[derive(Debug, Args)]
pub struct ScriptConfigArgs {
    /// Script execution timeout in milliseconds
    #[arg(long)]
    pub timeout_ms: Option<u64>,

    /// Memory limit in MB
    #[arg(long)]
    pub memory_limit_mb: Option<u64>,

    /// Enable console output capture
    #[arg(long)]
    pub capture_console: Option<bool>,
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
    /// Enable or disable a script
    Toggle(ScriptToggleArgs),
    /// List available script templates
    Templates,
    /// Test (dry-run) a script against a sample context
    Test(ScriptTestArgs),
    /// Validate a script's syntax without executing it
    Validate(ScriptValidateArgs),
    /// Show execution history for a script
    History(ScriptHistoryArgs),
    /// Show execution history across all scripts
    HistoryAll,
    /// Clear execution history for a script
    HistoryClear(ScriptIdArgs),
    /// Reorder a script (change its priority)
    Reorder(ScriptReorderArgs),
    /// Preview which scripts would match a given request
    MatchPreview(ScriptMatchPreviewArgs),
    /// Get global script runtime configuration
    Config,
    /// Update global script runtime configuration
    ConfigUpdate(ScriptConfigArgs),
}

impl ScriptCommands {
    pub async fn execute(&self, api_url: String, auth: super::CliAuth) -> Result<()> {
        let client = ApiClient::new(api_url, auth.clone());

        match self {
            ScriptCommands::List => {
                let result = client.get("scripts").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            ScriptCommands::Create(args) => {
                let source = read_source(&args.file, &args.inline)?;

                // Build hooks array — default to on_request if none specified.
                let hooks: Vec<&str> = if args.hooks.is_empty() {
                    vec!["on_request"]
                } else {
                    args.hooks.iter().map(|s| s.as_str()).collect()
                };

                let mut body = json!({
                    "name": args.name,
                    "source": source,
                    "hooks": hooks,
                });
                if let Some(ref desc) = args.description {
                    body["description"] = json!(desc);
                }
                let result = client.post("scripts", body).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            ScriptCommands::Get(args) => {
                let result = client.get(&format!("scripts/{}", args.id)).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            ScriptCommands::Delete(args) => {
                client.delete_void(&format!("scripts/{}", args.id)).await?;
                println!("Script {} deleted.", args.id);
            }
            ScriptCommands::Toggle(args) => {
                let body = json!({ "enabled": args.enabled });
                client
                    .post_void(&format!("scripts/{}/toggle", args.id), body)
                    .await?;
                println!(
                    "Script {} {}.",
                    args.id,
                    if args.enabled { "enabled" } else { "disabled" }
                );
            }
            ScriptCommands::Templates => {
                let result = client.get("scripts/templates").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            ScriptCommands::Test(args) => {
                let source = read_source(&args.file, &args.inline)?;
                let body = json!({
                    "source": source,
                    "hook": args.hook,
                });
                let result = client.post("scripts/test", body).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            ScriptCommands::Validate(args) => {
                let source = read_source(&args.file, &args.inline)?;
                let body = json!({ "source": source });
                let result = client.post("scripts/validate", body).await?;
                let valid = result
                    .get("valid")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                if valid {
                    println!("Script source is valid.");
                } else {
                    let error = result
                        .get("error")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown error");
                    println!("Script source is INVALID: {error}");
                    std::process::exit(1);
                }
            }
            ScriptCommands::History(args) => {
                let result = client
                    .get(&format!("scripts/{}/history?limit={}", args.id, args.limit))
                    .await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            ScriptCommands::HistoryAll => {
                let result = client.get("scripts/history").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            ScriptCommands::HistoryClear(args) => {
                client
                    .delete_void(&format!("scripts/{}/history", args.id))
                    .await?;
                println!("Cleared history for script {}.", args.id);
            }
            ScriptCommands::Reorder(args) => {
                let body = json!({ "priority": args.priority });
                client
                    .post_void(&format!("scripts/{}/reorder", args.id), body)
                    .await?;
                println!(
                    "Reordered script {} to priority {}.",
                    args.id, args.priority
                );
            }
            ScriptCommands::MatchPreview(args) => {
                let body = json!({
                    "url": args.url,
                    "method": args.method,
                });
                let result = client.post("scripts/match-preview", body).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            ScriptCommands::Config => {
                let result = client.get("scripts/config").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            ScriptCommands::ConfigUpdate(args) => {
                let mut body = json!({});
                if let Some(t) = args.timeout_ms {
                    body["timeout_ms"] = json!(t);
                }
                if let Some(m) = args.memory_limit_mb {
                    body["memory_limit_mb"] = json!(m);
                }
                if let Some(c) = args.capture_console {
                    body["capture_console"] = json!(c);
                }
                let result = client.put("scripts/config", body).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }

        Ok(())
    }
}

/// Read script source from a file or inline string.
fn read_source(file: &Option<String>, inline: &Option<String>) -> Result<String> {
    if let Some(ref path) = file {
        std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read script file '{}': {}", path, e))
    } else if let Some(ref src) = inline {
        Ok(src.clone())
    } else {
        anyhow::bail!("Either --file or --inline must be provided");
    }
}

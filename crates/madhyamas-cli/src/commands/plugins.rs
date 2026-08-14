//! Plugin commands

use anyhow::Result;
use clap::{Args, Subcommand};
use serde_json::json;

use super::ApiClient;

#[derive(Debug, Args)]
pub struct PluginIdArgs {
    /// Plugin ID
    pub id: String,
}

#[derive(Debug, Args)]
pub struct PluginInstallArgs {
    /// Install source: "url" or "registry"
    #[arg(long, default_value = "url")]
    pub source: String,
    /// Plugin URL (when source=url) or registry id (when source=registry)
    pub target: String,
    /// Expected SHA-256 checksum (optional for URL source)
    #[arg(long)]
    pub checksum: Option<String>,
}

#[derive(Debug, Args)]
pub struct PluginSearchArgs {
    /// Search query
    pub query: String,
}

#[derive(Debug, Args)]
pub struct PluginSettingsArgs {
    /// Plugin ID
    pub id: String,
    /// Settings as a JSON string
    #[arg(long)]
    pub settings: String,
}

#[derive(Debug, Args)]
pub struct PluginLogsArgs {
    /// Plugin ID
    pub id: String,
    /// Maximum number of log entries to return
    #[arg(long, default_value = "50")]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct PluginSignArgs {
    /// Path to the plugin zip package to sign
    pub zip_path: String,
    /// Publisher secret key as hex (64 hex chars = 32 bytes)
    #[arg(long)]
    pub secret_key: String,
    /// Output path for the signature file (default: <zip_path>.sig)
    #[arg(long)]
    pub output: Option<String>,
}

#[derive(Debug, Args)]
pub struct PluginGenKeyArgs {
    /// Output format: "hex" (default) or "json"
    #[arg(long, default_value = "hex")]
    pub format: String,
}

#[derive(Debug, Args)]
pub struct PluginNewArgs {
    /// Template id: basic, cors, request-logger, domain-blocker, response-modifier
    pub template: String,
    /// Plugin name (kebab-case, e.g. "my-cors-plugin")
    pub name: String,
    /// Output directory (default: current directory)
    #[arg(long, default_value = ".")]
    pub output: String,
}

#[derive(Debug, Subcommand)]
pub enum PluginCommands {
    /// List all plugins
    List,
    /// Get a specific plugin
    Get(PluginIdArgs),
    /// Enable a plugin
    Enable(PluginIdArgs),
    /// Disable a plugin
    Disable(PluginIdArgs),
    /// Get statistics for a plugin
    Stats(PluginIdArgs),
    /// Reload all plugins
    Reload,
    /// Install a plugin from a URL or registry id
    Install(PluginInstallArgs),
    /// Uninstall a plugin
    Uninstall(PluginIdArgs),
    /// Search the plugin registry
    Search(PluginSearchArgs),
    /// List available plugins in the registry
    Registry,
    /// Show or set the registry GitHub repo (e.g. "owner/repo" or "owner/repo@branch")
    RegistryConfig {
        /// Set the registry repo (omit to just show current config)
        repo: Option<String>,
    },
    /// Force-refresh the registry cache
    RegistryRefresh,
    /// Get a plugin's settings schema
    Schema(PluginIdArgs),
    /// Get a plugin's current settings
    GetSettings(PluginIdArgs),
    /// Update a plugin's settings (pass JSON via --settings)
    SetSettings(PluginSettingsArgs),
    /// Get a plugin's recent invocation logs
    Logs(PluginLogsArgs),
    /// Generate a new Ed25519 keypair for signing plugins
    GenKey(PluginGenKeyArgs),
    /// Sign a plugin zip package with a publisher secret key
    Sign(PluginSignArgs),
    /// Scaffold a new plugin project from a template
    New(PluginNewArgs),
    /// List available plugin templates
    Templates,
}

impl PluginCommands {
    pub async fn execute(&self, api_url: String, auth: super::CliAuth) -> Result<()> {
        let client = ApiClient::new(api_url, auth.clone());

        match self {
            PluginCommands::List => {
                let result = client.get("plugins").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            PluginCommands::Get(args) => {
                let result = client.get(&format!("plugins/{}", args.id)).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            PluginCommands::Enable(args) => {
                let result = client
                    .post(&format!("plugins/{}/enable", args.id), json!({}))
                    .await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            PluginCommands::Disable(args) => {
                let result = client
                    .post(&format!("plugins/{}/disable", args.id), json!({}))
                    .await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            PluginCommands::Stats(args) => {
                let result = client.get(&format!("plugins/{}/stats", args.id)).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            PluginCommands::Reload => {
                let result = client.post("plugins/reload", json!({})).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            PluginCommands::Install(args) => {
                let body = match args.source.as_str() {
                    "registry" => json!({
                        "source": "registry",
                        "id": args.target,
                    }),
                    _ => json!({
                        "source": "url",
                        "url": args.target,
                        "checksum": args.checksum,
                    }),
                };
                let result = client.post("plugins/install", body).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            PluginCommands::Uninstall(args) => {
                let result = client
                    .delete(&format!("plugins/{}/uninstall", args.id))
                    .await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            PluginCommands::Search(args) => {
                let result = client
                    .get(&format!("plugins/registry/search?q={}", args.query))
                    .await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            PluginCommands::Registry => {
                let result = client.get("plugins/registry").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            PluginCommands::RegistryConfig { repo } => {
                if let Some(repo) = repo {
                    let result = client
                        .put("plugins/registry/config", json!({ "repo": repo }))
                        .await?;
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    let result = client.get("plugins/registry/config").await?;
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
            }
            PluginCommands::RegistryRefresh => {
                let result = client.post("plugins/registry/refresh", json!({})).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            PluginCommands::Schema(args) => {
                let result = client.get(&format!("plugins/{}/schema", args.id)).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            PluginCommands::GetSettings(args) => {
                let result = client.get(&format!("plugins/{}/settings", args.id)).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            PluginCommands::SetSettings(args) => {
                let settings: serde_json::Value = serde_json::from_str(&args.settings)?;
                let result = client
                    .put(&format!("plugins/{}/settings", args.id), settings)
                    .await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            PluginCommands::Logs(args) => {
                let result = client
                    .get(&format!("plugins/{}/logs?limit={}", args.id, args.limit))
                    .await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            PluginCommands::GenKey(args) => {
                use madhyamas_core::{bytes_to_hex, generate_keypair};
                let kp = generate_keypair();
                let pub_hex = bytes_to_hex(&kp.public_key);
                let sec_hex = bytes_to_hex(&kp.secret_key);
                match args.format.as_str() {
                    "json" => {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "public_key": pub_hex,
                                "secret_key": sec_hex,
                            }))?
                        );
                    }
                    _ => {
                        println!("public_key:  {}", pub_hex);
                        println!("secret_key:  {}", sec_hex);
                        println!();
                        println!(
                            "Add the public_key to your manifest's `publisher_public_key` field."
                        );
                        println!("Keep the secret_key secure — you need it to sign packages.");
                    }
                }
            }
            PluginCommands::Sign(args) => {
                use madhyamas_core::{bytes_to_hex, hex_to_bytes, sign_package};
                let secret_key: [u8; 32] = hex_to_bytes(&args.secret_key)
                    .map_err(|e| anyhow::anyhow!("invalid secret key: {}", e))?;
                let zip_bytes = std::fs::read(&args.zip_path)
                    .map_err(|e| anyhow::anyhow!("failed to read zip: {}", e))?;
                let sig = sign_package(&zip_bytes, &secret_key)
                    .map_err(|e| anyhow::anyhow!("signing failed: {}", e))?;
                let sig_hex = bytes_to_hex(&sig);
                let out_path = args
                    .output
                    .clone()
                    .unwrap_or_else(|| format!("{}.sig", args.zip_path));
                std::fs::write(&out_path, sig)
                    .map_err(|e| anyhow::anyhow!("failed to write signature: {}", e))?;
                println!("Signed: {}", args.zip_path);
                println!("Signature written to: {}", out_path);
                println!("Signature (hex): {}", sig_hex);
            }
            PluginCommands::New(args) => {
                use madhyamas_core::{PluginTemplates, TemplateId};
                let template_id = TemplateId::from_id(&args.template)
                    .ok_or_else(|| anyhow::anyhow!("unknown template: {} (available: basic, cors, request-logger, domain-blocker, response-modifier)", args.template))?;
                let output_dir = std::path::Path::new(&args.output);
                PluginTemplates::scaffold(&template_id, &args.name, output_dir)
                    .map_err(|e| anyhow::anyhow!("failed to scaffold plugin: {}", e))?;
                let plugin_dir = output_dir.join(&args.name);
                println!(
                    "Created plugin '{}' from '{}' template at: {}",
                    args.name,
                    args.template,
                    plugin_dir.display()
                );
                println!();
                println!("Next steps:");
                println!("  1. rustup target add wasm32-unknown-unknown");
                println!(
                    "  2. cd {} && cargo build --target wasm32-unknown-unknown --release",
                    plugin_dir.display()
                );
                println!(
                    "  3. cp target/wasm32-unknown-unknown/release/{}.wasm plugin.wasm",
                    args.name.replace('-', "_")
                );
                println!(
                    "  4. madhyamas plugins install --source url file://{}",
                    plugin_dir.display()
                );
            }
            PluginCommands::Templates => {
                use madhyamas_core::PluginTemplates;
                let header = format!("{:<20} {:<20} {}", "ID", "Name", "Description");
                println!("{header}");
                println!("{}", "-".repeat(80));
                for t in PluginTemplates::all() {
                    println!("{:<20} {:<20} {}", t.id.as_str(), t.name, t.description);
                }
                println!();
                println!(
                    "Usage: madhyamas plugins new <template-id> <plugin-name> [--output <dir>]"
                );
            }
        }

        Ok(())
    }
}

//! Mock response commands

use anyhow::Result;
use clap::{Args, Subcommand};
use serde_json::{json, Value};

use super::ApiClient;

#[derive(Debug, Args)]
pub struct MockCreateArgs {
    /// URL pattern to match (supports wildcards: */api.example.com/*)
    #[arg(short, long)]
    pub url_pattern: String,

    /// HTTP method to match
    #[arg(short, long)]
    pub method: Option<String>,

    /// Response status code
    #[arg(short, long)]
    pub status_code: Option<u16>,

    /// Response body
    #[arg(short, long)]
    pub body: Option<String>,

    /// Response delay in milliseconds
    #[arg(short, long)]
    pub delay_ms: Option<u64>,

    /// Enable or disable
    #[arg(short, long)]
    pub enabled: Option<bool>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct MockDeleteArgs {
    /// Mock ID
    pub id: String,
}

#[derive(Debug, Args)]
pub struct MockToggleArgs {
    /// Mock ID
    pub id: String,

    /// Enable or disable
    pub enabled: bool,
}

#[derive(Debug, Args)]
pub struct MockBatchToggleArgs {
    /// Comma-separated list of mock IDs
    #[arg(short, long)]
    pub ids: String,

    /// Enable or disable
    #[arg(short, long)]
    pub enabled: bool,
}

#[derive(Debug, Args)]
pub struct MockUpdateArgs {
    /// Mock ID
    pub id: String,

    /// URL pattern to match (supports wildcards: */api.example.com/*)
    #[arg(short, long)]
    pub url_pattern: Option<String>,

    /// HTTP method to match
    #[arg(short, long)]
    pub method: Option<String>,

    /// Response status code
    #[arg(short, long)]
    pub status_code: Option<u16>,

    /// Response body
    #[arg(short, long)]
    pub body: Option<String>,

    /// Response delay in milliseconds
    #[arg(short, long)]
    pub delay_ms: Option<u64>,

    /// Enable or disable
    #[arg(short, long)]
    pub enabled: Option<bool>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct MockDuplicateArgs {
    /// Mock ID to duplicate
    pub id: String,

    /// Optional new name for the duplicate
    #[arg(short, long)]
    pub new_name: Option<String>,
}

#[derive(Debug, Args)]
pub struct MockRollbackArgs {
    /// Mock ID
    pub id: String,

    /// Version number to rollback to
    #[arg(short, long)]
    pub version: Option<u32>,
}

#[derive(Debug, Args)]
pub struct MockCreateAdvancedArgs {
    /// Advanced mock configuration as a JSON string
    #[arg(short, long)]
    pub config: Option<String>,

    /// Read advanced mock configuration from a JSON file
    #[arg(long)]
    pub config_file: Option<String>,
}

#[derive(Debug, Args)]
pub struct MockOptionalIdArgs {
    /// Optional mock ID (if omitted, returns global analytics/history)
    pub id: Option<String>,
}

#[derive(Debug, Args)]
pub struct MockPreviewArgs {
    /// HTTP method for the test request
    #[arg(short, long)]
    pub method: Option<String>,

    /// URL for the test request
    #[arg(short, long)]
    pub url: Option<String>,

    /// Request headers as a JSON string
    #[arg(short, long)]
    pub headers: Option<String>,

    /// Request body
    #[arg(short, long)]
    pub body: Option<String>,
}

#[derive(Debug, Args)]
pub struct MockExportArgs {
    /// Write export to a file instead of stdout
    #[arg(short, long)]
    pub output: Option<String>,
}

#[derive(Debug, Args)]
pub struct MockImportArgs {
    /// Read import data from a JSON file
    #[arg(short, long)]
    pub input: String,

    /// Import format (har, openapi, postman)
    #[arg(short, long, default_value = "har")]
    pub format: String,
}

#[derive(Debug, Args)]
pub struct MockTestArgs {
    /// Mock ID to test
    pub id: String,

    /// HTTP method for the test request
    #[arg(short, long)]
    pub method: Option<String>,

    /// URL for the test request
    #[arg(short, long)]
    pub url: Option<String>,

    /// Request headers as a JSON string
    #[arg(short, long)]
    pub headers: Option<String>,

    /// Request body
    #[arg(short, long)]
    pub body: Option<String>,
}

#[derive(Debug, Subcommand)]
pub enum MockRecordingCommands {
    /// Enable or disable mock recording mode
    Set(MockRecordingSetArgs),
    /// Get current recording status
    Status,
    /// List all recorded mock candidates
    List,
    /// Promote recorded mocks to active rules
    Promote,
}

#[derive(Debug, Args)]
pub struct MockRecordingSetArgs {
    /// Enable or disable recording
    #[arg(short, long)]
    pub enabled: bool,
}

#[derive(Debug, Subcommand)]
pub enum MockCollectionCommands {
    /// List all mock collections
    List,
    /// Create a new mock collection
    Create(MockCollectionCreateArgs),
    /// Get a specific mock collection by ID
    Get(MockCollectionIdArgs),
    /// Delete a mock collection
    Delete(MockCollectionDeleteArgs),
    /// Toggle all mocks in a collection on/off
    Toggle(MockCollectionToggleArgs),
    /// Update a mock collection's metadata
    Update(MockCollectionUpdateArgs),
}

#[derive(Debug, Args)]
pub struct MockCollectionCreateArgs {
    /// Collection name
    #[arg(short, long)]
    pub name: String,

    /// Optional description
    #[arg(short, long)]
    pub description: Option<String>,
}

#[derive(Debug, Args)]
pub struct MockCollectionIdArgs {
    /// Collection ID
    pub id: String,
}

#[derive(Debug, Args)]
pub struct MockCollectionDeleteArgs {
    /// Collection ID
    pub id: String,

    /// Also delete all rules in the collection
    #[arg(short, long)]
    pub delete_rules: bool,
}

#[derive(Debug, Args)]
pub struct MockCollectionToggleArgs {
    /// Collection ID
    pub id: String,

    /// Enable or disable all rules
    pub enabled: bool,
}

#[derive(Debug, Args)]
pub struct MockCollectionUpdateArgs {
    /// Collection ID
    pub id: String,

    /// New name for the collection
    #[arg(short, long)]
    pub name: Option<String>,

    /// New description for the collection
    #[arg(short, long)]
    pub description: Option<String>,

    /// Enable or disable the collection
    #[arg(short, long)]
    pub enabled: Option<bool>,
}

#[derive(Debug, Subcommand)]
pub enum MockCommands {
    /// List all mock rules
    List,
    /// Get a specific mock rule
    Get(MockDeleteArgs),
    /// Create a mock rule
    Create(MockCreateArgs),
    /// Update an existing mock rule
    Update(MockUpdateArgs),
    /// Delete a mock rule
    Delete(MockDeleteArgs),
    /// Toggle mock rule on/off
    Toggle(MockToggleArgs),
    /// Batch toggle multiple mock rules
    BatchToggle(MockBatchToggleArgs),
    /// Duplicate an existing mock rule
    Duplicate(MockDuplicateArgs),
    /// Rollback a mock rule to a previous version
    Rollback(MockRollbackArgs),
    /// Get version history for a mock rule
    Versions(MockDeleteArgs),
    /// Create an advanced mock rule from a JSON config
    CreateAdvanced(MockCreateAdvancedArgs),
    /// Get mock hit analytics (global or per-rule with an ID argument)
    Analytics(MockOptionalIdArgs),
    /// Get mock hit history (per-rule with an ID argument)
    History(MockOptionalIdArgs),
    /// Preview which mock rule would match a given request
    Preview(MockPreviewArgs),
    /// Test a mock rule against a sample request
    Test(MockTestArgs),
    /// Export all mock rules as JSON
    Export(MockExportArgs),
    /// Import mock rules from a file
    Import(MockImportArgs),
    /// List available mock templates
    Templates,
    /// Clear all recorded mock candidates
    ClearRecording,
    /// Clear all mock hit history/analytics
    ClearAnalytics,
    /// Mock recording subcommands
    #[command(subcommand)]
    Recording(MockRecordingCommands),
    /// Mock collection subcommands
    #[command(subcommand)]
    Collections(MockCollectionCommands),
}

impl MockCommands {
    pub async fn execute(&self, api_url: String) -> Result<()> {
        match self {
            MockCommands::List => {
                let client = ApiClient::new(api_url);
                let result = client.get("mocks").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            MockCommands::Create(args) => {
                let client = ApiClient::new(api_url);
                let mut body = json!({
                    "url_pattern": args.url_pattern,
                });
                if let Some(ref m) = args.method {
                    body["method"] = Value::String(m.clone());
                }
                if let Some(s) = args.status_code {
                    body["status_code"] = Value::Number(s.into());
                }
                if let Some(ref b) = args.body {
                    body["body"] = Value::String(b.clone());
                }
                if let Some(d) = args.delay_ms {
                    body["delay_ms"] = Value::Number(d.into());
                }
                if let Some(e) = args.enabled {
                    body["enabled"] = Value::Bool(e);
                }
                let result = client.post("mocks", body).await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    let id = result.get("id").and_then(|v| v.as_str()).unwrap_or("?");
                    println!("Created mock with ID: {}", id);
                }
            }
            MockCommands::Update(args) => {
                let client = ApiClient::new(api_url);
                let mut body = json!({});
                if let Some(ref p) = args.url_pattern {
                    body["url_pattern"] = Value::String(p.clone());
                }
                if let Some(ref m) = args.method {
                    body["method"] = Value::String(m.clone());
                }
                if let Some(s) = args.status_code {
                    body["status_code"] = Value::Number(s.into());
                }
                if let Some(ref b) = args.body {
                    body["body"] = Value::String(b.clone());
                }
                if let Some(d) = args.delay_ms {
                    body["delay_ms"] = Value::Number(d.into());
                }
                if let Some(e) = args.enabled {
                    body["enabled"] = Value::Bool(e);
                }
                let result = client.put(&format!("mocks/{}", args.id), body).await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("Updated mock {}", args.id);
                }
            }
            MockCommands::Delete(args) => {
                let client = ApiClient::new(api_url);
                let result = client.delete(&format!("mocks/{}", args.id)).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            MockCommands::Toggle(args) => {
                let client = ApiClient::new(api_url);
                let body = json!({ "enabled": args.enabled });
                let result = client
                    .post(&format!("mocks/{}/toggle", args.id), body)
                    .await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            MockCommands::BatchToggle(args) => {
                let client = ApiClient::new(api_url);
                let ids: Vec<&str> = args.ids.split(',').map(|s| s.trim()).collect();
                let body = json!({ "ids": ids, "enabled": args.enabled });
                let result = client.post("mocks/batch-toggle", body).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            MockCommands::Duplicate(args) => {
                let client = ApiClient::new(api_url);
                let mut body = json!({});
                if let Some(ref name) = args.new_name {
                    body["new_name"] = Value::String(name.clone());
                }
                let result = client
                    .post(&format!("mocks/{}/duplicate", args.id), body)
                    .await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            MockCommands::Rollback(args) => {
                let client = ApiClient::new(api_url);
                let body = if let Some(v) = args.version {
                    json!({ "version": v })
                } else {
                    json!({})
                };
                let result = client
                    .post(&format!("mocks/{}/rollback", args.id), body)
                    .await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            MockCommands::Versions(args) => {
                let client = ApiClient::new(api_url);
                let result = client.get(&format!("mocks/{}/versions", args.id)).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            MockCommands::CreateAdvanced(args) => {
                let config_str = if let Some(ref c) = args.config {
                    c.clone()
                } else if let Some(ref path) = args.config_file {
                    std::fs::read_to_string(path)?
                } else {
                    anyhow::bail!("Either --config or --config-file is required");
                };
                let config: Value = serde_json::from_str(&config_str)?;
                let client = ApiClient::new(api_url);
                let result = client.post("mocks/advanced", config).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            MockCommands::Analytics(args) => {
                let client = ApiClient::new(api_url);
                let result = if let Some(ref id) = args.id {
                    client.get(&format!("mocks/{}/analytics", id)).await?
                } else {
                    client.get("mocks/analytics").await?
                };
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            MockCommands::History(args) => {
                let client = ApiClient::new(api_url);
                let id = args
                    .id
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Mock ID is required for history"))?;
                let result = client.get(&format!("mocks/{}/history", id)).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            MockCommands::Preview(args) => {
                let client = ApiClient::new(api_url);
                let mut request = json!({});
                if let Some(ref m) = args.method {
                    request["method"] = Value::String(m.clone());
                }
                if let Some(ref u) = args.url {
                    request["url"] = Value::String(u.clone());
                }
                if let Some(ref h) = args.headers {
                    let headers: Value = serde_json::from_str(h)?;
                    request["headers"] = headers;
                }
                if let Some(ref b) = args.body {
                    request["body"] = Value::String(b.clone());
                }
                let body = json!({ "request": request });
                let result = client.post("mocks/preview", body).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            MockCommands::Test(args) => {
                let client = ApiClient::new(api_url);
                let mut request = json!({});
                if let Some(ref m) = args.method {
                    request["method"] = Value::String(m.clone());
                }
                if let Some(ref u) = args.url {
                    request["url"] = Value::String(u.clone());
                }
                if let Some(ref h) = args.headers {
                    let headers: Value = serde_json::from_str(h)?;
                    request["headers"] = headers;
                }
                if let Some(ref b) = args.body {
                    request["body"] = Value::String(b.clone());
                }
                let body = json!({ "request": request });
                let result = client
                    .post(&format!("mocks/{}/test", args.id), body)
                    .await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            MockCommands::Export(args) => {
                let client = ApiClient::new(api_url);
                let result = client.get("mocks/export").await?;
                let pretty = serde_json::to_string_pretty(&result)?;
                if let Some(ref path) = args.output {
                    std::fs::write(path, &pretty)?;
                    println!("Exported mocks to {}", path);
                } else {
                    println!("{}", pretty);
                }
            }
            MockCommands::Import(args) => {
                let data = std::fs::read_to_string(&args.input)?;
                let client = ApiClient::new(api_url);
                let body = json!({ "format": args.format, "data": data });
                let result = client.post("mocks/import", body).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            MockCommands::Templates => {
                let client = ApiClient::new(api_url);
                let result = client.get("mocks/templates").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            MockCommands::ClearRecording => {
                let client = ApiClient::new(api_url);
                client.post_void("mocks/recording/clear", json!({})).await?;
                println!("Cleared all recorded mock candidates.");
            }
            MockCommands::ClearAnalytics => {
                let client = ApiClient::new(api_url);
                client.post_void("mocks/history/clear", json!({})).await?;
                println!("Cleared all mock hit history.");
            }
            MockCommands::Get(args) => {
                let client = ApiClient::new(api_url);
                let result = client.get(&format!("mocks/{}", args.id)).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            MockCommands::Recording(cmd) => {
                cmd.execute(api_url).await?;
            }
            MockCommands::Collections(cmd) => {
                cmd.execute(api_url).await?;
            }
        }
        Ok(())
    }
}

impl MockRecordingCommands {
    pub async fn execute(&self, api_url: String) -> Result<()> {
        match self {
            MockRecordingCommands::Set(args) => {
                let client = ApiClient::new(api_url);
                let body = json!({ "enabled": args.enabled });
                let result = client.post("mocks/recording", body).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            MockRecordingCommands::Status => {
                let client = ApiClient::new(api_url);
                let result = client.get("mocks/recording/status").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            MockRecordingCommands::List => {
                let client = ApiClient::new(api_url);
                let result = client.get("mocks/recording/recorded").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            MockRecordingCommands::Promote => {
                let client = ApiClient::new(api_url);
                let result = client.post("mocks/recording/promote", json!({})).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }
        Ok(())
    }
}

impl MockCollectionCommands {
    pub async fn execute(&self, api_url: String) -> Result<()> {
        match self {
            MockCollectionCommands::List => {
                let client = ApiClient::new(api_url);
                let result = client.get("mocks/collections").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            MockCollectionCommands::Create(args) => {
                let client = ApiClient::new(api_url);
                let mut body = json!({ "name": args.name });
                if let Some(ref desc) = args.description {
                    body["description"] = Value::String(desc.clone());
                }
                let result = client.post("mocks/collections", body).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            MockCollectionCommands::Get(args) => {
                let client = ApiClient::new(api_url);
                let result = client
                    .get(&format!("mocks/collections/{}", args.id))
                    .await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            MockCollectionCommands::Delete(args) => {
                let client = ApiClient::new(api_url);
                let body = json!({ "delete_rules": args.delete_rules });
                let result = client
                    .delete_with_body(&format!("mocks/collections/{}", args.id), body)
                    .await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            MockCollectionCommands::Toggle(args) => {
                let client = ApiClient::new(api_url);
                let body = json!({ "enabled": args.enabled });
                let result = client
                    .post(&format!("mocks/collections/{}/toggle", args.id), body)
                    .await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            MockCollectionCommands::Update(args) => {
                let client = ApiClient::new(api_url);
                let mut body = json!({});
                if let Some(ref name) = args.name {
                    body["name"] = Value::String(name.clone());
                }
                if let Some(ref desc) = args.description {
                    body["description"] = Value::String(desc.clone());
                }
                if let Some(enabled) = args.enabled {
                    body["enabled"] = Value::Bool(enabled);
                }
                let result = client
                    .put(&format!("mocks/collections/{}", args.id), body)
                    .await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }
        Ok(())
    }
}

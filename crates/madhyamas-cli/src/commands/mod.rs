//! CLI commands module

use anyhow::Result;
use clap::Subcommand;

mod autosave;
mod breakpoints;
mod capture;
mod config;
mod export;
mod focus;
mod grpc;
mod logs;
mod mirror;
mod mocks;
mod plugins;
mod replay;
mod rewrites;
mod scripts;
mod sessions;
mod throttle;
mod traffic;

use self::autosave::AutoSaveCommands;
use self::breakpoints::BreakpointCommands;
use self::capture::CaptureCommands;
use self::config::ConfigCommands;
use self::export::ExportCommands;
use self::focus::FocusCommands;
use self::grpc::GrpcCommands;
use self::logs::LogsCommands;
use self::mirror::MirrorCommands;
use self::mocks::MockCommands;
use self::plugins::PluginCommands;
use self::replay::ReplayCommands;
use self::rewrites::RewriteCommands;
use self::scripts::ScriptCommands;
use self::sessions::SessionCommands;
use self::throttle::ThrottleCommands;
use self::traffic::TrafficCommands;

/// Common API client for CLI commands
pub struct ApiClient {
    client: reqwest::Client,
    base_url: String,
}

impl ApiClient {
    pub fn new(base_url: String) -> Self {
        let client = reqwest::Client::new();
        Self { client, base_url }
    }

    /// Execute a GET request
    pub async fn get(&self, path: &str) -> Result<serde_json::Value> {
        let url = format!("{}/api/{}", self.base_url, path);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("API error: HTTP {} - {}", status, body);
        }

        response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))
    }

    /// Execute a POST request
    pub async fn post(&self, path: &str, body: serde_json::Value) -> Result<serde_json::Value> {
        let url = format!("{}/api/{}", self.base_url, path);
        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("API error: HTTP {} - {}", status, body);
        }

        response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))
    }

    /// Execute a PATCH request
    pub async fn patch(&self, path: &str, body: serde_json::Value) -> Result<serde_json::Value> {
        let url = format!("{}/api/{}", self.base_url, path);
        let response = self
            .client
            .patch(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("API error: HTTP {} - {}", status, body);
        }

        response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))
    }

    /// Execute a DELETE request
    pub async fn delete(&self, path: &str) -> Result<serde_json::Value> {
        let url = format!("{}/api/{}", self.base_url, path);
        let response = self
            .client
            .delete(&url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("API error: HTTP {} - {}", status, body);
        }

        response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))
    }

    /// Execute a PUT request with a JSON body.
    pub async fn put(&self, path: &str, body: serde_json::Value) -> Result<serde_json::Value> {
        let url = format!("{}/api/{}", self.base_url, path);
        let response = self
            .client
            .put(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("API error: HTTP {} - {}", status, body);
        }

        // Handle 204 No Content gracefully.
        if response.status() == reqwest::StatusCode::NO_CONTENT {
            return Ok(serde_json::Value::Null);
        }

        response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))
    }

    /// Execute a POST request without parsing the response body (for 204
    /// No Content responses).
    pub async fn post_void(&self, path: &str, body: serde_json::Value) -> Result<()> {
        let url = format!("{}/api/{}", self.base_url, path);
        let response = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("API error: HTTP {} - {}", status, body);
        }
        Ok(())
    }

    /// Execute a DELETE request without parsing the response body (for 204
    /// No Content responses).
    pub async fn delete_void(&self, path: &str) -> Result<()> {
        let url = format!("{}/api/{}", self.base_url, path);
        let response = self
            .client
            .delete(&url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("API error: HTTP {} - {}", status, body);
        }
        Ok(())
    }
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Traffic inspection commands
    #[command(subcommand)]
    Traffic(TrafficCommands),
    /// Mock response commands
    #[command(subcommand)]
    Mocks(MockCommands),
    /// Breakpoint commands
    #[command(subcommand)]
    Breakpoints(BreakpointCommands),
    /// Session management commands
    #[command(subcommand)]
    Sessions(SessionCommands),
    /// Request replay commands
    #[command(subcommand)]
    Replay(ReplayCommands),
    /// Configuration commands
    #[command(subcommand)]
    Config(ConfigCommands),
    /// Capture mode commands (recording vs passthrough)
    #[command(subcommand)]
    Capture(CaptureCommands),
    /// Throttle commands (bandwidth/latency limiting)
    #[command(subcommand)]
    Throttle(ThrottleCommands),
    /// Rewrite rule commands
    #[command(subcommand)]
    Rewrites(RewriteCommands),
    /// gRPC inspection commands
    #[command(subcommand)]
    Grpc(GrpcCommands),
    /// Script commands
    #[command(subcommand)]
    Scripts(ScriptCommands),
    /// Plugin commands
    #[command(subcommand)]
    Plugins(PluginCommands),
    /// Export commands (HAR, cURL)
    #[command(subcommand)]
    Export(ExportCommands),
    /// Focus host commands (highlight specific hosts in traffic)
    #[command(subcommand)]
    Focus(FocusCommands),
    /// Auto Save commands (periodic session backup and rotation)
    #[command(subcommand)]
    Autosave(AutoSaveCommands),
    /// Mirror commands (save response bodies to disk)
    #[command(subcommand)]
    Mirror(MirrorCommands),
    /// Log rotation commands (rotate, status, config)
    #[command(subcommand)]
    Logs(LogsCommands),
}

impl Commands {
    pub async fn execute(&self, api_url: String) -> Result<()> {
        match self {
            Commands::Traffic(cmd) => cmd.execute(api_url).await,
            Commands::Mocks(cmd) => cmd.execute(api_url).await,
            Commands::Breakpoints(cmd) => cmd.execute(api_url).await,
            Commands::Sessions(cmd) => cmd.execute(api_url).await,
            Commands::Replay(cmd) => cmd.execute(api_url).await,
            Commands::Config(cmd) => cmd.execute(api_url).await,
            Commands::Capture(cmd) => cmd.execute(api_url).await,
            Commands::Throttle(cmd) => cmd.execute(api_url).await,
            Commands::Rewrites(cmd) => cmd.execute(api_url).await,
            Commands::Grpc(cmd) => cmd.execute(api_url).await,
            Commands::Scripts(cmd) => cmd.execute(api_url).await,
            Commands::Plugins(cmd) => cmd.execute(api_url).await,
            Commands::Export(cmd) => cmd.execute(api_url).await,
            Commands::Focus(cmd) => cmd.execute(api_url).await,
            Commands::Autosave(cmd) => cmd.execute(api_url).await,
            Commands::Mirror(cmd) => cmd.execute(api_url).await,
            Commands::Logs(cmd) => cmd.execute(api_url).await,
        }
    }
}

//! CLI commands module

use anyhow::Result;
use clap::Subcommand;

mod breakpoints;
mod capture;
mod config;
mod mocks;
mod replay;
mod sessions;
mod traffic;

use self::breakpoints::BreakpointCommands;
use self::capture::CaptureCommands;
use self::config::ConfigCommands;
use self::mocks::MockCommands;
use self::replay::ReplayCommands;
use self::sessions::SessionCommands;
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
        }
    }
}

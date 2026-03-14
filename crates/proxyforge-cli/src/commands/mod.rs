//! CLI commands module

use anyhow::Result;
use clap::Subcommand;

mod traffic;
mod mocks;
mod breakpoints;
mod sessions;
mod replay;
mod config;

use self::traffic::TrafficCommands;
use self::mocks::MockCommands;
use self::breakpoints::BreakpointCommands;
use self::sessions::SessionCommands;
use self::replay::ReplayCommands;
use self::config::ConfigCommands;

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
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("API error: HTTP {} - {}", status, body);
        }

        response.json().await.map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))
    }

    /// Execute a POST request
    pub async fn post(&self, path: &str, body: serde_json::Value) -> Result<serde_json::Value> {
        let url = format!("{}/api/{}", self.base_url, path);
        let response = self.client
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

        response.json().await.map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))
    }

    /// Execute a DELETE request
    pub async fn delete(&self, path: &str) -> Result<serde_json::Value> {
        let url = format!("{}/api/{}", self.base_url, path);
        let response = self.client
            .delete(&url)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("HTTP request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("API error: HTTP {} - {}", status, body);
        }

        response.json().await.map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))
    }
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Traffic inspection commands
    Traffic(TrafficCommands),
    /// Mock response commands
    Mocks(MockCommands),
    /// Breakpoint commands
    Breakpoints(BreakpointCommands),
    /// Session management commands
    Sessions(SessionCommands),
    /// Request replay commands
    Replay(ReplayCommands),
    /// Configuration commands
    Config(ConfigCommands),
}

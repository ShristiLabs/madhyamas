//! CLI commands module

use anyhow::Result;
use clap::Subcommand;

mod autosave;
mod blocklist;
mod breakpoints;
mod capture;
mod config;
mod enterprise;
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
mod wstraffic;

use self::autosave::AutoSaveCommands;
use self::blocklist::BlockListCommands;
use self::breakpoints::BreakpointCommands;
use self::capture::CaptureCommands;
use self::config::ConfigCommands;
use self::enterprise::{AuditCommands, AuthCommands, LicenseCommands, UsersCommands};
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
use self::wstraffic::WsTrafficCommands;

/// Authentication method for CLI API calls.
///
/// In enterprise mode (with `--enable-auth`), the Madhyamas API rejects
/// unauthenticated requests with HTTP 401. The CLI attaches the configured
/// credentials to every outbound request so commands continue to work
/// behind the auth middleware. In OSS mode (or when auth is disabled),
/// [`CliAuth::None`] sends no credentials — the API server ignores them.
#[derive(Debug, Clone, Default)]
pub enum CliAuth {
    /// No authentication (OSS mode or auth disabled).
    #[default]
    None,
    /// API key authentication (`X-API-Key` header).
    ApiKey(String),
    /// JWT authentication (`Authorization: Bearer` header).
    Jwt(String),
}

impl CliAuth {
    /// Build the HTTP auth header pairs for this configuration.
    ///
    /// Returns an empty vector when no authentication is configured
    /// ([`CliAuth::None`]). The returned pairs are applied as default
    /// headers on the API client so every request carries the credentials
    /// automatically.
    pub fn auth_headers(&self) -> Vec<(String, String)> {
        match self {
            CliAuth::None => vec![],
            CliAuth::ApiKey(key) => vec![("X-API-Key".to_string(), key.clone())],
            CliAuth::Jwt(token) => {
                vec![("Authorization".to_string(), format!("Bearer {}", token))]
            }
        }
    }
}

/// Common API client for CLI commands
pub struct ApiClient {
    client: reqwest::Client,
    base_url: String,
}

impl ApiClient {
    /// Create a new API client for `base_url` with the given auth.
    ///
    /// When `auth` is configured, the credentials are applied as default
    /// headers on the underlying HTTP client so every request (GET, POST,
    /// PUT, PATCH, DELETE) carries them automatically. In OSS mode / when
    /// auth is disabled, pass [`CliAuth::None`].
    pub fn new(base_url: String, auth: CliAuth) -> Self {
        let mut default_headers = reqwest::header::HeaderMap::new();
        for (name, value) in auth.auth_headers() {
            if let (Ok(header_name), Ok(header_value)) = (
                reqwest::header::HeaderName::from_bytes(name.as_bytes()),
                reqwest::header::HeaderValue::from_str(&value),
            ) {
                default_headers.insert(header_name, header_value);
            }
        }
        let client = reqwest::Client::builder()
            .default_headers(default_headers)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
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

    /// Execute a DELETE request with a JSON body.
    pub async fn delete_with_body(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let url = format!("{}/api/{}", self.base_url, path);
        let response = self
            .client
            .delete(&url)
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
    /// Block list commands (block domains/patterns)
    #[command(subcommand)]
    Blocklist(BlockListCommands),
    /// WebSocket traffic inspection commands
    #[command(subcommand)]
    WsTraffic(WsTrafficCommands),
    /// User management commands (enterprise tier)
    #[command(subcommand)]
    Users(UsersCommands),
    /// Audit log commands (enterprise tier)
    #[command(subcommand)]
    Audit(AuditCommands),
    /// License commands (enterprise tier)
    #[command(subcommand)]
    License(LicenseCommands),
    /// Authentication commands (enterprise tier)
    #[command(subcommand)]
    Auth(AuthCommands),
}

impl Commands {
    pub async fn execute(&self, api_url: String, auth: CliAuth) -> Result<()> {
        match self {
            Commands::Traffic(cmd) => cmd.execute(api_url, auth).await,
            Commands::Mocks(cmd) => cmd.execute(api_url, auth).await,
            Commands::Breakpoints(cmd) => cmd.execute(api_url, auth).await,
            Commands::Sessions(cmd) => cmd.execute(api_url, auth).await,
            Commands::Replay(cmd) => cmd.execute(api_url, auth).await,
            Commands::Config(cmd) => cmd.execute(api_url, auth).await,
            Commands::Capture(cmd) => cmd.execute(api_url, auth).await,
            Commands::Throttle(cmd) => cmd.execute(api_url, auth).await,
            Commands::Rewrites(cmd) => cmd.execute(api_url, auth).await,
            Commands::Grpc(cmd) => cmd.execute(api_url, auth).await,
            Commands::Scripts(cmd) => cmd.execute(api_url, auth).await,
            Commands::Plugins(cmd) => cmd.execute(api_url, auth).await,
            Commands::Export(cmd) => cmd.execute(api_url, auth).await,
            Commands::Focus(cmd) => cmd.execute(api_url, auth).await,
            Commands::Autosave(cmd) => cmd.execute(api_url, auth).await,
            Commands::Mirror(cmd) => cmd.execute(api_url, auth).await,
            Commands::Logs(cmd) => cmd.execute(api_url, auth).await,
            Commands::Blocklist(cmd) => cmd.execute(api_url, auth).await,
            Commands::WsTraffic(cmd) => cmd.execute(api_url, auth).await,
            Commands::Users(cmd) => cmd.execute(api_url, auth).await,
            Commands::Audit(cmd) => cmd.execute(api_url, auth).await,
            Commands::License(cmd) => cmd.execute(api_url, auth).await,
            Commands::Auth(cmd) => cmd.execute(api_url, auth).await,
        }
    }
}

//! CLI commands module

use anyhow::Result;
use clap::Subcommand;

mod autosave;
mod blocklist;
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
mod wstraffic;

use self::autosave::AutoSaveCommands;
use self::blocklist::BlockListCommands;
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_auth_none_headers() {
        assert!(CliAuth::None.auth_headers().is_empty());
        assert!(matches!(CliAuth::None, CliAuth::None));
    }

    #[test]
    fn test_cli_auth_api_key_headers() {
        let auth = CliAuth::ApiKey("cli-key-xyz".to_string());
        let headers = auth.auth_headers();
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].0, "X-API-Key");
        assert_eq!(headers[0].1, "cli-key-xyz");
        assert!(!matches!(auth, CliAuth::None));
    }

    #[test]
    fn test_cli_auth_jwt_headers() {
        let auth = CliAuth::Jwt("cli-jwt-abc".to_string());
        let headers = auth.auth_headers();
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].0, "Authorization");
        assert_eq!(headers[0].1, "Bearer cli-jwt-abc");
        assert!(!matches!(auth, CliAuth::None));
    }

    #[test]
    fn test_cli_auth_default_none() {
        let auth = CliAuth::default();
        assert!(matches!(auth, CliAuth::None));
        assert!(auth.auth_headers().is_empty());
    }

    /// Spawn a minimal mock HTTP server that captures the raw request
    /// headers from a single connection and returns a JSON `null` body.
    async fn spawn_mock_server() -> (String, tokio::sync::oneshot::Receiver<String>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = tokio::sync::oneshot::channel();
        let url = format!("http://{}", addr);

        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut buf = vec![0u8; 8192];
            let n = socket.read(&mut buf).await.unwrap();
            let request_text = String::from_utf8_lossy(&buf[..n]).to_string();
            let _ = tx.send(request_text);
            let body = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 4\r\n\r\nnull";
            socket.write_all(body).await.unwrap();
            socket.flush().await.unwrap();
        });

        (url, rx)
    }

    #[tokio::test]
    async fn test_cli_client_sends_api_key_header() {
        let (url, rx) = spawn_mock_server().await;
        let client = ApiClient::new(url, CliAuth::ApiKey("cli-key-xyz".to_string()));
        let _ = client.get("traffic").await;
        let request = rx.await.unwrap();
        let lower = request.to_ascii_lowercase();
        assert!(
            lower.contains("x-api-key: cli-key-xyz"),
            "request missing X-API-Key header: {}",
            request
        );
    }

    #[tokio::test]
    async fn test_cli_client_sends_jwt_header() {
        let (url, rx) = spawn_mock_server().await;
        let client = ApiClient::new(url, CliAuth::Jwt("cli-jwt-abc".to_string()));
        let _ = client.get("traffic").await;
        let request = rx.await.unwrap();
        let lower = request.to_ascii_lowercase();
        assert!(
            lower.contains("authorization: bearer cli-jwt-abc"),
            "request missing Authorization header: {}",
            request
        );
    }

    #[tokio::test]
    async fn test_cli_client_without_auth_sends_no_auth_headers() {
        let (url, rx) = spawn_mock_server().await;
        let client = ApiClient::new(url, CliAuth::None);
        let _ = client.get("traffic").await;
        let request = rx.await.unwrap();
        let lower = request.to_ascii_lowercase();
        assert!(
            !lower.contains("x-api-key"),
            "unexpected X-API-Key header: {}",
            request
        );
        assert!(
            !lower.contains("authorization:"),
            "unexpected Authorization header: {}",
            request
        );
    }
}

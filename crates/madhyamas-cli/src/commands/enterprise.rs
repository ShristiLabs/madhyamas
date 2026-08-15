//! Enterprise CLI commands.
//!
//! These subcommands expose enterprise-tier API endpoints (user
//! management, audit logging, licensing, and authentication) via the
//! CLI. They work against any Madhyamas API server; against an OSS
//! server the enterprise endpoints return 404 and the commands surface
//! the error gracefully.

use anyhow::Result;
use clap::{Args, Subcommand};
use serde_json::Value;

use super::ApiClient;

// ============ Users ============

#[derive(Debug, Args)]
pub struct UsersListArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct UsersCreateArgs {
    /// Username for the new user
    #[arg(long)]
    pub username: String,

    /// Email address
    #[arg(long)]
    pub email: String,

    /// Initial password
    #[arg(long)]
    pub password: String,

    /// User role (admin, user, viewer)
    #[arg(long)]
    pub role: String,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct UsersDeleteArgs {
    /// User ID
    #[arg(long)]
    pub id: String,
}

#[derive(Debug, Args)]
pub struct UsersUpdateRoleArgs {
    /// User ID
    #[arg(long)]
    pub id: String,

    /// New role (admin, user, viewer)
    #[arg(long)]
    pub role: String,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Subcommand)]
pub enum UsersCommands {
    /// List all users
    List(UsersListArgs),
    /// Create a new user
    Create(UsersCreateArgs),
    /// Delete a user
    Delete(UsersDeleteArgs),
    /// Update a user's role
    UpdateRole(UsersUpdateRoleArgs),
}

impl UsersCommands {
    pub async fn execute(&self, api_url: String, auth: super::super::CliAuth) -> Result<()> {
        match self {
            UsersCommands::List(args) => {
                let client = ApiClient::new(api_url, auth.clone());
                let result = client.get("users").await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    print_users_list(&result);
                }
            }
            UsersCommands::Create(args) => {
                let client = ApiClient::new(api_url, auth.clone());
                let body = serde_json::json!({
                    "username": args.username,
                    "email": args.email,
                    "password": args.password,
                    "role": args.role,
                });
                let result = client.post("users", body).await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    let id = result.get("id").and_then(|v| v.as_str()).unwrap_or("-");
                    let username = result
                        .get("username")
                        .and_then(|v| v.as_str())
                        .unwrap_or("-");
                    println!("Created user {} (id: {})", username, id);
                }
            }
            UsersCommands::Delete(args) => {
                let client = ApiClient::new(api_url, auth.clone());
                client.delete_void(&format!("users/{}", args.id)).await?;
                println!("Deleted user {}", args.id);
            }
            UsersCommands::UpdateRole(args) => {
                let client = ApiClient::new(api_url, auth.clone());
                let body = serde_json::json!({ "role": args.role });
                let result = client.put(&format!("users/{}", args.id), body).await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("Updated user {} role to {}", args.id, args.role);
                }
            }
        }
        Ok(())
    }
}

fn print_users_list(result: &Value) {
    if let Some(users) = result.as_array() {
        if users.is_empty() {
            println!("No users found.");
            return;
        }
        println!(
            "{:<36} {:<20} {:<30} {:<10}",
            "ID", "USERNAME", "EMAIL", "ROLE"
        );
        println!("{}", "-".repeat(100));
        for user in users {
            let id = user.get("id").and_then(|v| v.as_str()).unwrap_or("-");
            let username = user.get("username").and_then(|v| v.as_str()).unwrap_or("-");
            let email = user.get("email").and_then(|v| v.as_str()).unwrap_or("-");
            let role = user.get("role").and_then(|v| v.as_str()).unwrap_or("-");
            println!("{:<36} {:<20} {:<30} {:<10}", id, username, email, role);
        }
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(result).unwrap_or_default()
        );
    }
}

// ============ Audit ============

#[derive(Debug, Args)]
pub struct AuditListArgs {
    /// Filter by user ID
    #[arg(long)]
    pub user_id: Option<String>,

    /// Filter by event type
    #[arg(long)]
    pub event_type: Option<String>,

    /// Maximum number of results
    #[arg(long, default_value = "100")]
    pub limit: usize,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Subcommand)]
pub enum AuditCommands {
    /// List audit events
    List(AuditListArgs),
    /// Export all audit events
    Export,
    /// Show audit statistics
    Stats,
}

impl AuditCommands {
    pub async fn execute(&self, api_url: String, auth: super::super::CliAuth) -> Result<()> {
        match self {
            AuditCommands::List(args) => {
                let client = ApiClient::new(api_url, auth.clone());
                let mut parts: Vec<String> = Vec::new();
                if let Some(ref uid) = args.user_id {
                    parts.push(format!("user_id={}", uid));
                }
                if let Some(ref et) = args.event_type {
                    parts.push(format!("event_types={}", et));
                }
                parts.push(format!("limit={}", args.limit));
                let path = format!("audit?{}", parts.join("&"));
                let result = client.get(&path).await?;
                if args.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    print_audit_list(&result);
                }
            }
            AuditCommands::Export => {
                let client = ApiClient::new(api_url, auth.clone());
                let result = client.get("audit/export").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            AuditCommands::Stats => {
                let client = ApiClient::new(api_url, auth.clone());
                let result = client.get("audit/stats").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }
        Ok(())
    }
}

fn print_audit_list(result: &Value) {
    if let Some(events) = result.as_array() {
        if events.is_empty() {
            println!("No audit events found.");
            return;
        }
        println!(
            "{:<36} {:<20} {:<10} {:<30}",
            "ID", "USER_ID", "TYPE", "TIMESTAMP"
        );
        println!("{}", "-".repeat(100));
        for event in events {
            let id = event.get("id").and_then(|v| v.as_str()).unwrap_or("-");
            let user_id = event.get("user_id").and_then(|v| v.as_str()).unwrap_or("-");
            let event_type = event
                .get("event_type")
                .and_then(|v| v.as_str())
                .unwrap_or("-");
            let timestamp = event
                .get("timestamp")
                .and_then(|v| v.as_str())
                .unwrap_or("-");
            println!(
                "{:<36} {:<20} {:<10} {:<30}",
                id, user_id, event_type, timestamp
            );
        }
    } else {
        println!(
            "{}",
            serde_json::to_string_pretty(result).unwrap_or_default()
        );
    }
}

// ============ License ============

#[derive(Debug, Subcommand)]
pub enum LicenseCommands {
    /// Show license information
    Info,
}

impl LicenseCommands {
    pub async fn execute(&self, api_url: String, auth: super::super::CliAuth) -> Result<()> {
        match self {
            LicenseCommands::Info => {
                let client = ApiClient::new(api_url, auth.clone());
                let result = client.get("license").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        }
        Ok(())
    }
}

// ============ Auth ============

#[derive(Debug, Args)]
pub struct AuthLoginArgs {
    /// Username
    #[arg(long)]
    pub username: String,

    /// Password
    #[arg(long)]
    pub password: String,
}

#[derive(Debug, Args)]
pub struct ApiKeyCreateArgs {
    /// Name for the API key
    #[arg(long)]
    pub name: String,

    /// Comma-separated scopes (e.g. "*:*" or "traffic:read,config:read")
    #[arg(long, default_value = "*:*")]
    pub scopes: String,
}

#[derive(Debug, Args)]
pub struct ApiKeyRevokeArgs {
    /// API key ID
    #[arg(long)]
    pub id: String,
}

#[derive(Debug, Subcommand)]
pub enum ApiKeysCommands {
    /// List all API keys
    List,
    /// Create a new API key
    Create(ApiKeyCreateArgs),
    /// Revoke an API key
    Revoke(ApiKeyRevokeArgs),
}

#[derive(Debug, Subcommand)]
pub enum AuthCommands {
    /// Login and obtain a JWT token
    Login(AuthLoginArgs),
    /// Logout (invalidate current session)
    Logout,
    /// Manage API keys
    #[command(subcommand)]
    ApiKeys(ApiKeysCommands),
}

impl AuthCommands {
    pub async fn execute(&self, api_url: String, auth: super::super::CliAuth) -> Result<()> {
        match self {
            AuthCommands::Login(args) => {
                let client = ApiClient::new(api_url, super::super::CliAuth::None);
                let body = serde_json::json!({
                    "username": args.username,
                    "password": args.password,
                });
                let result = client.post("auth/login", body).await?;
                if let Some(token) = result.get("token").and_then(|v| v.as_str()) {
                    println!("{}", token);
                } else {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                }
            }
            AuthCommands::Logout => {
                let client = ApiClient::new(api_url, auth.clone());
                client
                    .post_void("auth/logout", serde_json::json!({}))
                    .await?;
                println!("Logged out.");
            }
            AuthCommands::ApiKeys(cmd) => {
                cmd.execute(api_url, auth).await?;
            }
        }
        Ok(())
    }
}

impl ApiKeysCommands {
    pub async fn execute(&self, api_url: String, auth: super::super::CliAuth) -> Result<()> {
        match self {
            ApiKeysCommands::List => {
                let client = ApiClient::new(api_url, auth.clone());
                let result = client.get("auth/api-keys").await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            ApiKeysCommands::Create(args) => {
                let client = ApiClient::new(api_url, auth.clone());
                let scopes: Vec<String> = args
                    .scopes
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();
                let body = serde_json::json!({
                    "name": args.name,
                    "scopes": scopes,
                });
                let result = client.post("auth/api-keys", body).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            ApiKeysCommands::Revoke(args) => {
                let client = ApiClient::new(api_url, auth.clone());
                client
                    .delete_void(&format!("auth/api-keys/{}", args.id))
                    .await?;
                println!("Revoked API key {}", args.id);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_users_commands_enum() {
        let cmd = UsersCommands::List(UsersListArgs { json: false });
        assert!(matches!(cmd, UsersCommands::List(_)));
    }

    #[test]
    fn test_audit_commands_enum() {
        let cmd = AuditCommands::Stats;
        assert!(matches!(cmd, AuditCommands::Stats));
    }

    #[test]
    fn test_license_commands_enum() {
        let cmd = LicenseCommands::Info;
        assert!(matches!(cmd, LicenseCommands::Info));
    }

    #[test]
    fn test_auth_commands_enum() {
        let cmd = AuthCommands::Logout;
        assert!(matches!(cmd, AuthCommands::Logout));
    }

    #[test]
    fn test_api_keys_commands_enum() {
        let cmd = ApiKeysCommands::List;
        assert!(matches!(cmd, ApiKeysCommands::List));
    }

    #[tokio::test]
    async fn test_enterprise_cli_users_list() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("http://{}", addr);

        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 8192];
            let _ = socket.read(&mut buf).await;
            let body =
                b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n[]";
            let _ = socket.write_all(body).await;
            let _ = socket.flush().await;
        });

        let client = ApiClient::new(url, super::super::CliAuth::None);
        let result = client.get("users").await.unwrap();
        assert!(result.is_array());
    }

    #[tokio::test]
    async fn test_enterprise_cli_license_info() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("http://{}", addr);

        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 8192];
            let _ = socket.read(&mut buf).await;
            let body = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 18\r\n\r\n{\"licensed\":false}";
            let _ = socket.write_all(body).await;
            let _ = socket.flush().await;
        });

        let client = ApiClient::new(url, super::super::CliAuth::None);
        let result = client.get("license").await.unwrap();
        assert_eq!(
            result.get("licensed").and_then(|v| v.as_bool()),
            Some(false)
        );
    }

    #[tokio::test]
    async fn test_enterprise_cli_audit_list() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("http://{}", addr);

        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 8192];
            let _ = socket.read(&mut buf).await;
            let body =
                b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n[]";
            let _ = socket.write_all(body).await;
            let _ = socket.flush().await;
        });

        let client = ApiClient::new(url, super::super::CliAuth::None);
        let result = client.get("audit?limit=100").await.unwrap();
        assert!(result.is_array());
    }
}

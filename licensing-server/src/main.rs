//! Madhyamas licensing server — issues, verifies, and manages Ed25519-signed
//! enterprise licenses.
//!
//! This is a **separate application** from the Madhyamas proxy binary. It runs
//! as a standalone axum web server backed by PostgreSQL. The proxy binary
//! verifies licenses offline (using only the Ed25519 public key); this server
//! is the authority that signs and manages licenses.
//!
//! # Quick start
//!
//! ```sh
//! madhyamas-licensing \
//!   --database-url "postgres://user:pass@localhost:5432/madhyamas" \
//!   --port 8080 \
//!   --ed25519-private-key-file /path/to/private.key \
//!   --admin-key dev
//! ```
//!
//! When `--ed25519-private-key-file` is omitted, a fresh keypair is generated
//! on startup (development mode). Use `--generate-keys --output-dir <dir>` to
//! create a persistent keypair for production.

use madhyamas_licensing::api;
use madhyamas_licensing::db;
use madhyamas_licensing::license;

use base64::Engine as _;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;

/// CLI arguments for the licensing server.
#[derive(Parser, Debug)]
#[command(
    name = "madhyamas-licensing",
    about = "Madhyamas enterprise licensing server"
)]
struct Cli {
    /// PostgreSQL connection URL.
    #[arg(long, env = "DATABASE_URL")]
    database_url: Option<String>,

    /// Port to listen on.
    #[arg(long, env = "PORT", default_value = "8080")]
    port: u16,

    /// Path to the Ed25519 private key file (32 raw bytes or base64). If
    /// omitted, a fresh keypair is generated on startup (development only).
    #[arg(long, env = "ED25519_PRIVATE_KEY_FILE")]
    ed25519_private_key_file: Option<PathBuf>,

    /// Path to the Ed25519 public key file (32 raw bytes or base64). If
    /// omitted, the public key is derived from the private key (or the
    /// generated keypair).
    #[arg(long, env = "ED25519_PUBLIC_KEY_FILE")]
    ed25519_public_key_file: Option<PathBuf>,

    /// Admin API key for X-Admin-Key header authentication. Defaults to "dev"
    /// for local development. **Set a strong value in production.**
    #[arg(long, env = "ADMIN_KEY", default_value = "dev")]
    admin_key: String,

    /// Bind address.
    #[arg(long, env = "BIND_ADDR", default_value = "0.0.0.0")]
    bind_addr: String,

    #[command(subcommand)]
    command: Option<Command>,
}

/// Subcommands.
#[derive(Subcommand, Debug)]
enum Command {
    /// Generate a fresh Ed25519 keypair and write the private and public keys
    /// to the output directory.
    GenerateKeys {
        /// Directory to write the key files into.
        #[arg(long)]
        output_dir: PathBuf,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    if let Some(Command::GenerateKeys { output_dir }) = &cli.command {
        return generate_keys(output_dir);
    }

    run_server(cli).await
}

/// Generate a keypair and write private/public key files.
fn generate_keys(output_dir: &std::path::Path) -> anyhow::Result<()> {
    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::Engine as _;

    std::fs::create_dir_all(output_dir)?;
    let (signer, public_key) = license::LicenseSigner::generate();

    let private_path = output_dir.join("ed25519_private.key");
    let public_path = output_dir.join("ed25519_public.key");

    std::fs::write(&private_path, signer.raw_private_key())?;
    std::fs::write(&public_path, public_key.to_bytes())?;

    let public_b64 = BASE64.encode(public_key.to_bytes());
    let private_b64 = BASE64.encode(signer.raw_private_key());

    println!("Ed25519 keypair generated:");
    println!("  Private key (raw):  {}", private_path.display());
    println!("  Public key  (raw):  {}", public_path.display());
    println!();
    println!(
        "  Public key  (base64)  — set this as MADHYAMAS_LICENSE_PUBLIC_KEY on proxy instances:"
    );
    println!("  {public_b64}");
    println!();
    println!("  Private key (base64): {private_b64}");
    println!();
    println!("IMPORTANT: Store the private key in a secrets manager. Never commit it to git.");
    Ok(())
}

/// Start the axum server.
async fn run_server(cli: Cli) -> anyhow::Result<()> {
    let database_url = cli
        .database_url
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("--database-url or DATABASE_URL env var is required"))?;

    let signer = match &cli.ed25519_private_key_file {
        Some(path) => {
            tracing::info!(path = %path.display(), "loading Ed25519 private key");
            license::LicenseSigner::from_file(path)?
        }
        None => {
            tracing::warn!(
                "no --ed25519-private-key-file provided; generating a fresh keypair \
                 (development mode). Licenses signed with this key will NOT verify on \
                 proxy instances configured with a different public key."
            );
            let (s, _) = license::LicenseSigner::generate();
            s
        }
    };

    let public_key = match &cli.ed25519_public_key_file {
        Some(path) => {
            let contents = std::fs::read_to_string(path)?;
            let trimmed = contents.trim();
            let bytes = if trimmed.len() == 32 {
                trimmed.as_bytes().to_vec()
            } else {
                base64::engine::general_purpose::STANDARD
                    .decode(trimmed.as_bytes())
                    .map_err(|e| anyhow::anyhow!("invalid public key base64: {e}"))?
            };
            if bytes.len() != 32 {
                anyhow::bail!("expected 32-byte public key, got {} bytes", bytes.len());
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            ed25519_dalek::VerifyingKey::from_bytes(&arr)
                .map_err(|e| anyhow::anyhow!("invalid public key: {e}"))?
        }
        None => signer.verifying_key(),
    };

    tracing::info!("connecting to PostgreSQL...");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await?;

    tracing::info!("initializing database schema...");
    db::init_schema(&pool).await?;

    // Bootstrap a default admin on first run (Phase 12d.1).
    bootstrap_admin(&pool).await?;

    let stripe_api_key = std::env::var("STRIPE_API_KEY")
        .ok()
        .filter(|s| !s.is_empty());
    let stripe_webhook_secret = std::env::var("STRIPE_WEBHOOK_SECRET")
        .ok()
        .filter(|s| !s.is_empty());

    if stripe_api_key.is_none() {
        tracing::warn!(
            "STRIPE_API_KEY is not set — billing endpoints will return 503. \
             Set STRIPE_API_KEY, STRIPE_PRICE_STARTER, STRIPE_PRICE_PRO, \
             STRIPE_PRICE_ENTERPRISE, and STRIPE_WEBHOOK_SECRET to enable Stripe."
        );
    }

    let state = api::AppState {
        pool,
        signer: Arc::new(signer),
        public_key,
        admin_key: cli.admin_key,
        stripe_api_key,
        stripe_webhook_secret,
    };

    let api_router = api::router(state);

    // Serve the customer portal frontend (if built) at /portal/*.
    let portal_dir = std::env::var("PORTAL_DIST_DIR")
        .unwrap_or_else(|_| "licensing-server/web/dist".to_string());
    let portal_path = std::path::Path::new(&portal_dir);
    let app = if portal_path.is_dir() {
        tracing::info!(dir = %portal_dir, "serving customer portal at /portal/*");
        api_router.nest_service("/portal", tower_http::services::ServeDir::new(portal_path))
    } else {
        tracing::debug!(dir = %portal_dir, "portal dist dir not found — portal disabled");
        api_router
    };

    let addr = format!("{}:{}", cli.bind_addr, cli.port);
    tracing::info!(addr = %addr, "licensing server listening");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Create a default admin account on first run using `ADMIN_EMAIL` and
/// `ADMIN_PASSWORD` env vars. If no admins exist and the env vars are unset,
/// a development admin (`admin@madhyamas.local` / `admin123`) is created with
/// a warning. This is idempotent — if admins already exist, it does nothing.
async fn bootstrap_admin(pool: &sqlx::PgPool) -> anyhow::Result<()> {
    let count = db::count_admins(pool).await?;
    if count > 0 {
        tracing::debug!(count, "admins already exist — skipping bootstrap");
        return Ok(());
    }

    let email =
        std::env::var("ADMIN_EMAIL").unwrap_or_else(|_| "admin@madhyamas.local".to_string());
    let password = std::env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "admin123".to_string());
    let role = "super_admin";

    if std::env::var("ADMIN_PASSWORD").is_err() {
        tracing::warn!(
            "ADMIN_PASSWORD env var not set — creating default admin \
             (admin@madhyamas.local / admin123). Set ADMIN_EMAIL and \
             ADMIN_PASSWORD for production."
        );
    }

    let hash = madhyamas_licensing::auth::hash_password(&password)?;
    db::insert_admin(pool, uuid::Uuid::new_v4(), &email, &hash, role).await?;

    tracing::info!(email = %email, role, "default admin created");
    Ok(())
}

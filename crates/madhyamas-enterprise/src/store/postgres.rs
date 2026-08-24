//! PostgreSQL-backed [`EnterpriseStore`] implementation using [`sqlx::PgPool`].
//!
//! The constructor runs idempotent `CREATE TABLE IF NOT EXISTS` DDL for the
//! four enterprise tables (`users`, `api_keys`, `auth_sessions`,
//! `audit_events`). All queries use runtime SQL strings with `$N`
//! placeholders so the crate compiles without a database at build time.

use std::collections::HashMap;

use async_trait::async_trait;
use chrono::Utc;
use sqlx::PgPool;

use super::types::{AuditEventRecord, UserRecord};
use super::{
    ApiKeyRecord, AuditEvent, AuditFilter, AuditStats, AuthSession, EnterpriseStore, Result,
    UserUpdate,
};
use crate::user::User;

/// PostgreSQL-backed enterprise store.
pub struct PostgresEnterpriseStore {
    pool: PgPool,
}

impl PostgresEnterpriseStore {
    /// Create a new store over the given pool, ensuring the schema exists.
    ///
    /// All DDL is wrapped in a single advisory-lock transaction to prevent
    /// concurrent schema initialization across multi-instance deployments.
    /// The lock key `0x4D414448` ("MADH") matches the one used by
    /// `run_pg_migrations()` in the main binary and by
    /// `PostgresTrafficStore::create_tables()`, so all DDL across the
    /// application is serialized. The lock is transaction-scoped, so it is
    /// released on commit/rollback and does not block normal operations.
    pub async fn new(pool: PgPool) -> Result<Self> {
        let mut tx = pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(0x4D414448)")
            .execute(&mut *tx)
            .await?;
        sqlx::query(SCHEMA_USERS).execute(&mut *tx).await?;
        sqlx::query(SCHEMA_API_KEYS).execute(&mut *tx).await?;
        sqlx::query(SCHEMA_AUTH_SESSIONS).execute(&mut *tx).await?;
        sqlx::query(SCHEMA_AUDIT_EVENTS).execute(&mut *tx).await?;
        sqlx::query(SCHEMA_SECRETS).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(Self { pool })
    }

    /// Borrow the underlying pool (used by callers that need direct access).
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

const SCHEMA_USERS: &str = "CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    email TEXT,
    display_name TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at BIGINT NOT NULL,
    last_login BIGINT,
    preferences TEXT NOT NULL
)";

const SCHEMA_API_KEYS: &str = "CREATE TABLE IF NOT EXISTS api_keys (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    name TEXT NOT NULL,
    key_hash TEXT UNIQUE NOT NULL,
    key_prefix TEXT NOT NULL,
    scopes TEXT NOT NULL,
    expires_at TEXT,
    last_used_at TEXT,
    created_at TEXT NOT NULL
)";

const SCHEMA_AUTH_SESSIONS: &str = "CREATE TABLE IF NOT EXISTS auth_sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    jwt_jti TEXT NOT NULL,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    last_activity TEXT NOT NULL,
    revoked BOOLEAN NOT NULL
)";

const SCHEMA_AUDIT_EVENTS: &str = "CREATE TABLE IF NOT EXISTS audit_events (
    id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    user_id TEXT,
    api_key_id TEXT,
    client_ip TEXT,
    description TEXT NOT NULL,
    metadata TEXT NOT NULL,
    prev_hash TEXT,
    hash TEXT
)";

const SCHEMA_SECRETS: &str = "CREATE TABLE IF NOT EXISTS secrets (
    name TEXT PRIMARY KEY,
    nonce TEXT NOT NULL,
    ciphertext TEXT NOT NULL,
    updated_at TEXT NOT NULL
)";

#[async_trait]
impl EnterpriseStore for PostgresEnterpriseStore {
    async fn set_secret(&self, name: &str, nonce: &str, ciphertext: &str) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO secrets (name, nonce, ciphertext, updated_at) VALUES ($1, $2, $3, $4) \
             ON CONFLICT (name) DO UPDATE SET nonce = EXCLUDED.nonce, \
             ciphertext = EXCLUDED.ciphertext, updated_at = EXCLUDED.updated_at",
        )
        .bind(name)
        .bind(nonce)
        .bind(ciphertext)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_secret(&self, name: &str) -> Result<bool> {
        let res = sqlx::query("DELETE FROM secrets WHERE name = $1")
            .bind(name)
            .execute(&self.pool)
            .await?;
        Ok(res.rows_affected() > 0)
    }

    async fn list_secrets(&self) -> Result<Vec<(String, String, String)>> {
        let rows = sqlx::query_as::<_, (String, String, String)>(
            "SELECT name, nonce, ciphertext FROM secrets ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn create_user(&self, user: &User, password_hash: &str) -> Result<()> {
        let rec = UserRecord::from(user);
        sqlx::query(
            "INSERT INTO users \
             (id, username, email, display_name, password_hash, role, status, created_at, last_login, preferences) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
             ON CONFLICT (username) DO NOTHING",
        )
        .bind(&rec.id)
        .bind(&rec.username)
        .bind(&rec.email)
        .bind(&rec.display_name)
        .bind(password_hash)
        .bind(&rec.role)
        .bind(&rec.status)
        .bind(rec.created_at)
        .bind(rec.last_login)
        .bind(&rec.preferences)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_user(&self, id: &str) -> Result<Option<User>> {
        let row: Option<UserRecord> =
            sqlx::query_as::<_, UserRecord>("SELECT * FROM users WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.map(User::from))
    }

    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
        let row: Option<UserRecord> =
            sqlx::query_as::<_, UserRecord>("SELECT * FROM users WHERE username = $1")
                .bind(username)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.map(User::from))
    }

    async fn get_user_credentials(&self, username: &str) -> Result<Option<(User, String)>> {
        let row: Option<UserRecord> =
            sqlx::query_as::<_, UserRecord>("SELECT * FROM users WHERE username = $1")
                .bind(username)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.map(|r| {
            let hash = r.password_hash.clone();
            (User::from(r), hash)
        }))
    }

    async fn list_users(&self) -> Result<Vec<User>> {
        let rows: Vec<UserRecord> =
            sqlx::query_as::<_, UserRecord>("SELECT * FROM users ORDER BY created_at ASC")
                .fetch_all(&self.pool)
                .await?;
        Ok(rows.into_iter().map(User::from).collect())
    }

    async fn update_user(&self, id: &str, updates: &UserUpdate) -> Result<()> {
        let mut sets: Vec<&'static str> = Vec::new();
        let mut param_idx = 1;
        if updates.username.is_some() {
            sets.push("username = $");
            param_idx += 1;
        }
        if updates.email.is_some() {
            sets.push("email = $");
            param_idx += 1;
        }
        if updates.password_hash.is_some() {
            sets.push("password_hash = $");
            param_idx += 1;
        }
        if updates.role.is_some() {
            sets.push("role = $");
            param_idx += 1;
        }
        if updates.status.is_some() {
            sets.push("status = $");
            param_idx += 1;
        }
        if updates.preferences.is_some() {
            sets.push("preferences = $");
            param_idx += 1;
        }
        if updates.last_login.is_some() {
            sets.push("last_login = $");
            param_idx += 1;
        }
        if sets.is_empty() {
            return Ok(());
        }
        // Build the SET clause with numbered placeholders.
        let set_parts: Vec<String> = sets
            .iter()
            .enumerate()
            .map(|(i, s)| format!("{}{}", s.trim_end_matches('$'), i + 1))
            .collect();
        let sql = format!(
            "UPDATE users SET {} WHERE id = ${}",
            set_parts.join(", "),
            param_idx
        );
        let mut q = sqlx::query(&sql);
        if let Some(ref v) = updates.username {
            q = q.bind(v);
        }
        if let Some(ref v) = updates.email {
            q = q.bind(v);
        }
        if let Some(ref v) = updates.password_hash {
            q = q.bind(v);
        }
        if let Some(ref v) = updates.role {
            q = q.bind(v);
        }
        if let Some(ref v) = updates.status {
            q = q.bind(v);
        }
        if let Some(ref v) = updates.preferences {
            q = q.bind(v);
        }
        if let Some(v) = updates.last_login {
            q = q.bind(v);
        }
        q.bind(id).execute(&self.pool).await?;
        Ok(())
    }

    async fn delete_user(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn create_api_key(&self, key: &ApiKeyRecord) -> Result<()> {
        sqlx::query(
            "INSERT INTO api_keys \
             (id, user_id, name, key_hash, key_prefix, scopes, expires_at, last_used_at, created_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(&key.id)
        .bind(&key.user_id)
        .bind(&key.name)
        .bind(&key.key_hash)
        .bind(&key.key_prefix)
        .bind(&key.scopes)
        .bind(&key.expires_at)
        .bind(&key.last_used_at)
        .bind(&key.created_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_api_key_by_hash(&self, hash: &str) -> Result<Option<ApiKeyRecord>> {
        let row: Option<ApiKeyRecord> =
            sqlx::query_as::<_, ApiKeyRecord>("SELECT * FROM api_keys WHERE key_hash = $1")
                .bind(hash)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row)
    }

    async fn list_api_keys(&self, user_id: &str) -> Result<Vec<ApiKeyRecord>> {
        let rows: Vec<ApiKeyRecord> = sqlx::query_as::<_, ApiKeyRecord>(
            "SELECT * FROM api_keys WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn revoke_api_key(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM api_keys WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn update_api_key_last_used(&self, id: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE api_keys SET last_used_at = $1 WHERE id = $2")
            .bind(&now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn create_session(&self, session: &AuthSession) -> Result<()> {
        sqlx::query(
            "INSERT INTO auth_sessions \
             (id, user_id, jwt_jti, created_at, expires_at, last_activity, revoked) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) \
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(&session.id)
        .bind(&session.user_id)
        .bind(&session.jwt_jti)
        .bind(&session.created_at)
        .bind(&session.expires_at)
        .bind(&session.last_activity)
        .bind(session.revoked)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_session(&self, id: &str) -> Result<Option<AuthSession>> {
        let row: Option<AuthSession> =
            sqlx::query_as::<_, AuthSession>("SELECT * FROM auth_sessions WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row)
    }

    async fn revoke_session(&self, id: &str) -> Result<()> {
        sqlx::query("UPDATE auth_sessions SET revoked = TRUE WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn cleanup_expired_sessions(&self) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("DELETE FROM auth_sessions WHERE expires_at < $1 AND revoked = FALSE")
            .bind(&now)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn update_session_activity(&self, session_id: &str) -> Result<()> {
        let now = Utc::now().to_rfc3339();
        sqlx::query("UPDATE auth_sessions SET last_activity = $1 WHERE id = $2")
            .bind(&now)
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn log_audit_event(&self, event: &AuditEvent) -> Result<()> {
        // Use a transaction-scoped advisory lock to serialize audit event
        // insertion. This ensures the hash chain remains consistent across
        // concurrent inserts from multiple instances: the "read last hash"
        // and "insert new event" steps are atomic within the locked
        // transaction, so two concurrent inserts can never reference the
        // same `prev_hash`.
        // Lock key: 0x4D414450 ("MADP" for Madhyamas audit chain), distinct
        // from the DDL lock (0x4D414448) and the prune lock (0x4D414449).
        let mut tx = self.pool.begin().await?;
        sqlx::query("SELECT pg_advisory_xact_lock(0x4D414450)")
            .execute(&mut *tx)
            .await?;

        // Get the last event's hash within the locked transaction. Order by
        // timestamp then id to break ties deterministically.
        let prev_hash: Option<String> = sqlx::query_scalar(
            "SELECT hash FROM audit_events ORDER BY timestamp DESC, id DESC LIMIT 1",
        )
        .fetch_optional(&mut *tx)
        .await?;

        // Recompute the hash chain based on the authoritative last hash read
        // under the lock, rather than trusting the (possibly stale) hash
        // computed by the in-memory ring buffer.
        let mut event = event.clone();
        event.prev_hash = prev_hash.clone();
        event.hash = Some(crate::audit::AuditLogger::compute_hash(&event));

        let rec = AuditEventRecord::from(&event);
        sqlx::query(
            "INSERT INTO audit_events \
             (id, event_type, timestamp, user_id, api_key_id, client_ip, description, metadata, prev_hash, hash) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(&rec.id)
        .bind(&rec.event_type)
        .bind(&rec.timestamp)
        .bind(&rec.user_id)
        .bind(&rec.api_key_id)
        .bind(&rec.client_ip)
        .bind(&rec.description)
        .bind(&rec.metadata)
        .bind(&rec.prev_hash)
        .bind(&rec.hash)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn query_audit_events(&self, filter: &AuditFilter) -> Result<Vec<AuditEvent>> {
        let mut where_clauses: Vec<String> = Vec::new();
        let mut param_idx = 1;
        if filter.event_type.is_some() {
            where_clauses.push(format!("event_type = ${}", param_idx));
            param_idx += 1;
        }
        if filter.user_id.is_some() {
            where_clauses.push(format!("user_id = ${}", param_idx));
            param_idx += 1;
        }
        if filter.start_time.is_some() {
            where_clauses.push(format!("timestamp >= ${}", param_idx));
            param_idx += 1;
        }
        if filter.end_time.is_some() {
            where_clauses.push(format!("timestamp <= ${}", param_idx));
            param_idx += 1;
        }
        let where_sql = if where_clauses.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", where_clauses.join(" AND "))
        };
        let limit = filter.limit.unwrap_or(1000);
        let offset = filter.offset.unwrap_or(0);
        let sql = format!(
            "SELECT * FROM audit_events{} ORDER BY timestamp DESC LIMIT ${} OFFSET ${}",
            where_sql,
            param_idx,
            param_idx + 1
        );
        let mut q = sqlx::query_as::<_, AuditEventRecord>(&sql);
        if let Some(t) = filter.event_type {
            q = q.bind(event_type_label(t));
        }
        if let Some(ref u) = filter.user_id {
            q = q.bind(u);
        }
        if let Some(t) = filter.start_time {
            q = q.bind(t.to_rfc3339());
        }
        if let Some(t) = filter.end_time {
            q = q.bind(t.to_rfc3339());
        }
        let rows = q
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(AuditEvent::from).collect())
    }

    async fn get_audit_stats(&self) -> Result<AuditStats> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_events")
            .fetch_one(&self.pool)
            .await?;
        let today = Utc::now()
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap_or_default()
            .and_utc()
            .to_rfc3339();
        let events_today: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM audit_events WHERE timestamp >= $1")
                .bind(&today)
                .fetch_one(&self.pool)
                .await?;
        let unique_users: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT user_id) FROM audit_events WHERE user_id IS NOT NULL",
        )
        .fetch_one(&self.pool)
        .await?;
        let type_rows: Vec<(String, i64)> = sqlx::query_as::<_, (String, i64)>(
            "SELECT event_type, COUNT(*) FROM audit_events GROUP BY event_type",
        )
        .fetch_all(&self.pool)
        .await?;
        let events_by_type: HashMap<String, i64> = type_rows.into_iter().collect();
        Ok(AuditStats {
            total_events: total,
            events_by_type,
            events_today,
            unique_users,
        })
    }

    async fn clear_audit_events(&self) -> Result<()> {
        sqlx::query("DELETE FROM audit_events")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_latest_audit_hash(&self) -> Result<Option<String>> {
        let hash: Option<Option<String>> =
            sqlx::query_scalar("SELECT hash FROM audit_events ORDER BY timestamp DESC LIMIT 1")
                .fetch_optional(&self.pool)
                .await?;
        Ok(hash.flatten())
    }
}

fn event_type_label(t: crate::audit::AuditEventType) -> String {
    use crate::audit::AuditEventType::*;
    match t {
        Login => "login",
        Logout => "logout",
        ApiKeyCreated => "api_key_created",
        ApiKeyRevoked => "api_key_revoked",
        TrafficExported => "traffic_exported",
        SessionCreated => "session_created",
        SessionDeleted => "session_deleted",
        MockCreated => "mock_created",
        MockDeleted => "mock_deleted",
        BreakpointCreated => "breakpoint_created",
        BreakpointDeleted => "breakpoint_deleted",
        ConfigChanged => "config_changed",
        Custom => "custom",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::{AuditEvent, AuditEventType};
    use crate::user::{UserRole, UserStatus};

    async fn make_store() -> PostgresEnterpriseStore {
        let url = std::env::var("MADHYAMAS_PG_TEST_URL").unwrap_or_else(|_| {
            "postgres://madhyamas:testpass@localhost:5432/madhyamas".to_string()
        });
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await
            .expect("failed to connect to PostgreSQL test instance");
        PostgresEnterpriseStore::new(pool)
            .await
            .expect("failed to create PostgresEnterpriseStore")
    }

    #[tokio::test]
    #[ignore]
    async fn test_pg_enterprise_user_crud() {
        let store = make_store().await;
        let user_id = uuid::Uuid::new_v4().to_string();
        let user = User::new(
            user_id.clone(),
            format!("testuser_{}", &user_id[..8]),
            Some("test@example.com".to_string()),
            UserRole::Admin,
            "Test User".to_string(),
            UserStatus::Active,
        );
        store
            .create_user(&user, "hashed_password_123")
            .await
            .unwrap();

        let fetched = store.get_user(&user_id).await.unwrap().unwrap();
        assert_eq!(fetched.username, user.username);
        assert_eq!(fetched.role, UserRole::Admin);

        let by_username = store
            .get_user_by_username(&user.username)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(by_username.id, user_id);

        store.delete_user(&user_id).await.unwrap();
        assert!(store.get_user(&user_id).await.unwrap().is_none());
    }

    #[tokio::test]
    #[ignore]
    async fn test_pg_enterprise_audit_log() {
        let store = make_store().await;
        let mut event = AuditEvent::new(AuditEventType::Login, "User logged in");
        event.hash = Some("abc123".to_string());
        store.log_audit_event(&event).await.unwrap();

        let filter = AuditFilter {
            event_type: Some(AuditEventType::Login),
            ..Default::default()
        };
        let events = store.query_audit_events(&filter).await.unwrap();
        assert!(!events.is_empty());

        let latest = store.get_latest_audit_hash().await.unwrap();
        assert_eq!(latest, Some("abc123".to_string()));
    }

    #[tokio::test]
    #[ignore]
    async fn test_pg_enterprise_api_key() {
        let store = make_store().await;
        let user_id = uuid::Uuid::new_v4().to_string();
        let user = User::new(
            user_id.clone(),
            format!("apikeyuser_{}", &user_id[..8]),
            None,
            UserRole::Viewer,
            "API Key User".to_string(),
            UserStatus::Active,
        );
        store.create_user(&user, "hash").await.unwrap();

        let key = ApiKeyRecord {
            id: uuid::Uuid::new_v4().to_string(),
            user_id: user_id.clone(),
            name: "test-key".to_string(),
            key_hash: format!("hash_{}", uuid::Uuid::new_v4()),
            key_prefix: "mk_test".to_string(),
            scopes: "[]".to_string(),
            expires_at: None,
            last_used_at: None,
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        store.create_api_key(&key).await.unwrap();

        let fetched = store
            .get_api_key_by_hash(&key.key_hash)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(fetched.name, "test-key");

        let keys = store.list_api_keys(&user_id).await.unwrap();
        assert!(!keys.is_empty());

        store.revoke_api_key(&key.id).await.unwrap();
        assert!(store
            .get_api_key_by_hash(&key.key_hash)
            .await
            .unwrap()
            .is_none());

        store.delete_user(&user_id).await.unwrap();
    }
}

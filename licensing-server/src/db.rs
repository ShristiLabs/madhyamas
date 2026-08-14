//! PostgreSQL database layer for the licensing server.
//!
//! Provides schema initialization (idempotent `CREATE TABLE IF NOT EXISTS`
//! DDL run on startup) and query helpers for accounts, customers, licenses,
//! seats, and the audit log. Uses `sqlx` with the PostgreSQL backend.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// Database error wrapper.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("license not found: {0}")]
    LicenseNotFound(String),
    #[error("seat limit reached: active={active}, limit={limit}")]
    SeatLimitReached { active: i64, limit: i64 },
    #[error("seat not found for instance: {0}")]
    SeatNotFound(String),
}

/// Row in the `accounts` table.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AccountRow {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub status: String,
}

/// Row in the `customers` table.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CustomerRow {
    pub id: Uuid,
    pub account_id: Uuid,
    pub company_name: String,
    pub contact_email: String,
    pub created_at: DateTime<Utc>,
}

/// Row in the `licenses` table.
///
/// `customer_id` is a TEXT reference string (e.g. "cust_test") rather than a
/// UUID FK — the API accepts arbitrary customer references and the customers
/// table (UUID-keyed) is populated by the customer portal (Phase 12b).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct LicenseRow {
    pub id: Uuid,
    pub customer_id: String,
    pub license_id: String,
    pub plan: String,
    pub seats: i32,
    pub instance_id: Option<String>,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub features: serde_json::Value,
    pub status: String,
    pub signature: String,
}

/// Row in the `seats` table.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SeatRow {
    pub id: Uuid,
    pub license_id: Uuid,
    pub instance_id: String,
    pub registered_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
    pub status: String,
}

/// Row in the `audit_log` table.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AuditLogRow {
    pub id: Uuid,
    pub event_type: String,
    pub account_id: Option<Uuid>,
    pub description: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

/// Run the full schema DDL (idempotent). Called on server startup.
pub async fn init_schema(pool: &PgPool) -> Result<(), DbError> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS accounts (
            id            UUID PRIMARY KEY,
            name          TEXT NOT NULL,
            email         TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            status        TEXT NOT NULL DEFAULT 'active'
        );
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS customers (
            id            UUID PRIMARY KEY,
            account_id    UUID NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
            company_name  TEXT NOT NULL,
            contact_email TEXT NOT NULL,
            created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS licenses (
            id          UUID PRIMARY KEY,
            customer_id TEXT NOT NULL,
            license_id  TEXT NOT NULL UNIQUE,
            plan        TEXT NOT NULL,
            seats       INTEGER NOT NULL DEFAULT 1,
            instance_id TEXT,
            issued_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            expires_at  TIMESTAMPTZ NOT NULL,
            features    JSONB NOT NULL DEFAULT '[]',
            status      TEXT NOT NULL DEFAULT 'active',
            signature   TEXT NOT NULL
        );
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS seats (
            id             UUID PRIMARY KEY,
            license_id     UUID NOT NULL REFERENCES licenses(id) ON DELETE CASCADE,
            instance_id    TEXT NOT NULL UNIQUE,
            registered_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            last_heartbeat TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            status         TEXT NOT NULL DEFAULT 'active'
        );
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS audit_log (
            id          UUID PRIMARY KEY,
            event_type  TEXT NOT NULL,
            account_id  UUID,
            description TEXT NOT NULL,
            timestamp   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            metadata    JSONB NOT NULL DEFAULT '{}'
        );
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_licenses_customer_id ON licenses(customer_id);")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_seats_license_id ON seats(license_id);")
        .execute(pool)
        .await?;

    Ok(())
}

/// Insert a license record.
pub async fn insert_license(pool: &PgPool, row: &LicenseRow) -> Result<(), DbError> {
    sqlx::query(
        r#"
        INSERT INTO licenses (id, customer_id, license_id, plan, seats, instance_id,
                              issued_at, expires_at, features, status, signature)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11);
        "#,
    )
    .bind(row.id)
    .bind(&row.customer_id)
    .bind(&row.license_id)
    .bind(&row.plan)
    .bind(row.seats)
    .bind(&row.instance_id)
    .bind(row.issued_at)
    .bind(row.expires_at)
    .bind(&row.features)
    .bind(&row.status)
    .bind(&row.signature)
    .execute(pool)
    .await?;
    Ok(())
}

/// Fetch a license by its string license_id (e.g. "lic_abc123").
pub async fn get_license_by_id(pool: &PgPool, license_id: &str) -> Result<LicenseRow, DbError> {
    sqlx::query_as::<_, LicenseRow>(
        r#"
        SELECT id, customer_id, license_id, plan, seats, instance_id,
               issued_at, expires_at, features, status, signature
        FROM licenses WHERE license_id = $1;
        "#,
    )
    .bind(license_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| DbError::LicenseNotFound(license_id.to_string()))
}

/// List licenses, optionally filtered by customer_id (string reference).
pub async fn list_licenses(
    pool: &PgPool,
    customer_id: Option<&str>,
) -> Result<Vec<LicenseRow>, DbError> {
    if let Some(cid) = customer_id {
        sqlx::query_as::<_, LicenseRow>(
            r#"
            SELECT id, customer_id, license_id, plan, seats, instance_id,
                   issued_at, expires_at, features, status, signature
            FROM licenses WHERE customer_id = $1 ORDER BY issued_at DESC;
            "#,
        )
        .bind(cid)
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    } else {
        sqlx::query_as::<_, LicenseRow>(
            r#"
            SELECT id, customer_id, license_id, plan, seats, instance_id,
                   issued_at, expires_at, features, status, signature
            FROM licenses ORDER BY issued_at DESC;
            "#,
        )
        .fetch_all(pool)
        .await
        .map_err(Into::into)
    }
}

/// Revoke a license (set status to 'revoked').
pub async fn revoke_license(pool: &PgPool, license_id: &str) -> Result<LicenseRow, DbError> {
    sqlx::query_as::<_, LicenseRow>(
        r#"
        UPDATE licenses SET status = 'revoked'
        WHERE license_id = $1 AND status != 'revoked'
        RETURNING id, customer_id, license_id, plan, seats, instance_id,
                  issued_at, expires_at, features, status, signature;
        "#,
    )
    .bind(license_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| DbError::LicenseNotFound(license_id.to_string()))
}

/// Count active seats for a license.
pub async fn count_active_seats(pool: &PgPool, license_db_id: Uuid) -> Result<i64, DbError> {
    let row: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM seats WHERE license_id = $1 AND status = 'active';")
            .bind(license_db_id)
            .fetch_one(pool)
            .await?;
    Ok(row.0)
}

/// Register a seat for a license. Enforces the seat limit. If the instance is
/// already registered and active, updates the heartbeat and returns success
/// without consuming a new seat.
pub async fn register_seat(
    pool: &PgPool,
    license_db_id: Uuid,
    instance_id: &str,
    seat_limit: i32,
) -> Result<i64, DbError> {
    // Check if already registered.
    let existing: Option<(Uuid, String)> =
        sqlx::query_as("SELECT id, status FROM seats WHERE instance_id = $1;")
            .bind(instance_id)
            .fetch_optional(pool)
            .await?;

    if let Some((seat_id, status)) = existing {
        if status == "active" {
            // Already active — refresh heartbeat.
            sqlx::query("UPDATE seats SET last_heartbeat = NOW() WHERE id = $1;")
                .bind(seat_id)
                .execute(pool)
                .await?;
            return count_active_seats(pool, license_db_id).await;
        }
        // Was inactive — reactivate.
        let active = count_active_seats(pool, license_db_id).await?;
        if active >= seat_limit as i64 {
            return Err(DbError::SeatLimitReached {
                active,
                limit: seat_limit as i64,
            });
        }
        sqlx::query("UPDATE seats SET status = 'active', last_heartbeat = NOW() WHERE id = $1;")
            .bind(seat_id)
            .execute(pool)
            .await?;
        return count_active_seats(pool, license_db_id).await;
    }

    // New registration — enforce seat limit.
    let active = count_active_seats(pool, license_db_id).await?;
    if active >= seat_limit as i64 {
        return Err(DbError::SeatLimitReached {
            active,
            limit: seat_limit as i64,
        });
    }
    sqlx::query(
        r#"
        INSERT INTO seats (id, license_id, instance_id, status)
        VALUES ($1, $2, $3, 'active');
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(license_db_id)
    .bind(instance_id)
    .execute(pool)
    .await?;
    count_active_seats(pool, license_db_id).await
}

/// Update the heartbeat timestamp for a seat.
pub async fn heartbeat_seat(pool: &PgPool, instance_id: &str) -> Result<(), DbError> {
    let result = sqlx::query(
        "UPDATE seats SET last_heartbeat = NOW() WHERE instance_id = $1 AND status = 'active';",
    )
    .bind(instance_id)
    .execute(pool)
    .await?;
    if result.rows_affected() == 0 {
        return Err(DbError::SeatNotFound(instance_id.to_string()));
    }
    Ok(())
}

/// Deregister a seat (set status to 'inactive').
pub async fn deregister_seat(pool: &PgPool, instance_id: &str) -> Result<(), DbError> {
    let result = sqlx::query("UPDATE seats SET status = 'inactive' WHERE instance_id = $1;")
        .bind(instance_id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(DbError::SeatNotFound(instance_id.to_string()));
    }
    Ok(())
}

/// List all seats for a license.
pub async fn list_seats(pool: &PgPool, license_db_id: Uuid) -> Result<Vec<SeatRow>, DbError> {
    sqlx::query_as::<_, SeatRow>(
        r#"
        SELECT id, license_id, instance_id, registered_at, last_heartbeat, status
        FROM seats WHERE license_id = $1 ORDER BY registered_at DESC;
        "#,
    )
    .bind(license_db_id)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

/// Insert an audit log entry.
pub async fn insert_audit(
    pool: &PgPool,
    event_type: &str,
    account_id: Option<Uuid>,
    description: &str,
    metadata: serde_json::Value,
) -> Result<(), DbError> {
    sqlx::query(
        r#"
        INSERT INTO audit_log (id, event_type, account_id, description, metadata)
        VALUES ($1, $2, $3, $4, $5);
        "#,
    )
    .bind(Uuid::new_v4())
    .bind(event_type)
    .bind(account_id)
    .bind(description)
    .bind(metadata)
    .execute(pool)
    .await?;
    Ok(())
}

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

/// Row in the `team_members` table.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TeamMemberRow {
    pub id: Uuid,
    pub customer_id: Uuid,
    pub email: String,
    pub role: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

/// Row in the `admins` table.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AdminRow {
    pub id: Uuid,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
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

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS team_members (
            id          UUID PRIMARY KEY,
            customer_id UUID NOT NULL REFERENCES customers(id) ON DELETE CASCADE,
            email       TEXT NOT NULL,
            role        TEXT NOT NULL DEFAULT 'developer',
            status      TEXT NOT NULL DEFAULT 'invited',
            created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS admins (
            id            UUID PRIMARY KEY,
            email         TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            role          TEXT NOT NULL DEFAULT 'admin',
            created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
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
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_team_members_customer_id ON team_members(customer_id);",
    )
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

// ---------------------------------------------------------------------------
// Account / customer helpers (Phase 12b)
// ---------------------------------------------------------------------------

/// Insert an account record.
pub async fn insert_account(
    pool: &PgPool,
    id: Uuid,
    name: &str,
    email: &str,
    password_hash: &str,
) -> Result<(), DbError> {
    sqlx::query(
        r#"
        INSERT INTO accounts (id, name, email, password_hash, status)
        VALUES ($1, $2, $3, $4, 'active');
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(email)
    .bind(password_hash)
    .execute(pool)
    .await?;
    Ok(())
}

/// Look up an account by email. Returns the full row (including password hash).
pub async fn get_account_by_email(pool: &PgPool, email: &str) -> Result<AccountRow, DbError> {
    sqlx::query_as::<_, AccountRow>(
        r#"
        SELECT id, name, email, password_hash, created_at, status
        FROM accounts WHERE email = $1;
        "#,
    )
    .bind(email)
    .fetch_optional(pool)
    .await?
    .ok_or(DbError::LicenseNotFound(email.to_string()))
}

/// Get an account by ID.
pub async fn get_account_by_id(pool: &PgPool, id: Uuid) -> Result<AccountRow, DbError> {
    sqlx::query_as::<_, AccountRow>(
        r#"
        SELECT id, name, email, password_hash, created_at, status
        FROM accounts WHERE id = $1;
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or(DbError::LicenseNotFound(id.to_string()))
}

/// Insert a customer record linked to an account.
pub async fn insert_customer(
    pool: &PgPool,
    id: Uuid,
    account_id: Uuid,
    company_name: &str,
    contact_email: &str,
) -> Result<(), DbError> {
    sqlx::query(
        r#"
        INSERT INTO customers (id, account_id, company_name, contact_email)
        VALUES ($1, $2, $3, $4);
        "#,
    )
    .bind(id)
    .bind(account_id)
    .bind(company_name)
    .bind(contact_email)
    .execute(pool)
    .await?;
    Ok(())
}

/// Get the customer record for an account.
pub async fn get_customer_by_account(
    pool: &PgPool,
    account_id: Uuid,
) -> Result<CustomerRow, DbError> {
    sqlx::query_as::<_, CustomerRow>(
        r#"
        SELECT id, account_id, company_name, contact_email, created_at
        FROM customers WHERE account_id = $1;
        "#,
    )
    .bind(account_id)
    .fetch_optional(pool)
    .await?
    .ok_or(DbError::LicenseNotFound(account_id.to_string()))
}

/// List all customers with pagination.
pub async fn list_all_customers(
    pool: &PgPool,
    limit: i64,
    offset: i64,
) -> Result<Vec<CustomerRow>, DbError> {
    sqlx::query_as::<_, CustomerRow>(
        r#"
        SELECT id, account_id, company_name, contact_email, created_at
        FROM customers ORDER BY created_at DESC LIMIT $1 OFFSET $2;
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

/// Count total customers.
pub async fn count_customers(pool: &PgPool) -> Result<i64, DbError> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM customers;")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

/// Set an account's status (e.g. 'active' or 'suspended').
pub async fn set_account_status(
    pool: &PgPool,
    account_id: Uuid,
    status: &str,
) -> Result<(), DbError> {
    let result = sqlx::query("UPDATE accounts SET status = $1 WHERE id = $2;")
        .bind(status)
        .bind(account_id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(DbError::LicenseNotFound(account_id.to_string()));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Team member helpers (Phase 12b.4)
// ---------------------------------------------------------------------------

/// Insert a team member invitation.
pub async fn insert_team_member(
    pool: &PgPool,
    id: Uuid,
    customer_id: Uuid,
    email: &str,
    role: &str,
) -> Result<(), DbError> {
    sqlx::query(
        r#"
        INSERT INTO team_members (id, customer_id, email, role, status)
        VALUES ($1, $2, $3, $4, 'invited');
        "#,
    )
    .bind(id)
    .bind(customer_id)
    .bind(email)
    .bind(role)
    .execute(pool)
    .await?;
    Ok(())
}

/// List team members for a customer.
pub async fn list_team_members(
    pool: &PgPool,
    customer_id: Uuid,
) -> Result<Vec<TeamMemberRow>, DbError> {
    sqlx::query_as::<_, TeamMemberRow>(
        r#"
        SELECT id, customer_id, email, role, status, created_at
        FROM team_members WHERE customer_id = $1 ORDER BY created_at DESC;
        "#,
    )
    .bind(customer_id)
    .fetch_all(pool)
    .await
    .map_err(Into::into)
}

/// Remove a team member. Returns `NotFound` if the member does not exist or
/// does not belong to the given customer.
pub async fn delete_team_member(
    pool: &PgPool,
    customer_id: Uuid,
    member_id: Uuid,
) -> Result<(), DbError> {
    let result = sqlx::query("DELETE FROM team_members WHERE id = $1 AND customer_id = $2;")
        .bind(member_id)
        .bind(customer_id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(DbError::LicenseNotFound(member_id.to_string()));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Admin helpers (Phase 12d.1)
// ---------------------------------------------------------------------------

/// Insert an admin record.
pub async fn insert_admin(
    pool: &PgPool,
    id: Uuid,
    email: &str,
    password_hash: &str,
    role: &str,
) -> Result<(), DbError> {
    sqlx::query(
        r#"
        INSERT INTO admins (id, email, password_hash, role)
        VALUES ($1, $2, $3, $4);
        "#,
    )
    .bind(id)
    .bind(email)
    .bind(password_hash)
    .bind(role)
    .execute(pool)
    .await?;
    Ok(())
}

/// Look up an admin by email.
pub async fn get_admin_by_email(pool: &PgPool, email: &str) -> Result<AdminRow, DbError> {
    sqlx::query_as::<_, AdminRow>(
        r#"
        SELECT id, email, password_hash, role, created_at
        FROM admins WHERE email = $1;
        "#,
    )
    .bind(email)
    .fetch_optional(pool)
    .await?
    .ok_or(DbError::LicenseNotFound(email.to_string()))
}

/// Count total admins (used to detect first-run bootstrap).
pub async fn count_admins(pool: &PgPool) -> Result<i64, DbError> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM admins;")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

// ---------------------------------------------------------------------------
// License status helpers (Phase 12c + 12d)
// ---------------------------------------------------------------------------

/// Update a license's status by its string license_id.
pub async fn set_license_status(
    pool: &PgPool,
    license_id: &str,
    status: &str,
) -> Result<LicenseRow, DbError> {
    sqlx::query_as::<_, LicenseRow>(
        r#"
        UPDATE licenses SET status = $2
        WHERE license_id = $1
        RETURNING id, customer_id, license_id, plan, seats, instance_id,
                  issued_at, expires_at, features, status, signature;
        "#,
    )
    .bind(license_id)
    .bind(status)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| DbError::LicenseNotFound(license_id.to_string()))
}

/// Extend a license's expiry. Returns the updated row.
pub async fn extend_license(
    pool: &PgPool,
    license_id: &str,
    new_expires_at: DateTime<Utc>,
) -> Result<LicenseRow, DbError> {
    sqlx::query_as::<_, LicenseRow>(
        r#"
        UPDATE licenses SET expires_at = $2
        WHERE license_id = $1
        RETURNING id, customer_id, license_id, plan, seats, instance_id,
                  issued_at, expires_at, features, status, signature;
        "#,
    )
    .bind(license_id)
    .bind(new_expires_at)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| DbError::LicenseNotFound(license_id.to_string()))
}

/// Count active licenses.
pub async fn count_active_licenses(pool: &PgPool) -> Result<i64, DbError> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM licenses WHERE status = 'active';")
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

/// Sum the seat counts across all active licenses.
pub async fn sum_active_seats(pool: &PgPool) -> Result<i64, DbError> {
    let row: (i64,) =
        sqlx::query_as("SELECT COALESCE(SUM(seats), 0) FROM licenses WHERE status = 'active';")
            .fetch_one(pool)
            .await?;
    Ok(row.0)
}

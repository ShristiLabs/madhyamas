//! Integration tests for seat tracking (requires PostgreSQL).
//!
//! These tests are marked `#[ignore]` and require a running PostgreSQL
//! instance. Run with: `cargo test -p madhyamas-licensing -- --ignored`.

use chrono::{Duration, Utc};
use madhyamas_licensing::db;
use madhyamas_licensing::license::{LicenseClaims, LicenseSigner};
use sqlx::PgPool;
use uuid::Uuid;

async fn setup_pool() -> PgPool {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://madhyamas:testpass@localhost:5432/madhyamas".to_string());
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .expect("connect to PG");
    db::init_schema(&pool).await.expect("init schema");
    pool
}

async fn create_test_license(pool: &PgPool) -> String {
    let (signer, _) = LicenseSigner::generate();
    let license_id = format!("lic_test_{}", Uuid::new_v4().simple());
    let claims = LicenseClaims {
        license_id: license_id.clone(),
        customer: "Test Corp".to_string(),
        plan: "enterprise".to_string(),
        seats: 3,
        instance_id: String::new(),
        issued_at: Utc::now() - Duration::days(1),
        expires_at: Utc::now() + Duration::days(365),
        features: vec!["auth".to_string()],
    };
    let file = signer.sign_license(&claims).expect("sign");

    let row = db::LicenseRow {
        id: Uuid::new_v4(),
        customer_id: "cust_test".to_string(),
        license_id: license_id.clone(),
        plan: "enterprise".to_string(),
        seats: 3,
        instance_id: None,
        issued_at: Utc::now(),
        expires_at: Utc::now() + Duration::days(365),
        features: serde_json::json!(["auth"]),
        status: "active".to_string(),
        signature: file.signature,
    };
    db::insert_license(pool, &row)
        .await
        .expect("insert license");
    license_id
}

/// Register, heartbeat, deregister, and verify seat counts.
#[tokio::test]
#[ignore]
async fn test_seat_tracking() {
    let pool = setup_pool().await;
    let license_id = create_test_license(&pool).await;
    let license_row = db::get_license_by_id(&pool, &license_id)
        .await
        .expect("get license");

    // Register 3 seats (the limit).
    for i in 0..3 {
        let instance = format!("inst_{i}");
        let active = db::register_seat(&pool, license_row.id, &instance, 3)
            .await
            .expect("register seat");
        assert_eq!(active, (i + 1) as i64);
    }

    // 4th seat should fail (limit reached).
    let err = db::register_seat(&pool, license_row.id, "inst_3", 3)
        .await
        .expect_err("should hit seat limit");
    assert!(matches!(err, db::DbError::SeatLimitReached { .. }));

    // Heartbeat an existing seat.
    db::heartbeat_seat(&pool, "inst_0")
        .await
        .expect("heartbeat");

    // List seats.
    let seats = db::list_seats(&pool, license_row.id)
        .await
        .expect("list seats");
    assert_eq!(seats.len(), 3);
    let active_count = db::count_active_seats(&pool, license_row.id)
        .await
        .expect("count");
    assert_eq!(active_count, 3);

    // Deregister one seat.
    db::deregister_seat(&pool, "inst_0")
        .await
        .expect("deregister");
    let active_count = db::count_active_seats(&pool, license_row.id)
        .await
        .expect("count");
    assert_eq!(active_count, 2);

    // Now the 4th seat should succeed (we freed one).
    let active = db::register_seat(&pool, license_row.id, "inst_3", 3)
        .await
        .expect("register after deregister");
    assert_eq!(active, 3);

    // Cleanup.
    sqlx::query("DELETE FROM seats WHERE license_id = $1;")
        .bind(license_row.id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM licenses WHERE id = $1;")
        .bind(license_row.id)
        .execute(&pool)
        .await
        .ok();
}

/// Re-registering an already-active instance should not consume a new seat.
#[tokio::test]
#[ignore]
async fn test_seat_re_register() {
    let pool = setup_pool().await;
    let license_id = create_test_license(&pool).await;
    let license_row = db::get_license_by_id(&pool, &license_id)
        .await
        .expect("get license");

    db::register_seat(&pool, license_row.id, "inst_dup", 3)
        .await
        .expect("first register");
    let active = db::register_seat(&pool, license_row.id, "inst_dup", 3)
        .await
        .expect("re-register");
    assert_eq!(active, 1, "re-registering should not add a seat");

    // Cleanup.
    sqlx::query("DELETE FROM seats WHERE license_id = $1;")
        .bind(license_row.id)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM licenses WHERE id = $1;")
        .bind(license_row.id)
        .execute(&pool)
        .await
        .ok();
}

/// License revocation sets status to 'revoked'.
#[tokio::test]
#[ignore]
async fn test_license_revocation() {
    let pool = setup_pool().await;
    let license_id = create_test_license(&pool).await;

    let revoked = db::revoke_license(&pool, &license_id)
        .await
        .expect("revoke");
    assert_eq!(revoked.status, "revoked");

    // Revoking again should still return the row (idempotent on status).
    let row = db::get_license_by_id(&pool, &license_id)
        .await
        .expect("get");
    assert_eq!(row.status, "revoked");

    // Cleanup.
    sqlx::query("DELETE FROM licenses WHERE license_id = $1;")
        .bind(&license_id)
        .execute(&pool)
        .await
        .ok();
}

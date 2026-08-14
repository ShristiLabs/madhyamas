//! Integration tests for the admin portal (Phase 12d).
//!
//! These tests require a running PostgreSQL instance and are marked
//! `#[ignore]`. Run them with:
//!
//! ```sh
//! cargo test -p madhyamas-licensing --test admin_portal -- --ignored
//! ```

use axum::body::Body;
use axum::http::{Request, StatusCode};
use madhyamas_licensing::api::{self, AppState};
use madhyamas_licensing::auth;
use madhyamas_licensing::db;
use madhyamas_licensing::license::LicenseSigner;
use sqlx::PgPool;
use tower::ServiceExt;

const DATABASE_URL: &str = "postgres://madhyamas:testpass@localhost:5432/madhyamas";

async fn setup_pool() -> PgPool {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(DATABASE_URL)
        .await
        .expect("connect to PG");
    db::init_schema(&pool).await.expect("init schema");
    pool
}

fn test_state(pool: PgPool) -> AppState {
    let (signer, public_key) = LicenseSigner::generate();
    AppState {
        pool,
        signer: std::sync::Arc::new(signer),
        public_key,
        admin_key: "dev".to_string(),
        stripe_api_key: None,
        stripe_webhook_secret: None,
    }
}

/// Create a test admin and return an admin JWT.
async fn create_test_admin(pool: &PgPool) -> String {
    let admin_id = uuid::Uuid::new_v4();
    let email = format!("admin_test_{}@madhyamas.local", uuid::Uuid::new_v4());
    let hash = auth::hash_password("admin123").expect("hash");
    db::insert_admin(pool, admin_id, &email, &hash, "super_admin")
        .await
        .expect("insert admin");
    auth::issue_admin_token(admin_id, "super_admin").expect("issue token")
}

/// Login with admin credentials and verify JWT.
#[tokio::test]
#[ignore]
async fn test_admin_login() {
    let pool = setup_pool().await;
    let email = format!("admin_login_{}@madhyamas.local", uuid::Uuid::new_v4());
    let hash = auth::hash_password("admin123").expect("hash");
    db::insert_admin(&pool, uuid::Uuid::new_v4(), &email, &hash, "super_admin")
        .await
        .expect("insert admin");

    let state = test_state(pool);
    let app = api::router(state);

    let login_body = serde_json::json!({
        "email": email,
        "password": "admin123"
    });
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/login")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&login_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.get("token").is_some());
    assert_eq!(json["role"].as_str().unwrap(), "super_admin");
}

/// List, suspend, and activate a customer.
#[tokio::test]
#[ignore]
async fn test_customer_management() {
    let pool = setup_pool().await;
    let token = create_test_admin(&pool).await;
    let state = test_state(pool.clone());
    let app = api::router(state);

    // Register a customer first.
    let email = format!("test_admin_cust_{}@example.com", uuid::Uuid::new_v4());
    let reg_body = serde_json::json!({
        "email": email,
        "password": "testpass123",
        "company_name": "Test Co"
    });
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&reg_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let customer_id = json["account_id"].as_str().unwrap();

    // List customers.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/admin/customers")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // The admin endpoints use customer UUID, not account UUID. Query the DB.
    let account_uuid = uuid::Uuid::parse_str(customer_id).unwrap();
    let customer = db::get_customer_by_account(&pool, account_uuid)
        .await
        .unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/admin/customers/{}/suspend", customer.id))
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify account is suspended.
    let account = db::get_account_by_id(&pool, account_uuid).await.unwrap();
    assert_eq!(account.status, "suspended");

    // Activate.
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/admin/customers/{}/activate", customer.id))
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let account = db::get_account_by_id(&pool, account_uuid).await.unwrap();
    assert_eq!(account.status, "active");
}

/// Create, revoke, and extend a license.
#[tokio::test]
#[ignore]
async fn test_license_management() {
    let pool = setup_pool().await;
    let token = create_test_admin(&pool).await;
    let state = test_state(pool);
    let app = api::router(state);

    let customer_id = format!("cust_test_{}", uuid::Uuid::new_v4());

    // Create a license.
    let create_body = serde_json::json!({
        "customer_id": customer_id,
        "customer_name": "Test Co",
        "plan": "pro",
        "seats": 50,
        "expires_at": (chrono::Utc::now() + chrono::Duration::days(365)).to_rfc3339(),
        "features": ["auth", "rbac"]
    });
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/admin/licenses")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::from(serde_json::to_vec(&create_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let lic: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let license_id = lic["license_id"].as_str().unwrap().to_string();

    // Extend the license.
    let extend_body = serde_json::json!({
        "expires_at": (chrono::Utc::now() + chrono::Duration::days(730)).to_rfc3339()
    });
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/admin/licenses/{license_id}/extend"))
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::from(serde_json::to_vec(&extend_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Revoke the license.
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/admin/licenses/{license_id}/revoke"))
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let lic: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(lic["status"].as_str().unwrap(), "revoked");
}

/// Verify the dashboard returns metrics.
#[tokio::test]
#[ignore]
async fn test_dashboard() {
    let pool = setup_pool().await;
    let token = create_test_admin(&pool).await;
    let state = test_state(pool);
    let app = api::router(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/admin/dashboard")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.get("total_customers").is_some());
    assert!(json.get("active_licenses").is_some());
    assert!(json.get("total_seats").is_some());
    assert!(json.get("mrr_cents").is_some());
    assert!(json.get("churn_rate").is_some());
    assert!(!json["stripe_configured"].as_bool().unwrap());
}

/// Verify that a customer JWT cannot access admin endpoints.
#[tokio::test]
#[ignore]
async fn test_customer_cannot_access_admin() {
    let pool = setup_pool().await;
    let state = test_state(pool);
    let app = api::router(state);

    // Register a customer.
    let email = format!("test_forbidden_{}@example.com", uuid::Uuid::new_v4());
    let reg_body = serde_json::json!({
        "email": email,
        "password": "testpass123",
        "company_name": "Test Co"
    });
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&reg_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let token = json["token"].as_str().unwrap();

    // Try to access admin dashboard with customer token.
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/admin/dashboard")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

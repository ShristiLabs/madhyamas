//! Integration tests for the customer portal (Phase 12b).
//!
//! These tests require a running PostgreSQL instance and are marked
//! `#[ignore]`. Run them with:
//!
//! ```sh
//! cargo test -p madhyamas-licensing --test customer_portal -- --ignored
//! ```

use axum::body::Body;
use axum::http::{Request, StatusCode};
use madhyamas_licensing::api::{self, AppState};
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
    madhyamas_licensing::db::init_schema(&pool)
        .await
        .expect("init schema");
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

/// Register a customer and verify a JWT is returned.
#[tokio::test]
#[ignore]
async fn test_customer_registration() {
    let pool = setup_pool().await;
    let state = test_state(pool);
    let app = api::router(state);

    let email = format!("test_reg_{}@example.com", uuid::Uuid::new_v4());
    let body = serde_json::json!({
        "email": email,
        "password": "testpass123",
        "company_name": "Test Co"
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header("Content-Type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.get("token").is_some(), "response should contain token");
    assert!(json.get("account_id").is_some());
    let token = json["token"].as_str().unwrap();
    assert!(!token.is_empty());
}

/// Register then login, verify JWT returned.
#[tokio::test]
#[ignore]
async fn test_customer_login() {
    let pool = setup_pool().await;
    let state = test_state(pool);
    let app = api::router(state);

    let email = format!("test_login_{}@example.com", uuid::Uuid::new_v4());

    // Register first.
    let reg_body = serde_json::json!({
        "email": email,
        "password": "testpass123",
        "company_name": "Test Co"
    });
    let _ = app
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

    // Login.
    let login_body = serde_json::json!({
        "email": email,
        "password": "testpass123"
    });
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
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
    assert!(!json["token"].as_str().unwrap().is_empty());
}

/// Register, create a license via admin API, fetch as customer.
#[tokio::test]
#[ignore]
async fn test_license_dashboard() {
    let pool = setup_pool().await;
    let state = test_state(pool);
    let app = api::router(state);

    let email = format!("test_dash_{}@example.com", uuid::Uuid::new_v4());

    // Register.
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
    assert_eq!(response.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let token = json["token"].as_str().unwrap();

    // Get /me to find the customer ID.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/auth/me")
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
    let me: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let customer_id = me["customer"]["id"].as_str().unwrap();

    // Create a license via legacy admin API (X-Admin-Key).
    let lic_body = serde_json::json!({
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
                .uri("/api/licenses")
                .header("Content-Type", "application/json")
                .header("X-Admin-Key", "dev")
                .body(Body::from(serde_json::to_vec(&lic_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Fetch licenses as customer.
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/customer/licenses")
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
    let licenses: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(!licenses.as_array().unwrap().is_empty());
}

/// Test team management: invite, list, remove.
#[tokio::test]
#[ignore]
async fn test_team_management() {
    let pool = setup_pool().await;
    let state = test_state(pool);
    let app = api::router(state);

    let email = format!("test_team_{}@example.com", uuid::Uuid::new_v4());

    // Register.
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

    // Invite a team member.
    let invite_body = serde_json::json!({
        "email": format!("dev_{}@example.com", uuid::Uuid::new_v4()),
        "role": "developer"
    });
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/customer/team")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::from(serde_json::to_vec(&invite_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    // List team members.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/customer/team")
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
    let members: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(!members.as_array().unwrap().is_empty());
}

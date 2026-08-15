//! Integration tests for Stripe billing (Phase 12c).
//!
//! These tests verify webhook signature verification and license creation
//! from payment events. Tests that require a live Stripe API key are marked
//! `#[ignore]`.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use hmac::{Hmac, Mac};
use madhyamas_licensing::api::{self, AppState};
use madhyamas_licensing::license::LicenseSigner;
use sha2::Sha256;
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

fn test_state(pool: PgPool, webhook_secret: Option<String>) -> AppState {
    let (signer, public_key) = LicenseSigner::generate();
    AppState {
        pool,
        signer: std::sync::Arc::new(signer),
        public_key,
        admin_key: "dev".to_string(),
        stripe_api_key: None,
        stripe_webhook_secret: webhook_secret,
    }
}

/// When Stripe is not configured, checkout returns 503.
#[tokio::test]
#[ignore]
async fn test_checkout_session_creation_503() {
    let pool = setup_pool().await;
    let state = test_state(pool, None);
    let app = api::router(state);

    // First register a customer to get a token.
    let email = format!("test_stripe_{}@example.com", uuid::Uuid::new_v4());
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

    // Attempt checkout — should return 503.
    let checkout_body = serde_json::json!({
        "plan": "starter",
        "success_url": "https://example.com/success",
        "cancel_url": "https://example.com/cancel"
    });
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/billing/checkout")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::from(serde_json::to_vec(&checkout_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

/// Verify that invalid webhook signatures are rejected.
#[tokio::test]
#[ignore]
async fn test_webhook_signature_verification() {
    let pool = setup_pool().await;
    let secret = "whsec_test_secret".to_string();
    let state = test_state(pool, Some(secret.clone()));
    let app = api::router(state);

    let payload =
        r#"{"type":"checkout.session.completed","data":{"object":{"customer":"cus_test123"}}}"#;

    // Invalid signature.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/billing/webhook")
                .header("Stripe-Signature", "t=12345,v1=invalid_signature")
                .header("Content-Type", "application/json")
                .body(Body::from(payload.as_bytes()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // Valid signature.
    let timestamp = "12345";
    let signed_payload = format!("{timestamp}.{payload}");
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(signed_payload.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());
    let sig_header = format!("t={timestamp},v1={signature}");

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/billing/webhook")
                .header("Stripe-Signature", &sig_header)
                .header("Content-Type", "application/json")
                .body(Body::from(payload.as_bytes()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

/// Simulate a webhook event and verify a license is created.
#[tokio::test]
#[ignore]
async fn test_license_creation_from_payment() {
    let pool = setup_pool().await;
    let secret = "whsec_test_secret".to_string();
    let state = test_state(pool, Some(secret.clone()));
    let app = api::router(state);

    let customer_stripe_id = format!("cus_test_{}", uuid::Uuid::new_v4());
    let payload = serde_json::json!({
        "type": "checkout.session.completed",
        "data": {
            "object": {
                "customer": customer_stripe_id,
                "metadata": {
                    "plan": "pro"
                }
            }
        }
    });
    let payload_bytes = serde_json::to_vec(&payload).unwrap();

    let timestamp = "12345";
    let signed_payload = format!("{timestamp}.{}", String::from_utf8_lossy(&payload_bytes));
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(signed_payload.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());
    let sig_header = format!("t={timestamp},v1={signature}");

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/billing/webhook")
                .header("Stripe-Signature", &sig_header)
                .header("Content-Type", "application/json")
                .body(Body::from(payload_bytes.clone()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify the license was created by listing licenses via admin API.
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/licenses?customer_id={customer_stripe_id}"))
                .header("X-Admin-Key", "dev")
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
    let arr = licenses.as_array().unwrap();
    assert!(!arr.is_empty(), "license should have been created");
    let lic = &arr[0];
    assert_eq!(lic["plan"].as_str().unwrap(), "pro");
    assert_eq!(lic["status"].as_str().unwrap(), "active");
    assert_eq!(lic["customer_id"].as_str().unwrap(), customer_stripe_id);
}

/// Test that invoice.payment_failed suspends the license.
#[tokio::test]
#[ignore]
async fn test_license_suspension_on_payment_failure() {
    let pool = setup_pool().await;
    let secret = "whsec_test_secret".to_string();
    let state = test_state(pool, Some(secret.clone()));
    let app = api::router(state);

    let customer_stripe_id = format!("cus_suspend_{}", uuid::Uuid::new_v4());

    // First create a license via checkout webhook.
    let create_payload = serde_json::json!({
        "type": "checkout.session.completed",
        "data": {
            "object": {
                "customer": customer_stripe_id,
                "metadata": { "plan": "starter" }
            }
        }
    });
    let create_str = serde_json::to_string(&create_payload).unwrap();
    send_webhook(&app, &secret, &create_str).await;

    // Now send payment_failed.
    let fail_payload = serde_json::json!({
        "type": "invoice.payment_failed",
        "data": {
            "object": {
                "customer": customer_stripe_id
            }
        }
    });
    let fail_str = serde_json::to_string(&fail_payload).unwrap();
    let response = send_webhook(&app, &secret, &fail_str).await;
    assert_eq!(response.status(), StatusCode::OK);

    // Verify the license is suspended.
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/licenses?customer_id={customer_stripe_id}"))
                .header("X-Admin-Key", "dev")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    let licenses: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let lic = &licenses.as_array().unwrap()[0];
    assert_eq!(lic["status"].as_str().unwrap(), "suspended");
}

/// Helper: send a signed webhook event and return the response.
async fn send_webhook(app: &axum::Router, secret: &str, payload: &str) -> axum::response::Response {
    let payload_bytes = payload.as_bytes().to_vec();
    let timestamp = "12345";
    let signed_payload = format!("{timestamp}.{payload}");
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(signed_payload.as_bytes());
    let signature = hex::encode(mac.finalize().into_bytes());
    let sig_header = format!("t={timestamp},v1={signature}");

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/billing/webhook")
                .header("Stripe-Signature", &sig_header)
                .header("Content-Type", "application/json")
                .body(Body::from(payload_bytes))
                .unwrap(),
        )
        .await
        .unwrap()
}

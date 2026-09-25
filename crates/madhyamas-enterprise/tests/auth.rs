//! Integration tests for the public auth API: JWT generate/validate/refresh
//! flows, API-key lifecycle against a store, scope matching, and proxy
//! credential principal resolution (issue #103).

use madhyamas_core::{ProxyAuthValidator, ProxyCredentials, ProxyPrincipal};
use madhyamas_enterprise::auth::hash_api_key;
use madhyamas_enterprise::store::ApiKeyRecord;
use madhyamas_enterprise::{ApiKey, AuthConfig, AuthManager, Scope};
use madhyamas_test_utils::enterprise::{seed_user, test_manager, test_store};

#[test]
fn test_jwt_generate_and_validate() {
    let mgr = test_manager();
    let token = mgr.generate_jwt("user-1", "admin").expect("generate");
    let claims = mgr.validate_jwt(&token).expect("validate");
    assert_eq!(claims.sub, "user-1");
    assert_eq!(claims.role, "admin");
    assert_eq!(claims.typ, "access");
}

#[test]
fn test_jwt_wrong_secret_rejected() {
    let mgr = test_manager();
    let token = mgr.generate_jwt("user-1", "admin").expect("generate");
    let other = AuthManager::new(AuthConfig {
        enabled: true,
        jwt_secret: "a-completely-different-secret".to_string(),
        ..AuthConfig::default()
    });
    assert!(other.validate_jwt(&token).is_err());
}

#[test]
fn test_refresh_token_flow() {
    let mgr = test_manager();
    let (access, refresh, sid, _exp) = mgr
        .generate_token_pair("user-1", "admin")
        .expect("generate pair");
    // Access token validates.
    let access_claims = mgr.validate_jwt(&access).expect("validate access");
    assert_eq!(access_claims.typ, "access");
    // Refresh token validates and shares the session ID.
    let refresh_claims = mgr
        .validate_refresh_token(&refresh)
        .expect("validate refresh");
    assert_eq!(refresh_claims.typ, "refresh");
    assert_eq!(refresh_claims.sub, "user-1");
    assert_eq!(refresh_claims.sid, Some(sid.clone()));
    // Access token is rejected by validate_refresh_token (wrong typ).
    assert!(mgr.validate_refresh_token(&access).is_err());
    // Refresh token is rejected by validate_jwt (wrong typ).
    assert!(mgr.validate_jwt(&refresh).is_err());
}

// ---- Phase 4c: API key scopes + store-backed validation ----

#[tokio::test]
async fn test_api_key_create_and_validate() {
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let mgr = test_manager().with_store(store.clone());

    let api_key = ApiKey::generate(&uid, "test-key");
    let hash = hash_api_key(&api_key.key);
    let record = ApiKeyRecord {
        id: api_key.id.clone(),
        user_id: uid.clone(),
        name: api_key.name.clone(),
        key_hash: hash,
        key_prefix: api_key.key.chars().take(12).collect(),
        scopes: serde_json::to_string(&["traffic:read"]).unwrap(),
        expires_at: None,
        last_used_at: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    store.create_api_key(&record).await.expect("persist key");

    let auth = mgr.validate_api_key(&api_key.key).await.expect("validate");
    assert_eq!(auth.user_id, uid);
    assert_eq!(auth.scopes, vec!["traffic:read"]);
    assert_eq!(auth.key_id, api_key.id);
}

#[tokio::test]
async fn test_api_key_expired() {
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let mgr = test_manager().with_store(store.clone());

    let api_key = ApiKey::generate(&uid, "expired-key");
    let hash = hash_api_key(&api_key.key);
    let past = (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
    let record = ApiKeyRecord {
        id: api_key.id.clone(),
        user_id: uid.clone(),
        name: api_key.name.clone(),
        key_hash: hash,
        key_prefix: api_key.key.chars().take(12).collect(),
        scopes: "[]".to_string(),
        expires_at: Some(past),
        last_used_at: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    store.create_api_key(&record).await.expect("persist key");

    let result = mgr.validate_api_key(&api_key.key).await;
    assert!(result.is_err(), "expired key should be rejected");
}

#[tokio::test]
async fn test_api_key_revoked() {
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let mgr = test_manager().with_store(store.clone());

    let api_key = ApiKey::generate(&uid, "temp-key");
    let hash = hash_api_key(&api_key.key);
    let record = ApiKeyRecord {
        id: api_key.id.clone(),
        user_id: uid.clone(),
        name: api_key.name.clone(),
        key_hash: hash,
        key_prefix: api_key.key.chars().take(12).collect(),
        scopes: "[]".to_string(),
        expires_at: None,
        last_used_at: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    store.create_api_key(&record).await.expect("persist key");
    store.revoke_api_key(&api_key.id).await.expect("revoke");

    let result = mgr.validate_api_key(&api_key.key).await;
    assert!(result.is_err(), "revoked key should be rejected");
}

#[test]
fn test_scope_matching() {
    let traffic_read = Scope::parse("traffic:read");
    let traffic_write = Scope::parse("traffic:write");
    let wildcard = Scope::parse("*:*");
    let star = Scope::parse("*");

    assert!(traffic_read.is_valid());
    assert!(wildcard.is_valid());
    assert!(star.is_valid());
    assert_eq!(star, wildcard);

    assert!(Scope::matches(&traffic_read, &traffic_read));
    assert!(!Scope::matches(&traffic_read, &traffic_write));
    assert!(Scope::matches(&traffic_read, &wildcard));
    assert!(Scope::matches(&traffic_write, &wildcard));
    assert!(Scope::matches(&traffic_read, &Scope::parse("traffic:*")));
    assert!(Scope::matches(&traffic_read, &Scope::parse("*:read")));
    assert!(!Scope::matches(&traffic_read, &Scope::parse("mocks:read")));
}

// ---- Issue #103: ProxyAuthValidator resolves a principal ----

/// A valid API key supplied as proxy credentials resolves to a principal
/// carrying both the owning user id and the key record id (instead of the
/// identity being discarded after validation).
#[tokio::test]
async fn test_proxy_validator_api_key_resolves_principal() {
    let store = test_store().await;
    let uid = seed_user(&store).await;
    let mgr = test_manager().with_store(store.clone());

    let api_key = ApiKey::generate(&uid, "proxy-key");
    let record = ApiKeyRecord {
        id: api_key.id.clone(),
        user_id: uid.clone(),
        name: api_key.name.clone(),
        key_hash: hash_api_key(&api_key.key),
        key_prefix: api_key.key.chars().take(12).collect(),
        scopes: serde_json::to_string(&["traffic:read"]).unwrap(),
        expires_at: None,
        last_used_at: None,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    store.create_api_key(&record).await.expect("persist key");

    let principal = mgr
        .validate(&ProxyCredentials::ApiKey(api_key.key.clone()))
        .await
        .expect("api-key credentials validate");
    assert_eq!(principal.user_id.as_deref(), Some(uid.as_str()));
    assert_eq!(principal.api_key_id.as_deref(), Some(api_key.id.as_str()));
    assert!(principal.is_authenticated());
    // The key material itself must not leak into the principal.
    assert_ne!(principal.user_id.as_deref(), Some(api_key.key.as_str()));
}

/// A Bearer JWT supplied as proxy credentials resolves to the JWT subject
/// with no API-key id.
#[tokio::test]
async fn test_proxy_validator_bearer_resolves_user_principal() {
    let mgr = test_manager();
    let token = mgr.generate_jwt("user-7", "admin").expect("generate jwt");

    let principal = mgr
        .validate(&ProxyCredentials::ProxyBearer(token))
        .await
        .expect("bearer credentials validate");
    assert_eq!(principal.user_id.as_deref(), Some("user-7"));
    assert!(
        principal.api_key_id.is_none(),
        "JWT principals carry no api-key id"
    );
    assert!(principal.is_authenticated());
}

/// Unknown credentials are rejected with an error (the 407 path in the
/// engine), never a principal.
#[tokio::test]
async fn test_proxy_validator_rejects_unknown_api_key() {
    let store = test_store().await;
    seed_user(&store).await;
    let mgr = test_manager().with_store(store);

    let result = mgr
        .validate(&ProxyCredentials::ApiKey(
            "madhyamas_doesnotexist".to_string(),
        ))
        .await;
    assert!(result.is_err(), "unknown api key must be rejected");
}

/// Basic credentials currently have no password backend, so they must be
/// rejected rather than resolve a partial principal.
#[tokio::test]
async fn test_proxy_validator_basic_rejected_until_password_backend_lands() {
    let mgr = test_manager();
    let result = mgr
        .validate(&ProxyCredentials::ProxyBasicAuth(
            "alice:password123".to_string(),
        ))
        .await;
    assert!(
        result.is_err(),
        "basic auth stays rejected until the password backend exists"
    );
}

/// The OSS-tier contract: without a configured validator, connections stay
/// on the unauthenticated principal.
#[test]
fn test_unauthenticated_principal_is_the_oss_default() {
    let principal = ProxyPrincipal::unauthenticated();
    assert_eq!(principal, ProxyPrincipal::default());
    assert!(!principal.is_authenticated());
    assert!(principal.user_id.is_none() && principal.api_key_id.is_none());
}

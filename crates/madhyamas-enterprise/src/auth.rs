//! Authentication module
//!
//! Supports API keys and JWT tokens

use std::collections::HashMap;

use async_trait::async_trait;
use madhyamas_api::auth::{AuthError, AuthMethod, AuthProvider, Identity};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use super::enterprise_error::EnterpriseError;

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Enable authentication
    pub enabled: bool,
    /// JWT secret key
    pub jwt_secret: String,
    /// JWT expiration time in seconds
    pub jwt_expiration_secs: u64,
    /// API key header name
    pub api_key_header: String,
    /// Require authentication for all requests
    pub require_auth: bool,
    /// Token refresh interval in seconds
    pub refresh_interval_secs: u64,
    /// Refresh token lifetime in seconds (default 7 days).
    pub refresh_token_secs: u64,
    /// Session idle timeout in seconds (default 30 minutes). A session whose
    /// `last_activity` is older than this is considered expired and revoked.
    pub session_idle_timeout_secs: u64,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            jwt_secret: "madhyamas-secret-key-change-me".to_string(),
            jwt_expiration_secs: 3600, // 1 hour
            api_key_header: "X-API-Key".to_string(),
            require_auth: false,
            refresh_interval_secs: 300,        // 5 minutes
            refresh_token_secs: 7 * 24 * 3600, // 7 days
            session_idle_timeout_secs: 1800,   // 30 minutes
        }
    }
}

impl AuthConfig {
    /// Create development config
    pub fn development() -> Self {
        Self {
            enabled: false,
            ..Self::default()
        }
    }

    /// Create production config
    pub fn production(jwt_secret: String) -> Self {
        Self {
            enabled: true,
            jwt_secret,
            jwt_expiration_secs: 3600,
            api_key_header: "X-API-Key".to_string(),
            require_auth: true,
            refresh_interval_secs: 300,
            refresh_token_secs: 7 * 24 * 3600,
            session_idle_timeout_secs: 1800,
        }
    }
}

/// API Key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    /// Key ID
    pub id: String,
    /// User ID this key belongs to
    pub user_id: String,
    /// The actual key value
    pub key: String,
    /// Key name/description
    pub name: String,
    /// When the key was created
    pub created_at: i64,
    /// When the key expires (if ever)
    pub expires_at: Option<i64>,
    /// Is the key active
    pub is_active: bool,
    /// Last used timestamp
    pub last_used: Option<i64>,
}

impl ApiKey {
    /// Generate a new API key
    pub fn generate(user_id: &str, name: &str) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        let key = format!(
            "mad_{}",
            uuid::Uuid::new_v4().simple().to_string().replace('-', "")
        );
        let now = chrono::Utc::now().timestamp();

        Self {
            id,
            user_id: user_id.to_string(),
            key,
            name: name.to_string(),
            created_at: now,
            expires_at: None,
            is_active: true,
            last_used: None,
        }
    }

    /// Check if key is valid
    pub fn is_valid(&self) -> bool {
        if !self.is_active {
            return false;
        }

        if let Some(expires) = self.expires_at {
            if chrono::Utc::now().timestamp() > expires {
                return false;
            }
        }

        true
    }
}

/// JWT Claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    /// Subject (user ID)
    pub sub: String,
    /// Issuer
    pub iss: String,
    /// Audience
    pub aud: String,
    /// Expiration time
    pub exp: i64,
    /// Issued at
    pub iat: i64,
    /// User role
    pub role: String,
    /// Session ID
    pub sid: Option<String>,
    /// Token type: "access" or "refresh"
    pub typ: String,
}

impl JwtClaims {
    /// Create new claims for a user
    pub fn new(user_id: &str, role: &str, expiration_secs: i64) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            sub: user_id.to_string(),
            iss: "madhyamas".to_string(),
            aud: "madhyamas-api".to_string(),
            exp: now + expiration_secs,
            iat: now,
            role: role.to_string(),
            sid: Some(uuid::Uuid::new_v4().to_string()),
            typ: "access".to_string(),
        }
    }

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        chrono::Utc::now().timestamp() > self.exp
    }
}

/// Refresh token claims. Same shape as [`JwtClaims`] but with a longer
/// expiry and `typ = "refresh"`. The `sid` links the refresh token to a
/// persisted session so it can be revoked on logout.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshTokenClaims {
    /// Subject (user ID)
    pub sub: String,
    /// Issuer
    pub iss: String,
    /// Audience
    pub aud: String,
    /// Expiration time
    pub exp: i64,
    /// Issued at
    pub iat: i64,
    /// User role
    pub role: String,
    /// Session ID (links to the persisted auth session)
    pub sid: Option<String>,
    /// Token type: always "refresh"
    pub typ: String,
}

impl RefreshTokenClaims {
    /// Create new refresh token claims for a user.
    pub fn new(user_id: &str, role: &str, expiration_secs: i64, sid: &str) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            sub: user_id.to_string(),
            iss: "madhyamas".to_string(),
            aud: "madhyamas-api".to_string(),
            exp: now + expiration_secs,
            iat: now,
            role: role.to_string(),
            sid: Some(sid.to_string()),
            typ: "refresh".to_string(),
        }
    }
}

/// Authentication manager
#[derive(Debug)]
pub struct AuthManager {
    /// Configuration
    config: AuthConfig,
    /// API keys by key value
    api_keys: RwLock<HashMap<String, ApiKey>>,
    /// API keys by user ID
    user_keys: RwLock<HashMap<String, Vec<String>>>,
    /// Active sessions (session ID -> user ID)
    sessions: RwLock<HashMap<String, String>>,
}

impl AuthManager {
    /// Create a new authentication manager
    pub fn new(config: AuthConfig) -> Self {
        Self {
            config,
            api_keys: RwLock::new(HashMap::new()),
            user_keys: RwLock::new(HashMap::new()),
            sessions: RwLock::new(HashMap::new()),
        }
    }

    /// Check if authentication is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Check whether all requests must be authenticated. When false the auth
    /// middleware lets requests through even though the auth system is
    /// available (e.g. for bootstrap before any users exist).
    pub fn require_auth(&self) -> bool {
        self.config.require_auth
    }

    /// Validate an API key
    pub fn validate_api_key(&self, key: &str) -> Result<String, EnterpriseError> {
        let keys = self.api_keys.read();
        let api_key = keys.get(key).ok_or_else(|| EnterpriseError::AuthFailed {
            message: "Invalid API key".to_string(),
        })?;

        if !api_key.is_valid() {
            return Err(EnterpriseError::AuthFailed {
                message: "API key expired or inactive".to_string(),
            });
        }

        // Update last used
        let mut keys = self.api_keys.write();
        if let Some(k) = keys.get_mut(key) {
            k.last_used = Some(chrono::Utc::now().timestamp());
        }

        Ok(api_key.user_id.clone())
    }

    /// Create a new API key for a user
    pub fn create_api_key(&self, user_id: &str, name: &str) -> ApiKey {
        let api_key = ApiKey::generate(user_id, name);

        // Store by key value
        self.api_keys
            .write()
            .insert(api_key.key.clone(), api_key.clone());

        // Store by user ID
        self.user_keys
            .write()
            .entry(user_id.to_string())
            .or_default()
            .push(api_key.id.clone());

        api_key
    }

    /// Revoke an API key
    pub fn revoke_api_key(&self, key_id: &str) -> Result<(), EnterpriseError> {
        let mut keys = self.api_keys.write();
        let api_key = keys
            .remove(key_id)
            .ok_or_else(|| EnterpriseError::AuthFailed {
                message: "API key not found".to_string(),
            })?;

        // Remove from user's keys
        if let Some(user_keys) = self.user_keys.write().get_mut(&api_key.user_id) {
            user_keys.retain(|id| id != key_id);
        }

        Ok(())
    }

    /// Get all API keys for a user
    pub fn get_user_api_keys(&self, user_id: &str) -> Vec<ApiKey> {
        let user_keys = self.user_keys.read();
        let keys = self.api_keys.read();

        user_keys
            .get(user_id)
            .map(|ids| ids.iter().filter_map(|id| keys.get(id).cloned()).collect())
            .unwrap_or_default()
    }

    /// Generate a JWT access token for a user using HMAC-SHA256 signing.
    pub fn generate_jwt(&self, user_id: &str, role: &str) -> Result<String, EnterpriseError> {
        let claims = JwtClaims::new(user_id, role, self.config.jwt_expiration_secs as i64);
        let encoding_key = jsonwebtoken::EncodingKey::from_secret(self.config.jwt_secret.as_ref());
        jsonwebtoken::encode(&jsonwebtoken::Header::default(), &claims, &encoding_key).map_err(
            |e| EnterpriseError::JwtError {
                message: e.to_string(),
            },
        )
    }

    /// Validate a JWT access token and return its claims.
    ///
    /// The token is validated with an explicit `HS256` algorithm pin (prevents
    /// `none` algorithm and RS256/HS256 confusion attacks), a 60-second leeway
    /// for clock skew, and `exp` enforcement. The `typ` claim must be
    /// `access`; refresh tokens are rejected here (use
    /// [`validate_refresh_token`]).
    pub fn validate_jwt(&self, token: &str) -> Result<JwtClaims, EnterpriseError> {
        let decoding_key = jsonwebtoken::DecodingKey::from_secret(self.config.jwt_secret.as_ref());
        let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
        validation.leeway = 60;
        validation.validate_exp = true;
        validation.validate_aud = false;
        let token_data = jsonwebtoken::decode::<JwtClaims>(token, &decoding_key, &validation)
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("ExpiredSignature") {
                    EnterpriseError::TokenExpired
                } else {
                    EnterpriseError::AuthFailed {
                        message: format!("Invalid token: {}", msg),
                    }
                }
            })?;
        if token_data.claims.typ != "access" {
            return Err(EnterpriseError::AuthFailed {
                message: "Expected access token".to_string(),
            });
        }
        Ok(token_data.claims)
    }

    /// Generate a refresh token for a user. The refresh token has a longer
    /// lifetime (configured via `refresh_token_secs`, default 7 days) and
    /// shares the same session ID (`sid`) as the access token so it can be
    /// revoked on logout.
    pub fn generate_refresh_token(
        &self,
        user_id: &str,
        role: &str,
        sid: &str,
    ) -> Result<String, EnterpriseError> {
        let claims =
            RefreshTokenClaims::new(user_id, role, self.config.refresh_token_secs as i64, sid);
        let encoding_key = jsonwebtoken::EncodingKey::from_secret(self.config.jwt_secret.as_ref());
        jsonwebtoken::encode(&jsonwebtoken::Header::default(), &claims, &encoding_key).map_err(
            |e| EnterpriseError::JwtError {
                message: e.to_string(),
            },
        )
    }

    /// Validate a refresh token and return its claims. Uses the same HS256
    /// pin and leeway as [`validate_jwt`].
    pub fn validate_refresh_token(
        &self,
        token: &str,
    ) -> Result<RefreshTokenClaims, EnterpriseError> {
        let decoding_key = jsonwebtoken::DecodingKey::from_secret(self.config.jwt_secret.as_ref());
        let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
        validation.leeway = 60;
        validation.validate_exp = true;
        validation.validate_aud = false;
        let token_data =
            jsonwebtoken::decode::<RefreshTokenClaims>(token, &decoding_key, &validation).map_err(
                |e| {
                    let msg = e.to_string();
                    if msg.contains("ExpiredSignature") {
                        EnterpriseError::TokenExpired
                    } else {
                        EnterpriseError::AuthFailed {
                            message: format!("Invalid refresh token: {}", msg),
                        }
                    }
                },
            )?;
        if token_data.claims.typ != "refresh" {
            return Err(EnterpriseError::AuthFailed {
                message: "Expected refresh token".to_string(),
            });
        }
        Ok(token_data.claims)
    }

    /// Generate both an access token and a refresh token for a user, sharing
    /// the same session ID. Returns `(access_token, refresh_token,
    /// session_id, access_expires_at)`.
    pub fn generate_token_pair(
        &self,
        user_id: &str,
        role: &str,
    ) -> Result<(String, String, String, i64), EnterpriseError> {
        let claims = JwtClaims::new(user_id, role, self.config.jwt_expiration_secs as i64);
        let sid = claims
            .sid
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let encoding_key = jsonwebtoken::EncodingKey::from_secret(self.config.jwt_secret.as_ref());
        let access_token =
            jsonwebtoken::encode(&jsonwebtoken::Header::default(), &claims, &encoding_key)
                .map_err(|e| EnterpriseError::JwtError {
                    message: e.to_string(),
                })?;
        let refresh_token = self.generate_refresh_token(user_id, role, &sid)?;
        Ok((access_token, refresh_token, sid, claims.exp))
    }

    /// Session idle timeout in seconds (exposed for the auth middleware).
    pub fn session_idle_timeout_secs(&self) -> u64 {
        self.config.session_idle_timeout_secs
    }

    /// JWT signing secret (exposed for tests that need to craft tokens with
    /// custom claims, e.g. expired tokens).
    #[cfg(test)]
    pub(crate) fn jwt_secret(&self) -> &str {
        &self.config.jwt_secret
    }

    /// Invalidate a session
    pub fn invalidate_session(&self, session_id: &str) {
        self.sessions.write().remove(session_id);
    }

    /// Check if a session is valid
    pub fn is_session_valid(&self, session_id: &str) -> bool {
        self.sessions.read().contains_key(session_id)
    }
}

#[async_trait]
impl AuthProvider for AuthManager {
    async fn validate_token(&self, token: &str) -> Result<Identity, AuthError> {
        let claims = self.validate_jwt(token)?;
        Ok(Identity {
            user_id: claims.sub.clone(),
            username: claims.sub.clone(),
            role: claims.role.clone(),
            email: None,
            display_name: None,
            api_key_id: None,
            session_id: claims.sid,
            status: Some("active".to_string()),
            method: AuthMethod::Jwt,
        })
    }

    async fn validate_api_key(&self, key: &str) -> Result<Identity, AuthError> {
        let user_id = self.validate_api_key(key)?;
        Ok(Identity {
            user_id: user_id.clone(),
            username: user_id,
            role: "user".to_string(),
            email: None,
            display_name: None,
            api_key_id: Some(key.to_string()),
            session_id: None,
            status: Some("active".to_string()),
            method: AuthMethod::ApiKey,
        })
    }

    async fn authenticate_password(
        &self,
        _username: &str,
        _password: &str,
    ) -> Result<String, AuthError> {
        // TODO(Phase 4): implement Argon2id credential verification against a
        // persisted user store. Until the user store exists we cannot safely
        // validate passwords.
        Err(AuthError::AuthFailed {
            message: "Password authentication not yet implemented".to_string(),
        })
    }

    async fn generate_token(&self, user_id: &str, role: &str) -> Result<String, AuthError> {
        self.generate_jwt(user_id, role).map_err(From::from)
    }

    async fn create_api_key(&self, user_id: &str, name: &str) -> Result<String, AuthError> {
        let api_key = self.create_api_key(user_id, name);
        Ok(api_key.key)
    }

    async fn revoke_api_key(&self, key_id: &str) -> Result<(), AuthError> {
        self.revoke_api_key(key_id).map_err(From::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_manager() -> AuthManager {
        AuthManager::new(AuthConfig {
            enabled: true,
            jwt_secret: "test-secret-key-for-tests".to_string(),
            jwt_expiration_secs: 3600,
            refresh_token_secs: 7 * 24 * 3600,
            ..AuthConfig::default()
        })
    }

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
    fn test_jwt_expired_rejected() {
        // Build a token that is already expired by more than the 60s leeway.
        // We do this by crafting claims with a negative expiry offset rather
        // than sleeping, so the test runs in milliseconds.
        let mgr = AuthManager::new(AuthConfig {
            enabled: true,
            jwt_secret: "test-secret-key-for-tests".to_string(),
            jwt_expiration_secs: 3600,
            ..AuthConfig::default()
        });
        let mut claims = JwtClaims::new("user-1", "admin", 3600);
        // Set expiry to 120 seconds in the past — beyond the 60s leeway.
        claims.exp = chrono::Utc::now().timestamp() - 120;
        let encoding_key = jsonwebtoken::EncodingKey::from_secret(mgr.jwt_secret().as_ref());
        let token = jsonwebtoken::encode(&jsonwebtoken::Header::default(), &claims, &encoding_key)
            .expect("encode");
        let result = mgr.validate_jwt(&token);
        assert!(result.is_err(), "expired token should be rejected");
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
}

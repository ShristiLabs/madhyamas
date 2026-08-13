# Enterprise Performance and Security Analysis

> Part of: [Enterprise Analysis Overview](ENTERPRISE_OVERVIEW.md)

This document provides a comprehensive performance and security
analysis of the enterprise design across all components: the proxy
binary (enterprise tier), the licensing server, the web UI, the
multi-instance deployment, and the CI/CD pipeline. It consolidates
the scattered security and performance notes from other documents,
identifies gaps, and proposes additional mitigations.

---

## Table of Contents

1. [Security Analysis](#1-security-analysis)
2. [Threat Model](#2-threat-model)
3. [Security Gaps and Remediations](#3-security-gaps-and-remediations)
4. [Performance Analysis](#4-performance-analysis)
5. [Performance Bottlenecks and Optimizations](#5-performance-bottlenecks-and-optimizations)
6. [Database Optimization for High-Volume Traffic](#6-database-optimization-for-high-volume-traffic)
7. [Multi-Instance Performance and Security](#7-multi-instance-performance-and-security)
8. [Security Checklist](#8-security-checklist)
9. [Performance Checklist](#9-performance-checklist)

---

## 1. Security Analysis

### 1.1 Current security posture

The existing security measures are spread across multiple documents:

| Area | Current measures | Document |
|---|---|---|
| License signing | Ed25519, key rotation via `issuer_key_id`, Secrets Manager storage | [LICENSING_SERVER §13](ENTERPRISE_LICENSING_SERVER.md#13-security) |
| JWT auth | HMAC-SHA256, 1h expiry, secret from env/file, reject default in prod | [AUTH_RBAC §1.5](ENTERPRISE_AUTH_RBAC.md#15-security-requirements) |
| Password hashing | argon2id (memory-hard, GPU-resistant) | [OVERVIEW §10.1](ENTERPRISE_OVERVIEW.md#101-security-risks) |
| API key auth | SHA-256 hashed, tracked `last_used`, expiry support | [AUTH_RBAC §1](ENTERPRISE_AUTH_RBAC.md) |
| RBAC | Role-based (Admin/User/Viewer), permission per resource type | [AUTH_RBAC §2](ENTERPRISE_AUTH_RBAC.md#2-authorization-design) |
| CORS | Origin allowlist (localhost, private IPs, `.localhost` domains) | `madhyamas-api/src/lib.rs` |
| Rate limiting | `tower_governor` per-IP | [AUTH_RBAC §1.5](ENTERPRISE_AUTH_RBAC.md#15-security-requirements) |
| SQL injection | `sqlx` parameterized queries (planned), `rusqlite` params (current) | [LICENSING_SERVER §13](ENTERPRISE_LICENSING_SERVER.md#api-security) |
| XSS | React auto-escaping, CSP headers (planned) | [LICENSING_SERVER §13](ENTERPRISE_LICENSING_SERVER.md#api-security) |
| CSRF | SameSite cookies, CSRF token (planned for portal) | [LICENSING_SERVER §13](ENTERPRISE_LICENSING_SERVER.md#api-security) |
| Data encryption | TLS in transit, disk-level encryption at rest | [LICENSING_SERVER §13](ENTERPRISE_LICENSING_SERVER.md#data-encryption) |
| Audit logging | In-memory ring buffer (current), PostgreSQL (planned) | [ENTERPRISE.md](ENTERPRISE.md) |
| Stripe webhooks | Signature verification, idempotent processing | [LICENSING_SERVER §13](ENTERPRISE_LICENSING_SERVER.md#stripe-webhook-security) |
| CI/CD secrets | GitHub Actions secrets, AWS Secrets Manager for prod keys | [CICD §8](ENTERPRISE_CICD.md#8-secrets-management) |

### 1.2 Security architecture diagram

```mermaid
graph TD
    subgraph "Proxy binary (enterprise tier)"
        AUTH_MW["Auth Middleware<br/>JWT validation"]
        RBAC_MW["RBAC Middleware<br/>Permission check"]
        RATE["Rate Limiter<br/>tower_governor"]
        CORS_MW["CORS Layer<br/>Origin allowlist"]
        AUDIT["Audit Logger<br/>PostgreSQL"]
        LICENSE["License Verifier<br/>Ed25519 signature"]
    end

    subgraph "Licensing server"
        STRIPE["Stripe Webhook Handler<br/>Signature verification"]
        JWT_PORTAL["Portal JWT Auth<br/>HttpOnly cookies"]
        MFA["TOTP MFA<br/>Required for admins"]
        SIGNER["License Signer<br/>Ed25519 key in Secrets Manager"]
    end

    subgraph "Web UI (enterprise)"
        TOKEN["JWT in sessionStorage<br/>Auto-injected by API client"]
        TIER["Tier Detection<br/>Fallback to community"]
        LAZY["Enterprise features<br/>Lazy-loaded chunks"]
    end

    subgraph "Multi-instance"
        REDIS["Redis Pub/Sub<br/>TLS connection"]
        PG["PostgreSQL<br/>TLS + TDE"]
        SHARED_CA["Shared CA<br/>Volume or PG-backed"]
    end

    USER["User browser"] -->|"HTTPS + JWT"| AUTH_MW
    AUTH_MW --> RBAC_MW
    RBAC_MW --> RATE
    RATE --> CORS_MW
    CORS_MW --> AUDIT
    LICENSE -->|"offline verify"| AUDIT
```

---

## 2. Threat Model

### 2.1 Assets

| Asset | Sensitivity | Location |
|---|---|---|
| Ed25519 license signing private key | Critical | AWS Secrets Manager (licensing server) |
| JWT signing secret | High | Env var / file (proxy), Secrets Manager (licensing server) |
| User passwords | High | argon2id hashed (PostgreSQL) |
| API keys | High | SHA-256 hashed (PostgreSQL) |
| MFA TOTP secrets | High | AES-256-GCM encrypted (PostgreSQL) |
| License files | Medium | Customer filesystem |
| Traffic capture data | Medium | PostgreSQL / SQLite |
| Audit logs | Medium | PostgreSQL |
| Stripe API keys | High | Secrets Manager (licensing server) |
| CA private key (proxy) | High | Shared volume / PostgreSQL |
| Redis connection | Medium | Network (pub/sub event bus) |

### 2.2 Threat actors

| Actor | Capability | Motivation |
|---|---|---|
| Unauthenticated network attacker | Intercept/modify traffic between user and proxy | Steal JWT, inject traffic |
| Authenticated low-privilege user (Viewer) | Read traffic, sessions, config | Escalate to Admin, access other users' data |
| Authenticated user (User role) | Read/write traffic, mocks, rewrites | Escalate to Admin, access other users' data |
| Malicious insider (Admin) | Full CRUD, user management | Exfiltrate data, create backdoor accounts |
| License attacker | Reverse-engineer binary, forge license | Bypass licensing, use enterprise without paying |
| Attacker targeting licensing server | SQL injection, SSRF, key theft | Steal license signing key, mass-issue licenses |
| Container escape attacker | Break out of Docker/K8s | Access other tenants' data, steal secrets |
| Supply chain attacker | Compromise a dependency | Inject malicious code into build |

### 2.3 Attack surfaces

| Surface | Entry point | Current protection | Gap? |
|---|---|---|---|
| Proxy API (port 3001) | HTTP requests to `/api/*` | JWT auth, RBAC, rate limiting | See §3 |
| Proxy listener (port 8888) | HTTP/HTTPS proxy traffic | None (proxy is open by design) | See §3.5 |
| WebSocket (port 3001) | WS connection to `/ws` | None (no auth on WS) | **Yes — §3.6** |
| Web UI (embedded) | Browser | React auto-escaping, CSP (planned) | See §3.7 |
| Licensing server API | HTTP to `madhyamas.ai` | JWT, rate limiting, Stripe signature | See §3.8 |
| License file | File on customer disk | Ed25519 signature | See §3.9 |
| Redis (if multi-instance) | Network connection | None (assumed private network) | **Yes — §3.10** |
| PostgreSQL | Network connection | TLS, parameterized queries | See §3.11 |
| Docker image | Container registry | GHCR private (enterprise), public (simple) | See §3.12 |

---

## 3. Security Gaps and Remediations

### 3.1 JWT clock skew and validation gaps

**Current state:** `JwtClaims::is_expired()` compares `chrono::Utc::now().timestamp() > self.exp` with no clock skew tolerance. JWT validation in `AuthManager::validate_jwt` checks expiry but may not validate all required claims.

**Risk:** If server clocks are slightly out of sync (common in containers, NTP drift), valid tokens may be rejected or expired tokens may be accepted briefly.

**Remediation:**

```rust
// Add clock skew tolerance (±60 seconds)
const JWT_CLOCK_SKEW_SECS: i64 = 60;

pub fn is_expired(&self) -> bool {
    chrono::Utc::now().timestamp() > self.exp + JWT_CLOCK_SKEW_SECS
}

// In validate_jwt, also check:
// 1. `iat` (issued at) is not in the future (beyond skew)
// 2. `sub` (subject/user_id) is not empty
// 3. `role` is a valid role
// 4. Token is not in revocation list (if implemented)
```

| Gap | Severity | Remediation |
|---|---|---|
| No clock skew tolerance | Medium | Add ±60s tolerance |
| No `iat` validation | Low | Reject tokens with future `iat` |
| No `nbf` (not-before) support | Low | Add `nbf` claim support |
| No token revocation list | Medium | Redis-backed revocation list (enterprise) |
| JWT algorithm confusion (HS256 vs RS256) | Medium | Explicitly validate `alg` in header |

### 3.2 JWT algorithm confusion attack

**Current state:** JWT uses HMAC-SHA256 (`alg: HS256`). If the auth library allows `alg: none` or accepts RSA public keys as HMAC secrets, an attacker could forge tokens.

**Risk:** Token forgery bypassing authentication entirely.

**Remediation:**

```rust
// In validate_jwt, explicitly check the algorithm
use jsonwebtoken::{decode, Validation, Algorithm};

let mut validation = Validation::new(Algorithm::HS256);
validation.validate_exp = true;
validation.leeway = 60; // 60 seconds clock skew tolerance
// Reject "none" algorithm and any non-HS256 algorithm

let token_data = decode::<JwtClaims>(&token, &key, &validation)?;
```

### 3.3 Missing CSP headers on proxy web UI

**Current state:** The licensing server doc mentions CSP headers, but the proxy binary's embedded asset serving (`embedded_assets.rs`) does not set CSP headers.

**Risk:** XSS attacks could inject scripts that steal JWT from `sessionStorage` or make authenticated API calls.

**Remediation:**

```rust
// crates/madhyamas-api/src/embedded_assets.rs

fn security_headers() -> [(HeaderName, HeaderValue); 5] {
    [
        // CSP: only allow scripts from same origin (embedded assets)
        (header::CONTENT_SECURITY_POLICY,
         HeaderValue::from_static(
             "default-src 'self'; \
              script-src 'self'; \
              style-src 'self' 'unsafe-inline'; \
              img-src 'self' data:; \
              connect-src 'self' ws: wss:; \
              font-src 'self'; \
              object-src 'none'; \
              base-uri 'self'; \
              frame-ancestors 'none';"
         )),
        // X-Content-Type-Options: prevent MIME sniffing
        (header::X_CONTENT_TYPE_OPTIONS,
         HeaderValue::from_static("nosniff")),
        // X-Frame-Options: prevent clickjacking
        (header::X_FRAME_OPTIONS,
         HeaderValue::from_static("DENY")),
        // Strict-Transport-Security: enforce HTTPS
        (header::STRICT_TRANSPORT_SECURITY,
         HeaderValue::from_static("max-age=31536000; includeSubDomains")),
        // Referrer-Policy: minimal referrer
        (header::REFERRER_POLICY,
         HeaderValue::from_static("strict-origin-when-cross-origin")),
    ]
}
```

Note: `style-src 'unsafe-inline'` is needed because Tailwind injects
inline styles. If this is unacceptable, use a nonce-based CSP.

### 3.4 Missing CSRF protection for proxy API

**Current state:** The proxy API uses JWT in `Authorization: Bearer`
header, not cookies. CSRF primarily affects cookie-based auth. However,
if the enterprise tier adds cookie-based auth (e.g., for SSO
redirect flows), CSRF protection is needed.

**Risk:** If cookie auth is added without CSRF protection, an attacker
could craft a malicious page that makes authenticated requests on
behalf of a logged-in user.

**Remediation:**

If cookie-based auth is added:
1. Use `SameSite=Strict` or `SameSite=Lax` cookies
2. Add CSRF token: server generates random token, client sends it in
   `X-CSRF-Token` header, server validates
3. Only accept state-changing requests (POST/PUT/PATCH/DELETE) with
   the correct CSRF token

For the current JWT-in-header approach, CSRF is not a risk. Document
this explicitly so future changes don't introduce it.

### 3.5 Proxy listener has no authentication

**Current state:** The proxy listener (port 8888) accepts all
connections. This is by design — the proxy needs to accept traffic
from client apps (browsers, curl, etc.).

**Risk:** On a network-accessible host, anyone can use the proxy to
intercept HTTPS traffic (if they trust the CA) or relay traffic.

**Remediation:**

The existing `AccessControlList` (IP allowlist) is the primary
defense. For enterprise:

| Layer | Protection |
|---|---|
| Network | IP allowlist (`--allowed-ip`, `MADHYAMAS_ALLOWED_IPS`) |
| Proxy auth (enterprise) | Optional proxy authentication (HTTP Proxy-Authenticate header) |
| Container | Bind to `127.0.0.1` by default; expose only via K8s Service |
| Load balancer | Proxy port not exposed through LB (per-instance access only) |

**Proxy authentication (enterprise feature):**

```rust
// Optional: require Proxy-Authorization header
// Configured via MADHYAMAS_PROXY_AUTH=true
async fn proxy_auth_middleware(
    request: &Request,
    auth_manager: &AuthManager,
) -> Result<(), ProxyAuthError> {
    if !config.proxy_auth_enabled {
        return Ok(());
    }

    let auth = request.headers()
        .get("proxy-authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "));

    match auth {
        Some(token) => {
            auth_manager.validate_jwt(token)?;
            Ok(())
        }
        None => Err(ProxyAuthError::Unauthorized),
    }
}
```

### 3.6 WebSocket has no authentication

**Current state:** The WebSocket handler at `/ws` does not validate
JWT. Any client that can reach port 3001 can subscribe to real-time
traffic events.

**Risk:** An attacker on the same network can connect to the
WebSocket and see all traffic flowing through the proxy, including
headers, bodies, and cookies.

**Remediation:**

```rust
// crates/madhyamas-api/src/ws.rs — add auth check

pub async fn handle_ws(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    // Enterprise tier: validate JWT from query param or subprotocol
    #[cfg(feature = "enterprise")]
    {
        if let Some(auth) = &state.auth_service {
            // WebSocket can't use Authorization header reliably;
            // accept token from query param or Sec-WebSocket-Protocol
            // ...
            if !is_authorized {
                return ws_error_response(StatusCode::UNAUTHORIZED);
            }
        }
    }

    ws.on_upgrade(handle_ws_connection)
}
```

**WebSocket auth approaches:**

| Approach | Pros | Cons |
|---|---|---|
| Query param: `?token=...` | Simple, works with all browsers | Token in URL (logged in access logs) |
| Sec-WebSocket-Protocol | Not in URL | Non-standard usage |
| First message auth | Token not in URL | Extra round-trip; connection established before auth |
| Cookie-based | Automatic | Requires CSRF protection |

**Recommendation:** Query param with short-lived one-time WebSocket
token (exchanges JWT for a single-use WS token via API first).

### 3.7 Enterprise JS chunks leak feature information

**Current state:** Enterprise web UI code is embedded in the binary
via `rust-embed` but lazy-loaded. In the simple tier, the chunks are
present in the binary but never fetched by the browser.

**Risk:** A user with access to the binary could extract embedded
assets and inspect enterprise feature names, API endpoint paths, and
UI structure — even in the simple tier build.

**Assessment:** This is a **low-severity information leak**. The
enterprise code doesn't contain business logic (that's in Rust), just
UI components and API call patterns. The API endpoints return 404 in
the simple tier, so no features can be activated.

**Remediation options:**

| Option | Effort | Effectiveness |
|---|---|---|
| Accept the leak (current) | None | Low — UI structure is not sensitive |
| Build-time gating (`VITE_ENTERPRISE=true`) | Medium | Removes enterprise JS from simple build entirely |
| Separate enterprise web bundle (not embedded) | High | Enterprise UI served separately, not in binary |

**Recommendation:** Accept the leak for now. If enterprise UI code
becomes sensitive (e.g., contains proprietary algorithms), switch to
build-time gating. The CI/CD doc already documents this option.

### 3.8 Licensing server: no SSRF protection for SSO callbacks

**Current state:** The licensing server supports OIDC SSO (future).
OIDC redirect URIs are server-side and don't involve user-controlled
URLs. However, if the server fetches JWKS from an IdP, the IdP URL
could be manipulated.

**Risk:** If an attacker can control the IdP URL (e.g., via admin
config), they could make the server fetch from an internal service
(SSRF).

**Remediation:**

```rust
// Validate IdP URLs before fetching
fn validate_idp_url(url: &str) -> Result<()> {
    let parsed = url::Url::parse(url)?;
    // Must be HTTPS
    if parsed.scheme() != "https" {
        return Err("IdP URL must be HTTPS");
    }
    // Must not be a private/internal IP
    if let Some(host) = parsed.host_str() {
        if let Ok(ip) = host.parse::<std::net::IpAddr>() {
            if ip.is_loopback() || ip.is_private() || ip.is_link_local() {
                return Err("IdP URL must not point to private/internal address");
            }
        }
    }
    Ok(())
}
```

### 3.9 License file replay and tampering

**Current state:** License files are Ed25519-signed. Tampering is
detected by signature verification. Replay (using a license on
multiple deployments) is mitigated by soft binding (fingerprint
logging) and optional attestation.

**Risk:** A license file can be copied to multiple servers. The
current design logs fingerprint mismatches but doesn't enforce.

**Remediation:**

| Control | Type | Effectiveness |
|---|---|---|
| Ed25519 signature | Cryptographic | Prevents tampering |
| Fingerprint logging | Detective | Detects sharing (post-hoc) |
| Attestation endpoint | Detective | Detects multiple installations |
| Hard binding (machine ID) | Preventive | Prevents copying (but breaks migration) |
| Online revocation check | Detective | Detects revoked licenses |
| Short-lived licenses (e.g., 24h) | Preventive | Requires frequent online check |

**Recommendation:** Keep soft binding + attestation as default. Offer
hard binding as an option for strict licenses. Short-lived licenses
for trial only.

### 3.10 Redis has no authentication or TLS by default

**Current state:** The multi-instance design uses Redis for pub/sub.
The K8s manifest connects to Redis without authentication or TLS.

**Risk:** If Redis is on a shared network, an attacker could:
- Subscribe to traffic events (data leak)
- Publish fake events (config change injection, fake traffic)
- Flush the Redis instance (DoS)

**Remediation:**

```yaml
# Redis with authentication and TLS
apiVersion: apps/v1
kind: Deployment
metadata:
  name: madhyamas-redis
spec:
  template:
    spec:
      containers:
        - name: redis
          image: redis:7-alpine
          command: ["redis-server", "--requirepass", "$(REDIS_PASSWORD)", "--tls-port", "6379", "--port", "0", "--tls-cert-file", "/tls/tls.crt", "--tls-key-file", "/tls/tls.key", "--tls-ca-cert-file", "/tls/ca.crt", "--tls-auth-clients", "yes"]
          env:
            - name: REDIS_PASSWORD
              valueFrom:
                secretKeyRef:
                  name: madhyamas-redis-secret
                  key: password
          volumeMounts:
            - name: tls
              mountPath: /tls
      volumes:
        - name: tls
          secret:
            secretName: madhyamas-redis-tls
```

```rust
// RedisEventBus with TLS + auth
pub async fn new(redis_url: &str) -> Result<Self> {
    // redis://:password@host:6379/?tls=true
    let manager = redis::aio::ConnectionManager::new(redis_url).await?;
    // ...
}
```

**Minimum requirements:**
- Redis password (`requirepass`)
- Redis TLS (if on shared network)
- Network policy (K8s: restrict ingress to proxy pods only)
- Redis ACL (if Redis 6+: restrict to PUBLISH/SUBSCRIBE only)

### 3.11 PostgreSQL: connection string in env var

**Current state:** `DATABASE_URL` is passed as an env var. In K8s,
it's in a Secret (good). In Docker Compose, it may be in plaintext
(development only).

**Risk:** Env vars are visible in `/proc/<pid>/environ` and may be
logged by accident.

**Remediation:**

| Environment | Method |
|---|---|
| K8s | Secret → env var (current — acceptable) |
| Docker Compose (dev) | `.env` file (gitignored) |
| Docker Compose (prod) | Docker secrets or Vault |
| Bare metal | Config file with `0600` permissions |

Always use TLS for PostgreSQL connections:
```
DATABASE_URL=postgres://user:pass@host:5432/db?sslmode=require
```

### 3.12 Docker image: enterprise binary contains signing public keys

**Current state:** The enterprise binary embeds Ed25519 public keys
for license verification. These are public by design (verification
only).

**Risk:** None — public keys are not secret. An attacker knowing the
public key cannot forge licenses (they need the private key).

**No remediation needed.** Document this so reviewers don't flag it.

### 3.13 Audit log integrity

**Current state:** Audit logs are stored in PostgreSQL (enterprise)
or in-memory ring buffer (current). There's no tamper protection.

**Risk:** An admin with database access could modify or delete audit
entries to cover their tracks.

**Remediation:**

| Control | Implementation |
|---|---|
| Append-only table | `ALTER TABLE audit_events NO UPDATE; NO DELETE;` (PostgreSQL doesn't support this natively — use trigger) |
| Hash chaining | Each audit event includes `prev_hash = SHA256(prev_event || prev_hash)`. Tampering breaks the chain. |
| WORM storage | Write to S3 with Object Lock (compliance mode) |
| External log shipping | Stream audit events to external syslog/SIEM (can't be modified locally) |

```sql
-- Hash-chained audit events
CREATE TABLE audit_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_type TEXT NOT NULL,
    user_id UUID,
    description TEXT NOT NULL,
    metadata JSONB,
    client_ip INET,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    prev_hash TEXT NOT NULL,  -- SHA256 of previous event's hash
    event_hash TEXT NOT NULL   -- SHA256(id || event_type || user_id || description || metadata || client_ip || created_at || prev_hash)
);

-- Trigger to prevent UPDATE and DELETE
CREATE OR REPLACE FUNCTION prevent_audit_modification()
RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'audit_events is append-only';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER no_update_audit BEFORE UPDATE ON audit_events
    FOR EACH ROW EXECUTE FUNCTION prevent_audit_modification();
CREATE TRIGGER no_delete_audit BEFORE DELETE ON audit_events
    FOR EACH ROW EXECUTE FUNCTION prevent_audit_modification();
```

### 3.14 No password complexity enforcement

**Current state:** The auth design doesn't specify password complexity
rules.

**Risk:** Users choose weak passwords that are easily brute-forced.

**Remediation:**

```rust
pub fn validate_password_complexity(password: &str) -> Result<()> {
    if password.len() < 12 {
        return Err("Password must be at least 12 characters");
    }
    if !password.chars().any(|c| c.is_uppercase()) {
        return Err("Password must contain an uppercase letter");
    }
    if !password.chars().any(|c| c.is_lowercase()) {
        return Err("Password must contain a lowercase letter");
    }
    if !password.chars().any(|c| c.is_numeric()) {
        return Err("Password must contain a digit");
    }
    if !password.chars().any(|c| !c.is_alphanumeric()) {
        return Err("Password must contain a special character");
    }
    // Check against common password list (optional)
    if COMMON_PASSWORDS.contains(&password) {
        return Err("Password is too common");
    }
    Ok(())
}
```

### 3.15 No session timeout / idle timeout

**Current state:** JWT expires after 1 hour. There's no idle timeout
(a user who keeps making requests every 50 minutes stays logged in
indefinitely).

**Risk:** A compromised session stays active indefinitely as long as
the attacker makes periodic requests.

**Remediation:**

| Control | Implementation |
|---|---|
| Absolute timeout | JWT `exp` (current — 1h) |
| Idle timeout | Track `last_activity` in session; reject if idle > 15min |
| Refresh token rotation | Short access token (15min) + long refresh token (8h) with rotation |
| Max session age | Reject refresh after 8h regardless of activity |

**Recommendation:** Implement refresh token rotation:
- Access token: 15 minutes
- Refresh token: 8 hours, rotated on each use
- If a refresh token is used twice (replay), revoke all tokens for that user

### 3.16 No API key scope limitation

**Current state:** API keys grant the same permissions as the user
who created them. There's no scope limitation.

**Risk:** A script with an API key has full user permissions. If the
key leaks, the attacker has full access.

**Remediation:**

```rust
pub struct ApiKey {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub key_hash: String,
    pub scopes: Vec<ApiKeyScope>,  // NEW: limit what the key can do
    pub expires_at: Option<i64>,
    pub last_used: Option<i64>,
    pub created_at: i64,
}

pub enum ApiKeyScope {
    TrafficRead,      // GET /api/traffic/*
    TrafficWrite,     // POST/PATCH /api/traffic/*
    InterceptManage,  // CRUD on mocks, rewrites, breakpoints
    ConfigRead,       // GET /api/config
    ConfigWrite,      // PATCH /api/config
    Export,           // GET /api/export/*
    // No scope = full access (backward compatible)
}
```

### 3.17 Summary of security gaps

| # | Gap | Severity | Section | Status in docs |
|---|---|---|---|---|
| 1 | JWT clock skew tolerance | Medium | §3.1 | **New** |
| 2 | JWT algorithm confusion | Medium | §3.2 | **New** |
| 3 | Missing CSP headers on proxy | High | §3.3 | **New** |
| 4 | CSRF if cookie auth added | Low (future) | §3.4 | **New** |
| 5 | Proxy listener no auth | Medium | §3.5 | Partially covered |
| 6 | WebSocket no auth | **High** | §3.6 | **New** |
| 7 | Enterprise JS leak | Low | §3.7 | Covered in OVERVIEW §10 |
| 8 | SSRF for SSO callbacks | Medium | §3.8 | **New** |
| 9 | License replay | Low | §3.9 | Covered in LICENSING_SERVER §13 |
| 10 | Redis no auth/TLS | **High** | §3.10 | **New** |
| 11 | PG connection string exposure | Low | §3.11 | Partially covered |
| 12 | Enterprise binary public keys | None | §3.12 | **New (documented as non-issue)** |
| 13 | Audit log integrity | Medium | §3.13 | **New** |
| 14 | No password complexity | Medium | §3.14 | **New** |
| 15 | No session idle timeout | Medium | §3.15 | **New** |
| 16 | No API key scopes | Medium | §3.16 | **New** |

---

## 4. Performance Analysis

### 4.1 Current performance characteristics

The current single-instance architecture has these performance
properties:

| Component | Bottleneck | Current capacity | Limiting factor |
|---|---|---|---|
| Proxy engine (TLS interception) | CPU (per-connection TLS) | ~500 concurrent connections | CPU cores, TLS handshake cost |
| Traffic store (SQLite) | Disk I/O + `Mutex<Connection>` | ~1000 writes/sec | Single writer lock |
| Intercept pipeline | CPU (5 handlers per request) | ~10k requests/sec | Handler complexity (regex, mock matching) |
| WebSocket broadcast | Memory (fan-out to N clients) | ~100 concurrent WS clients | `broadcast` channel capacity |
| API server (axum) | Tokio runtime | ~10k req/sec | Connection pool, handler complexity |
| Web UI (embedded) | Binary size | Instant (embedded) | None |
| Script execution (boa_engine) | CPU (JS interpretation) | ~100 scripts/sec | Boa engine speed |
| Plugin execution (wasmtime) | CPU (WASM compilation) | ~1000 invocations/sec (after compile) | First-call compilation cost |

### 4.2 Enterprise tier performance impact

Adding enterprise features impacts performance:

| Feature | Impact | Reason | Mitigation |
|---|---|---|---|
| JWT validation on every request | +0.5ms per request | HMAC-SHA256 verify + claim parsing | Cache validated tokens (short TTL) |
| RBAC permission check | +0.1ms per request | HashMap lookup | Already fast |
| Audit logging | +1-5ms per request | PostgreSQL INSERT | Batch inserts (async) |
| PostgreSQL (vs SQLite) | +2-5ms per query | Network round-trip | Connection pooling, prepared statements |
| Redis pub/sub | +1ms per event | Network round-trip to Redis | Pipeline publishes |
| License verification (startup) | +50ms (one-time) | Ed25519 verify | Cache result in memory |
| Password hashing (argon2id) | +100-500ms per login | Memory-hard by design | Only on login, not every request |
| Config sync reconciliation | +5ms every 30s | PG query + hash compare | Background task, non-blocking |

### 4.3 Performance architecture

```mermaid
graph TD
    subgraph "Hot path (every proxy request)"
        REQ["Incoming request"] --> INT["Intercept pipeline<br/>~0.5ms"]
        INT --> FWD["Forward to upstream<br/>~network latency"]
        FWD --> RESP["Response received"]
        RESP --> STORE["Store traffic entry<br/>~2ms (PG) or ~0.5ms (SQLite)"]
        STORE --> EVT["Broadcast event<br/>~0.1ms (local) or ~1ms (Redis)"]
        EVT --> AUDIT["Audit log (if needed)<br/>~1ms (async batch)"]
    end

    subgraph "Auth path (every API request)"
        API["API request"] --> JWT["JWT validate<br/>~0.5ms"]
        JWT --> RBAC["RBAC check<br/>~0.1ms"]
        RBAC --> HANDLER["Handler logic<br/>~varies"]
    end

    subgraph "Background (non-blocking)"
        RECONCILE["Config reconciliation<br/>every 30s"]
        HEARTBEAT["Instance heartbeat<br/>every 10s"]
        AUDIT_FLUSH["Audit batch flush<br/>every 1s"]
        GC["Memory GC<br/>on pressure"]
    end
```

---

## 5. Performance Bottlenecks and Optimizations

### 5.1 SQLite `Mutex<Connection>` — single writer

**Current state:** `TrafficStore` uses `Mutex<Connection>` (parking_lot
Mutex). All reads and writes serialize through a single connection.

**Impact:** Under high traffic (1000+ req/sec), writes block reads.
The Mutex contention becomes the bottleneck.

**Enterprise solution:** PostgreSQL with `sqlx::PgPool` (multiple
connections, MVCC for concurrent reads/writes). This is already
planned in [STORAGE_TRAITS.md](ENTERPRISE_STORAGE_TRAITS.md).

**Simple tier optimization:** Use WAL mode (already enabled) and
consider `rwc` (read-write-connect) pooling with `sqlx::SqlitePool`:

```rust
// sqlx SQLite pool with WAL mode
let pool = SqlitePoolOptions::new()
    .max_connections(5)
    .connect_with(SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .pragma("synchronous", "normal"))
    .await?;
```

### 5.2 Intercept pipeline — regex compilation

**Current state:** `BlockListManager` and `RewriteManager` compile
regex patterns on every match. The `regex_cache` module caches
compiled regexes, but the cache is a global `Mutex<HashMap>`.

**Impact:** First match of a pattern is slow (regex compilation ~1ms).
Subsequent matches are fast (cache hit ~0.01ms). Under high traffic
with many patterns, the Mutex on the cache can contend.

**Optimization:**

```rust
// Use DashMap for concurrent regex cache (no Mutex)
use dashmap::DashMap;

static REGEX_CACHE: OnceLock<DashMap<String, Regex>> = OnceLock::new();

fn cache() -> &'static DashMap<String, Regex> {
    REGEX_CACHE.get_or_init(|| DashMap::new())
}
```

Or pre-compile all patterns when loading from the store:

```rust
// In BlockListManager::reload()
let loaded = store.load_block_list_entries()?;
let compiled: Vec<(BlockListEntry, Option<Regex>)> = loaded
    .into_iter()
    .map(|entry| {
        let regex = if entry.is_regex {
            Regex::new(&entry.pattern).ok()
        } else {
            None
        };
        (entry, regex)
    })
    .collect();
// Store compiled regexes alongside entries
```

### 5.3 Audit logging — synchronous insert per request

**Current state:** Audit events are written to the database
synchronously per request (or stored in-memory in the current
implementation).

**Impact:** Each audited API request adds 1-5ms for the database
insert. Under high load, this compounds.

**Optimization:** Batch audit inserts with a background flusher:

```rust
pub struct AuditBatcher {
    buffer: tokio::sync::mpsc::Sender<AuditEvent>,
}

impl AuditBatcher {
    pub fn new(pool: PgPool) -> Self {
        let (tx, mut rx) = tokio::sync::mpsc::channel(10000);

        // Background flusher: flushes every 1s or when buffer is full
        tokio::spawn(async move {
            let mut batch = Vec::with_capacity(100);
            let mut interval = tokio::time::interval(Duration::from_secs(1));

            loop {
                tokio::select! {
                    Some(event) = rx.recv() => {
                        batch.push(event);
                        if batch.len() >= 100 {
                            flush_batch(&pool, &mut batch).await;
                        }
                    }
                    _ = interval.tick() => {
                        if !batch.is_empty() {
                            flush_batch(&pool, &mut batch).await;
                        }
                    }
                }
            }
        });

        Self { buffer: tx }
    }

    pub async fn log(&self, event: AuditEvent) {
        // Non-blocking send; if buffer is full, drop event (better than blocking)
        let _ = self.buffer.try_send(event);
    }
}

async fn flush_batch(pool: &PgPool, batch: &mut Vec<AuditEvent>) {
    // Single INSERT with multiple values
    sqlx::query("INSERT INTO audit_events (event_type, user_id, description, metadata, client_ip) VALUES ($1, $2, $3, $4, $5)")
        // ... bind batch ...
        .execute(pool)
        .await
        .ok();
    batch.clear();
}
```

### 5.4 WebSocket broadcast — fan-out cost

**Current state:** `broadcast::Sender::send()` copies the event to
each receiver. With 100 concurrent WS clients, each traffic event
is copied 100 times.

**Impact:** Memory allocation pressure under high traffic + many WS
clients. `broadcast` has a fixed buffer; slow consumers miss events.

**Optimization:**

| Technique | Impact |
|---|---|
| Use `Arc<TrafficEvent>` instead of `TrafficEvent` in broadcast | Eliminates copy — all receivers share one allocation |
| Increase broadcast channel capacity | Reduces missed events for slow consumers |
| Client-side polling fallback | Already implemented (WebSocket mode toggle in UI) |
| Event filtering per client | Only send events matching client's current session/filter |

```rust
// Use Arc in broadcast channel
let (sender, _) = broadcast::channel::<Arc<TrafficEvent>>(1024);

// When emitting:
self.event_sender.send(Arc::new(event));
```

### 5.5 PostgreSQL connection pool sizing

**Current state:** Not yet implemented (planned in storage traits).

**Impact:** Too few connections = requests wait for pool. Too many =
PostgreSQL runs out of connections.

**Recommendation:**

| Deployment | Pool size | Calculation |
|---|---|---|
| Single instance | 10 | `max_connections = 10` (default) |
| 3 instances | 10 per instance (30 total) | `max_connections = 10` per instance |
| 10 instances | 5 per instance (50 total) | Reduce per-instance pool; add PgBouncer |
| 50+ instances | 2 per instance + PgBouncer | External connection pooler |

```rust
// Configurable pool size
let pool = PgPoolOptions::new()
    .max_connections(env::var("MADHYAMAS_DB_MAX_CONNECTIONS")
        .unwrap_or("10".to_string())
        .parse()?)
    .min_connections(2)  // Keep 2 warm
    .acquire_timeout(Duration::from_secs(5))
    .idle_timeout(Duration::from_secs(600))
    .max_lifetime(Duration::from_secs(1800))
    .connect(&database_url)
    .await?;
```

### 5.6 Redis pub/sub — event serialization overhead

**Current state:** Events are serialized to JSON before publishing to
Redis, and deserialized on receipt.

**Impact:** JSON serialization adds ~0.1-0.5ms per event depending on
event size. For high-traffic proxies (1000+ events/sec), this is
significant.

**Optimization:**

| Technique | Serialization | Size | Speed |
|---|---|---|---|
| JSON (current) | `serde_json` | Large | Slow |
| MessagePack | `rmp-serde` | Small | Fast |
| Bincode | `bincode` | Smallest | Fastest |

```rust
// Use bincode for Redis pub/sub (not JSON)
let bytes = bincode::serialize(&event)?;
let _: () = conn.publish("madhyamas:events", bytes).await?;

// On receive
let event: ClusterEvent = bincode::deserialize(&msg)?;
```

**Trade-off:** Bincode is not human-readable (harder to debug). Use
JSON for development, bincode for production (configurable).

### 5.7 License verification — startup cost

**Current state:** License verification runs at startup (Ed25519
signature verification + claim parsing).

**Impact:** ~50ms one-time cost. Not a per-request overhead.

**Optimization:** None needed. The cost is one-time and small. Cache
the verification result in memory for the process lifetime.

### 5.8 Memory usage — traffic capture with bodies

**Current state:** Traffic store captures request and response bodies
(up to `max_body_size` = 20MB per entry). With 10,000 entries and
large bodies, memory usage can reach 500MB+.

**Impact:** OOM kills in containerized deployments with limited
memory.

**Optimization:**

| Technique | Impact |
|---|---|
| Stream bodies to PostgreSQL (not in-memory) | Reduces peak memory |
| Compress bodies (gzip/zstd) before storage | Reduces DB size and memory |
| Lazy body loading (fetch on demand) | Reduces memory for list views |
| Memory pressure detection + GC | Already implemented (`MemoryManager`) |
| Configurable body capture (per-domain) | Already implemented (`ignored_domains`) |

```rust
// Compress bodies before storing
use flate2::Compression;
use flate2::write::GzEncoder;

fn compress_body(body: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(body).ok();
    encoder.finish().ok().unwrap_or_default()
}
```

### 5.9 Summary of performance optimizations

| # | Bottleneck | Severity | Solution | Phase |
|---|---|---|---|---|
| 1 | SQLite single-writer lock | High | Migrate to PostgreSQL + PgPool | MI-1 |
| 2 | Regex compilation per match | Medium | Pre-compile on reload, DashMap cache | MI-3 |
| 3 | Synchronous audit insert | Medium | Batch inserts with background flusher | Phase 3 |
| 4 | WebSocket broadcast copy | Low | Use `Arc<TrafficEvent>` in broadcast | MI-2 |
| 5 | PG pool sizing | Medium | Configurable, PgBouncer for 10+ instances | MI-1 |
| 6 | Redis JSON serialization | Low | Use bincode in production | MI-2 |
| 7 | License verify startup | None | One-time, 50ms — acceptable | — |
| 8 | Traffic body memory | Medium | Stream + compress + lazy load | Phase 2 |
| 9 | JWT validation per request | Low | Cache validated tokens (short TTL) | Phase 3 |
| 10 | argon2id login cost | None | 100-500ms by design — only on login | — |

---

## 6. Database Optimization for High-Volume Traffic

### 6.1 The problem: naive PostgreSQL migration doesn't scale

The current SQLite schema works for single-instance, low-volume
debugging. But a direct port to PostgreSQL (as sketched in
[STORAGE_TRAITS.md §1.5](ENTERPRISE_STORAGE_TRAITS.md)) will not
scale to high-volume enterprise traffic. Here's why:

| Current pattern | Why it fails at scale | Impact at 1000 req/sec |
|---|---|---|
| `INSERT` per request + `INSERT` per response | 2 round-trips per HTTP transaction | 2000 writes/sec — pool contention |
| `COUNT(*)` on every `store_request` to check `max_entries` | Full table scan (or index-only scan) per insert | 1000 extra queries/sec |
| `SUM(LENGTH(body))` every 100 inserts to check `max_total_size` | Reads every body byte from disk | Multi-second query; blocks writes |
| `LIKE '%pattern%'` for URL/search/header filtering | No index can help — full table scan | UI freezes on filter with 100k+ entries |
| Headers as `TEXT` (JSON string) | No structured query; `LIKE` scans all rows | Header filter is O(n) |
| Bodies up to 20MB stored as `BYTEA` inline | TOAST overhead; bloats table; slow vacuum | 20GB table for 1000 entries with large bodies |
| No partitioning | Single table grows unbounded; vacuum takes hours | Degraded performance over time |
| No data retention beyond FIFO pruning | `DELETE` + `VACUUM` churn | Bloat, fragmentation |
| No separation of metadata and bodies | List view loads bodies it doesn't need | 100x more data transferred than needed |

### 6.2 Data volume analysis

Before designing the schema, let's quantify the data volume at
different traffic levels:

| Traffic level | Req/sec | Entries/min | Metadata/day | Bodies/day (avg 50KB) | Bodies/day (avg 500KB) |
|---|---|---|---|---|---|
| Solo developer | 5 | 300 | ~40 MB | ~1.5 GB | ~15 GB |
| Small team (10 devs) | 50 | 3,000 | ~400 MB | ~15 GB | ~150 GB |
| Medium team (50 devs) | 200 | 12,000 | ~1.6 GB | ~60 GB | ~600 GB |
| Large team (100 devs) | 500 | 30,000 | ~4 GB | ~150 GB | ~1.5 TB |
| Enterprise (500 devs) | 2,000 | 120,000 | ~16 GB | ~600 GB | ~6 TB |

**Metadata** = id, session_id, method, url, host, path, status,
duration, timestamp (~400 bytes per entry).

**Bodies** = request body + response body (variable; 50KB avg for API
traffic, 500KB avg for media-heavy traffic).

At enterprise scale, **bodies dominate storage** by 100-1000x. The
schema must treat bodies differently from metadata.

### 6.3 Tiered storage architecture

```mermaid
graph TD
    REQ["Incoming traffic"] --> CLASSIFY{"Body size?"}

    CLASSIFY -->|"≤ 1KB (small)"| PG_INLINE["PostgreSQL<br/>body BYTEA inline<br/>(fast list view)"]
    CLASSIFY -->|"1KB - 100KB (medium)"| PG_TOAST["PostgreSQL<br/>body BYTEA TOAST<br/>(compressed by PG)"]
    CLASSIFY -->|"> 100KB (large)"| S3["S3 / MinIO<br/>body stored as object<br/>(reference in PG)"]
    CLASSIFY -->|"capture disabled"| META_ONLY["PostgreSQL<br/>metadata only<br/>(no body)"]

    PG_INLINE --> TABLE["traffic_entries table"]
    PG_TOAST --> TABLE
    S3 --> REF["body_ref TEXT<br/>(S3 key)"]
    META_ONLY --> TABLE
    REF --> TABLE

    TABLE --> INDEXES["Indexes:<br/>session_id + timestamp (B-tree)<br/>host (B-tree)<br/>headers (GIN JSONB)<br/>timestamp (BRIN)"]
```

The key insight: **not all bodies need to be in PostgreSQL**. Small
bodies (API JSON, headers) go inline. Large bodies (images, video,
file downloads) go to S3/MinIO with a reference in PostgreSQL.

| Tier | Body size | Storage | Rationale |
|---|---|---|---|
| Inline | ≤ 1 KB | PostgreSQL `BYTEA` (inline) | Small enough to not TOAST; fast list view |
| TOAST | 1 KB - 100 KB | PostgreSQL `BYTEA` (TOAST) | PostgreSQL compresses automatically; reasonable for medium bodies |
| External | > 100 KB | S3 / MinIO object | Avoids table bloat; cheaper storage; lazy-loaded only when user opens detail |
| Skipped | Any | Not stored | `capture_request_bodies=false` or `capture_response_bodies=false` |

```sql
-- traffic_entries: metadata + small/medium bodies
CREATE TABLE traffic_entries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    instance_id UUID,  -- which proxy instance captured this (NULL for single-instance)

    -- Request metadata (always stored — small)
    method VARCHAR(10) NOT NULL,
    url TEXT NOT NULL,
    host VARCHAR(255) NOT NULL,
    path TEXT NOT NULL,
    request_headers JSONB NOT NULL DEFAULT '{}',
    request_content_type VARCHAR(255),
    request_size INTEGER NOT NULL DEFAULT 0,
    http_version VARCHAR(10),

    -- Request body (NULL if not captured, inline if small, NULL if external)
    request_body BYTEA,  -- NULL if body_ref is set or capture disabled
    request_body_ref TEXT,  -- S3 key if body is in external storage
    request_body_compressed BOOLEAN NOT NULL DEFAULT false,

    -- Response metadata (NULL if request is still pending)
    status_code SMALLINT,
    status_message VARCHAR(100),
    response_headers JSONB,
    response_content_type VARCHAR(255),
    response_size INTEGER,
    response_duration_ms INTEGER,
    response_http_version VARCHAR(10),

    -- Response body (same tiering as request body)
    response_body BYTEA,
    response_body_ref TEXT,
    response_body_compressed BOOLEAN NOT NULL DEFAULT false,

    -- Metadata
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    modified BOOLEAN NOT NULL DEFAULT false,
    notes TEXT,
    is_passthrough BOOLEAN NOT NULL DEFAULT false,
    script_intercepted BOOLEAN NOT NULL DEFAULT false,

    -- Constraints: body is either inline, external, or absent
    CONSTRAINT body_storage CHECK (
        (request_body IS NOT NULL AND request_body_ref IS NULL) OR
        (request_body IS NULL AND request_body_ref IS NOT NULL) OR
        (request_body IS NULL AND request_body_ref IS NULL)  -- not captured
    ),
    CONSTRAINT response_body_storage CHECK (
        (response_body IS NOT NULL AND response_body_ref IS NULL) OR
        (response_body IS NULL AND response_body_ref IS NOT NULL) OR
        (response_body IS NULL AND response_body_ref IS NULL)
    )
);
```

### 6.4 Body compression

Bodies should be compressed before storage, regardless of tier:

| Body type | Compression | Ratio | CPU cost |
|---|---|---|---|
| JSON / XML | zstd level 3 | 5-10x | Low |
| HTML / text | zstd level 3 | 3-5x | Low |
| Images / video | None (already compressed) | 1x | None |
| Binary (protobuf, msgpack) | zstd level 1 | 1.5-3x | Low |

```rust
// crates/madhyamas-core/src/traffic/body_storage.rs

use zstd::stream::{encode_all, decode_all};

const COMPRESSION_THRESHOLD: usize = 256;  // Don't compress tiny bodies
const COMPRESSION_LEVEL: i32 = 3;
const INLINE_THRESHOLD: usize = 1024;       // ≤1KB: inline in PG
const TOAST_THRESHOLD: usize = 100 * 1024;  // ≤100KB: TOAST in PG
// >100KB: external (S3)

pub enum BodyStorage {
    /// Body is small enough to store inline in PostgreSQL (compressed).
    Inline { data: Vec<u8>, compressed: bool },
    /// Body is medium-sized; stored in PostgreSQL TOAST (compressed).
    Toast { data: Vec<u8>, compressed: bool },
    /// Body is large; stored in S3. The reference is the S3 object key.
    External { s3_key: String, size: usize, compressed: bool },
    /// Body was not captured (capture disabled or ignored domain).
    NotCaptured,
}

pub fn classify_body(body: &[u8], content_type: Option<&str>) -> BodyStorage {
    if body.is_empty() {
        return BodyStorage::NotCaptured;
    }

    // Don't compress already-compressed content types
    let should_compress = !is_already_compressed(content_type);
    let compressed_body = if should_compress && body.len() > COMPRESSION_THRESHOLD {
        encode_all(body, COMPRESSION_LEVEL).unwrap_or_else(|_| body.to_vec())
    } else {
        body.to_vec()
    };

    let compressed = compressed_body.len() < body.len();
    let data = compressed_body;

    if data.len() <= INLINE_THRESHOLD {
        BodyStorage::Inline { data, compressed }
    } else if data.len() <= TOAST_THRESHOLD {
        BodyStorage::Toast { data, compressed }
    } else {
        // Caller handles S3 upload and returns the key
        BodyStorage::External {
            s3_key: String::new(),  // filled by caller
            size: data.len(),
            compressed,
        }
    }
}

fn is_already_compressed(content_type: Option<&str>) -> bool {
    match content_type {
        Some(ct) => {
            let ct = ct.to_lowercase();
            ct.contains("gzip") ||
            ct.contains("deflate") ||
            ct.contains("br") ||
            ct.contains("image/") ||
            ct.contains("video/") ||
            ct.contains("audio/") ||
            ct.contains("application/zip") ||
            ct.contains("application/gzip") ||
            ct.contains("application/x-7z") ||
            ct.contains("application/x-rar") ||
            ct.contains("font/") ||
            ct.contains("application/octet-stream")
        }
        None => false,
    }
}
```

### 6.5 Write batching

The current pattern of one INSERT per request + one INSERT per
response is 2 round-trips per HTTP transaction. At 1000 req/sec,
that's 2000 writes/sec — enough to saturate a PostgreSQL connection
pool.

**Solution: Batch inserts via a channel + background flusher.**

```mermaid
sequenceDiagram
    participant Proxy as Proxy Engine
    participant Chan as mpsc Channel
    participant Flusher as Background Flusher
    participant PG as PostgreSQL
    participant S3 as S3 / MinIO

    Proxy->>Chan: send(TrafficEntry) [non-blocking]
    Note over Chan: Buffer up to 100 entries<br/>or 1 second timeout

    Flusher->>Chan: drain entries
    Chan-->>Flusher: Vec<TrafficEntry> (100 entries)

    Note over Flusher: Classify bodies
    Flusher->>S3: PUT large bodies (parallel)
    S3-->>Flusher: S3 keys

    Note over Flusher: Build batch INSERT
    Flusher->>PG: INSERT INTO traffic_entries VALUES ($1...), ($2...), ... (100 rows)
    PG-->>Flusher: OK (1 round-trip)

    Note over Flusher: Broadcast events
    Flusher->>PG: NOTIFY traffic_events (batch)
```

```rust
// crates/madhyamas-enterprise/src/traffic/batcher.rs

use tokio::sync::mpsc;
use std::time::Duration;

const BATCH_SIZE: usize = 100;
const BATCH_TIMEOUT: Duration = Duration::from_millis(500);

pub struct TrafficBatcher {
    sender: mpsc::Sender<TrafficEntry>,
}

impl TrafficBatcher {
    pub fn new(pool: PgPool, s3: Option<S3Client>, event_bus: Arc<dyn EventBus>) -> Self {
        let (tx, rx) = mpsc::channel(10_000);

        tokio::spawn(async move {
            batch_flush_loop(rx, pool, s3, event_bus).await;
        });

        Self { sender: tx }
    }

    /// Non-blocking send. If the channel is full, the entry is dropped
    /// (better than blocking the proxy pipeline).
    pub async fn store(&self, entry: TrafficEntry) {
        let _ = self.sender.try_send(entry);
    }
}

async fn batch_flush_loop(
    mut rx: mpsc::Receiver<TrafficEntry>,
    pool: PgPool,
    s3: Option<S3Client>,
    event_bus: Arc<dyn EventBus>,
) {
    let mut batch = Vec::with_capacity(BATCH_SIZE);
    let mut timeout = tokio::time::interval(BATCH_TIMEOUT);

    loop {
        tokio::select! {
            Some(entry) = rx.recv() => {
                batch.push(entry);
                if batch.len() >= BATCH_SIZE {
                    flush_batch(&mut batch, &pool, &s3, &event_bus).await;
                }
            }
            _ = timeout.tick() => {
                if !batch.is_empty() {
                    flush_batch(&mut batch, &pool, &s3, &event_bus).await;
                }
            }
        }
    }
}

async fn flush_batch(
    batch: &mut Vec<TrafficEntry>,
    pool: &PgPool,
    s3: &Option<S3Client>,
    event_bus: &Arc<dyn EventBus>,
) {
    if batch.is_empty() {
        return;
    }

    // 1. Upload large bodies to S3 (in parallel)
    if let Some(s3) = s3 {
        let futures: Vec<_> = batch.iter()
            .filter_map(|e| {
                if e.request.body.as_ref().map(|b| b.len()).unwrap_or(0) > TOAST_THRESHOLD {
                    Some(upload_body_to_s3(s3.clone(), e.id.clone(), e.request.body.clone()))
                } else {
                    None
                }
            })
            .collect();
        // Wait for all S3 uploads (with timeout)
        let _ = futures::future::join_all(futures).await;
    }

    // 2. Build a single multi-row INSERT
    // INSERT INTO traffic_entries (id, session_id, method, ...) VALUES
    //   ($1, $2, $3, ...),
    //   ($101, $102, $103, ...),
    //   ...
    let placeholders = build_placeholders(batch.len(), 20);  // 20 columns
    let sql = format!(
        "INSERT INTO traffic_entries
         (id, session_id, method, url, host, path, request_headers,
          request_body, request_body_ref, request_body_compressed,
          status_code, response_headers, response_body, response_body_ref,
          response_body_compressed, timestamp, request_size, response_size,
          response_duration_ms, is_passthrough)
         VALUES {}",
        placeholders
    );

    let mut query = sqlx::query(&sql);
    for entry in batch.iter() {
        query = query
            .bind(entry.id)
            .bind(entry.session_id)
            .bind(&entry.request.method)
            .bind(&entry.request.url)
            .bind(&entry.request.host)
            .bind(&entry.request.path)
            .bind(serde_json::to_value(&entry.request.headers).unwrap_or_default())
            .bind(&entry.request.body)  // NULL if external
            .bind(entry.request_body_s3_key.as_ref())  // NULL if inline
            .bind(entry.request_body_compressed)
            .bind(entry.response.as_ref().map(|r| r.status_code as i16))
            .bind(entry.response.as_ref().map(|r| serde_json::to_value(&r.headers).unwrap_or_default()))
            .bind(entry.response.as_ref().and_then(|r| r.body.as_ref()))
            .bind(entry.response_body_s3_key.as_ref())
            .bind(entry.response_body_compressed)
            .bind(entry.timestamp)
            .bind(entry.request_size as i32)
            .bind(entry.response_size.map(|s| s as i32))
            .bind(entry.response.as_ref().map(|r| r.duration_ms as i32))
            .bind(entry.is_passthrough);
    }

    if let Err(e) = query.execute(pool).await {
        error!("Batch insert failed ({} entries): {}", batch.len(), e);
        // Fallback: insert one at a time (slower but preserves data)
        for entry in batch.drain(..) {
            let _ = insert_single(pool, entry).await;
        }
    }

    // 3. Broadcast events for all entries in the batch
    for entry in batch.drain(..) {
        let _ = event_bus.publish(ClusterEvent::TrafficEvent {
            instance_id: event_bus.instance_id().to_string(),
            event: TrafficEvent::EntryStored(entry.id.clone()),
        }).await;
    }
}
```

**Performance impact:**

| Pattern | Round-trips/sec (1000 req/sec) | Pool connections used |
|---|---|---|
| Current (1 INSERT per entry) | 2000 | 2 (sustained) |
| Batched (100 per batch, 500ms) | 10 | 1 (burst) |
| Batched + S3 (parallel upload) | 10 + S3 async | 1 + S3 workers |

### 6.6 Indexing strategy

The current SQLite schema has 4 indexes on `requests`:
`session_id`, `url`, `method`, `timestamp`. These are B-tree indexes
that don't help with `LIKE '%pattern%'` queries.

PostgreSQL needs a fundamentally different indexing strategy:

```sql
-- === Critical indexes for traffic_entries ===

-- 1. Primary lookup: session + time range (list view, pagination)
--    This is the most common query: "give me traffic for session X,
--    newest first, page N"
CREATE INDEX idx_traffic_session_time
    ON traffic_entries (session_id, timestamp DESC);

-- 2. Host filtering (focus hosts, host-based search)
CREATE INDEX idx_traffic_host
    ON traffic_entries (host)
    WHERE host != '';  -- Partial index: skip empty hosts

-- 3. Status code filtering (error filtering: 4xx, 5xx)
CREATE INDEX idx_traffic_status
    ON traffic_entries (status_code)
    WHERE status_code IS NOT NULL;

-- 4. Header search via GIN (JSONB operations: @>, ?, ?|)
--    Enables: WHERE request_headers @> '{"Content-Type": "application/json"}'
--    Enables: WHERE request_headers ? 'Authorization'
CREATE INDEX idx_traffic_req_headers
    ON traffic_entries USING GIN (request_headers)
    WHERE request_headers != '{}';

CREATE INDEX idx_traffic_resp_headers
    ON traffic_entries USING GIN (response_headers)
    WHERE response_headers IS NOT NULL AND response_headers != '{}';

-- 5. Timestamp range (BRIN — Block Range Index, 100x smaller than B-tree)
--    Efficient for time-range queries on large tables
--    "Give me all traffic from yesterday"
CREATE INDEX idx_traffic_timestamp_brin
    ON traffic_entries USING BRIN (timestamp)
    WITH (pages_per_range = 32);

-- 6. Instance filtering (multi-instance: "which instance captured this?")
CREATE INDEX idx_traffic_instance
    ON traffic_entries (instance_id)
    WHERE instance_id IS NOT NULL;

-- 7. Method filtering (rare but cheap)
--    No separate index — method is low-cardinality; seq scan with
--    filter is fine for 10k entries. Add only if profiling shows need.
```

**Query patterns and index usage:**

| Query | Current (SQLite) | Optimized (PostgreSQL) |
|---|---|---|
| List traffic for session (newest first) | `WHERE session_id=? ORDER BY timestamp DESC` — uses `idx_requests_session` | Uses `idx_traffic_session_time` (covering) |
| Filter by URL pattern | `WHERE url LIKE '%api%'` — **full scan** | `WHERE url ILIKE '%api%'` — **full scan** (no fix for substring; consider trigram) |
| Filter by host | `WHERE host=?` — no index | Uses `idx_traffic_host` |
| Filter by status code range | `WHERE status_code >= 400` — no index | Uses `idx_traffic_status` |
| Filter by header | `WHERE headers LIKE '%Auth%'` — **full scan** | `WHERE request_headers ? 'Authorization'` — uses GIN |
| Filter by header value | `WHERE headers LIKE '%Bearer%'` — **full scan** | `WHERE request_headers @> '{"Authorization": "Bearer xyz"}'` — uses GIN |
| Time range | `WHERE timestamp > ?` — uses `idx_requests_timestamp` | Uses `idx_traffic_timestamp_brin` (10x smaller) |

**Trigram index for URL/path substring search:**

```sql
-- Enable pg_trgm extension (for ILIKE '%pattern%' optimization)
CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- Trigram GIN index on URL and path (enables fast ILIKE '%pattern%')
CREATE INDEX idx_traffic_url_trgm
    ON traffic_entries USING GIN (url gin_trgm_ops);

CREATE INDEX idx_traffic_path_trgm
    ON traffic_entries USING GIN (path gin_trgm_ops);

-- Now this query uses the trigram index instead of full scan:
-- SELECT * FROM traffic_entries WHERE url ILIKE '%api.example.com%' AND session_id = $1
```

### 6.7 Count maintenance (eliminate COUNT(*))

The current pattern runs `COUNT(*) FROM requests WHERE session_id=?`
on **every insert** to check the `max_entries` limit. At 1000
req/sec, this is 1000 extra queries/sec.

**Solution: Maintain a counter in the sessions table.**

```sql
ALTER TABLE sessions ADD COLUMN entry_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sessions ADD COLUMN total_body_bytes BIGINT NOT NULL DEFAULT 0;

-- Update counters in the same transaction as the batch insert
-- (atomic, no extra round-trip)
UPDATE sessions
SET entry_count = entry_count + $batch_count,
    updated_at = NOW()
WHERE id = $session_id;
```

```rust
// In batch flush: update count in same transaction
async fn flush_batch_with_count(pool: &PgPool, batch: &[TrafficEntry], session_id: Uuid) -> Result<()> {
    let mut tx = pool.begin().await?;

    // 1. Batch insert traffic entries
    sqlx::query(&batch_insert_sql)
        .bind_all(batch)
        .execute(&mut *tx)
        .await?;

    // 2. Update session counter (same transaction, no extra round-trip)
    sqlx::query("UPDATE sessions SET entry_count = entry_count + $1, updated_at = NOW() WHERE id = $2")
        .bind(batch.len() as i64)
        .bind(session_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

// Check max_entries: read from sessions (indexed, O(1))
async fn check_entry_limit(pool: &PgPool, session_id: Uuid, max_entries: i64) -> Result<bool> {
    let count: (i32,) = sqlx::query_as(
        "SELECT entry_count FROM sessions WHERE id = $1"
    )
    .bind(session_id)
    .fetch_one(pool)
    .await?;

    Ok(count.0 as i64 < max_entries)
}
```

**For total body size:** Maintain a `total_body_bytes` counter in
the sessions table, updated in the same transaction as the batch
insert. This eliminates the `SUM(LENGTH(body))` query entirely.

### 6.8 Table partitioning

For high-volume deployments, the `traffic_entries` table should be
partitioned by time range. This enables:
- **Fast pruning** — queries on recent data only scan recent partitions
- **Efficient retention** — drop old partitions instead of `DELETE` + `VACUUM`
- **Parallel query** — PostgreSQL can scan partitions in parallel

```sql
-- Partition by week (good balance for most deployments)
-- Each partition holds ~7 days of traffic
CREATE TABLE traffic_entries (
    id UUID NOT NULL DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    -- ... all other columns ...
) PARTITION BY RANGE (timestamp);

-- Create partitions for the current and next 4 weeks
CREATE TABLE traffic_entries_2026_w01
    PARTITION OF traffic_entries
    FOR VALUES FROM ('2026-01-01') TO ('2026-01-08');

CREATE TABLE traffic_entries_2026_w02
    PARTITION OF traffic_entries
    FOR VALUES FROM ('2026-01-08') TO ('2026-01-15');

-- ... etc ...

-- Default partition for out-of-range data (shouldn't happen, but safety net)
CREATE TABLE traffic_entries_default
    PARTITION OF traffic_entries DEFAULT;
```

**Automated partition management with pg_partman:**

```sql
-- Install pg_partman extension
CREATE EXTENSION IF NOT EXISTS pg_partman;

-- Configure weekly partitioning with 4-week retention
SELECT partman.create_parent(
    'public.traffic_entries',
    'timestamp',
    'weekly',
    '{"p_premake": 4, "p_retention": "4 weeks", "p_retention_schema": "archived"}'
);

-- pg_partman automatically:
-- 1. Creates new partitions 4 weeks ahead
-- 2. Drops partitions older than 4 weeks
-- 3. Can archive old partitions to colder storage instead of dropping
```

**Retention policies:**

| Deployment | Partition granularity | Retention | Storage after retention |
|---|---|---|---|
| Solo / small team | Monthly | 90 days | Drop |
| Medium team | Weekly | 30 days | Drop |
| Large team | Daily | 14 days | Archive to S3 (parquet) |
| Enterprise (compliance) | Daily | 90 days | Archive to S3 (parquet) + Glacier |

### 6.9 Cursor-based pagination

The current `get_traffic` query uses `LIMIT` and `OFFSET` for
pagination. At high offsets (e.g., page 1000 of 100 entries), this
becomes O(offset) — PostgreSQL must scan and discard all prior rows.

**Solution: Cursor-based pagination (keyset pagination).**

```sql
-- Current (slow at high offset):
SELECT * FROM traffic_entries
WHERE session_id = $1
ORDER BY timestamp DESC
LIMIT 50 OFFSET 50000;  -- Scans 50,050 rows, discards 50,000

-- Cursor-based (always fast):
SELECT * FROM traffic_entries
WHERE session_id = $1
  AND (timestamp, id) < ($cursor_timestamp, $cursor_id)  -- keyset
ORDER BY timestamp DESC, id DESC
LIMIT 50;  -- Always scans exactly 50 rows (using idx_traffic_session_time)
```

```rust
// API: GET /api/traffic?cursor=eyJ0IjoiMjAyNi0wMS0wMSIsImlkIjoiYWJjIn0&limit=50

pub struct TrafficCursor {
    timestamp: DateTime<Utc>,
    id: Uuid,
}

impl TrafficCursor {
    /// Encode cursor as base64 JSON (opaque to client)
    pub fn encode(&self) -> String {
        let json = serde_json::json!({
            "t": self.timestamp,
            "i": self.id
        });
        base64::encode(json.to_string())
    }

    /// Decode cursor from base64 JSON
    pub fn decode(s: &str) -> Result<Self> {
        let json: serde_json::Value = serde_json::from_str(&base64::decode(s)?)?;
        Ok(Self {
            timestamp: serde_json::from_value(json["t"].clone())?,
            id: serde_json::from_value(json["i"].clone())?,
        })
    }
}

pub async fn get_traffic_page(
    pool: &PgPool,
    session_id: Uuid,
    cursor: Option<TrafficCursor>,
    limit: i64,
) -> Result<Vec<TrafficEntry>> {
    let rows = match cursor {
        Some(c) => {
            sqlx::query_as::<_, TrafficRow>(
                "SELECT * FROM traffic_entries
                 WHERE session_id = $1
                   AND (timestamp, id) < ($2, $3)
                 ORDER BY timestamp DESC, id DESC
                 LIMIT $4"
            )
            .bind(session_id)
            .bind(c.timestamp)
            .bind(c.id)
            .bind(limit)
            .fetch_all(pool)
            .await?
        }
        None => {
            sqlx::query_as::<_, TrafficRow>(
                "SELECT * FROM traffic_entries
                 WHERE session_id = $1
                 ORDER BY timestamp DESC, id DESC
                 LIMIT $2"
            )
            .bind(session_id)
            .bind(limit)
            .fetch_all(pool)
            .await?
        }
    };

    Ok(rows.into_iter().map(TrafficEntry::from).collect())
}
```

**Performance comparison:**

| Page | OFFSET (current) | Cursor (optimized) |
|---|---|---|
| 1 (50 entries) | 0.5ms | 0.5ms |
| 100 (offset 5000) | 15ms | 0.5ms |
| 1000 (offset 50000) | 150ms | 0.5ms |
| 10000 (offset 500000) | 1500ms | 0.5ms |

Cursor-based pagination is **O(1)** regardless of page depth. OFFSET
is O(n).

### 6.10 Lazy body loading

The current `get_traffic` query loads all columns including bodies.
For a list view of 50 entries, this transfers 50 full bodies from
the database — most of which the user won't look at.

**Solution: Split metadata and body queries.**

```rust
// List view: fetch metadata only (no bodies)
pub async fn get_traffic_list(
    pool: &PgPool,
    session_id: Uuid,
    cursor: Option<TrafficCursor>,
    limit: i64,
) -> Result<Vec<TrafficEntrySummary>> {
    let rows = sqlx::query_as::<_, TrafficSummaryRow>(
        "SELECT id, session_id, method, url, host, path,
                status_code, response_duration_ms, timestamp,
                request_size, response_size, is_passthrough,
                script_intercepted, modified, notes
         FROM traffic_entries
         WHERE session_id = $1
           AND ($2::timestamp IS NULL OR (timestamp, id) < ($2, $3))
         ORDER BY timestamp DESC, id DESC
         LIMIT $4"
    )
    .bind(session_id)
    .bind(cursor.as_ref().map(|c| c.timestamp))
    .bind(cursor.as_ref().map(|c| c.id))
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(TrafficEntrySummary::from).collect())
}

// Detail view: fetch full entry with bodies
pub async fn get_traffic_detail(
    pool: &PgPool,
    id: Uuid,
) -> Result<Option<TrafficEntry>> {
    let row = sqlx::query_as::<_, TrafficRow>(
        "SELECT * FROM traffic_entries WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => {
            let mut entry = TrafficEntry::from(row);

            // If body is in S3, fetch it lazily (only when user opens detail)
            if let Some(ref s3_key) = entry.request_body_s3_key {
                entry.request.body = Some(fetch_body_from_s3(s3_key).await?);
            }
            if let Some(ref s3_key) = entry.response_body_s3_key {
                if let Some(ref mut response) = entry.response {
                    response.body = Some(fetch_body_from_s3(s3_key).await?);
                }
            }

            Ok(Some(entry))
        }
        None => Ok(None),
    }
}
```

**Data transfer comparison (50 entries, avg body 50KB):**

| Approach | Data transferred | Use case |
|---|---|---|
| Current (load all) | ~2.5 MB (50 × 50KB bodies) | List view — wasteful |
| Lazy (metadata only) | ~20 KB (50 × 400B metadata) | List view — 125x less data |
| Lazy + body on demand | ~20 KB + 50KB (one body) | Detail view — loads only when needed |

### 6.11 Connection pooling with PgBouncer

For 10+ proxy instances, each with its own connection pool, the total
connections to PostgreSQL can exceed the server's `max_connections`:

| Instances | Pool per instance | Total connections | PostgreSQL max (default) |
|---|---|---|---|
| 3 | 10 | 30 | 100 — OK |
| 10 | 10 | 100 | 100 — at limit |
| 20 | 10 | 200 | 100 — **exceeded** |
| 50 | 10 | 500 | 100 — **far exceeded** |

**Solution: PgBouncer (external connection pooler).**

```ini
# pgbouncer.ini
[databases]
madhyamas = host=postgres dbname=madhyamas

[pgbouncer]
listen_addr = 0.0.0.0
listen_port = 6432
pool_mode = transaction
max_client_conn = 1000
default_pool_size = 25
reserve_pool_size = 5
reserve_pool_timeout = 3
```

```yaml
# K8s: PgBouncer as a sidecar or separate deployment
apiVersion: apps/v1
kind: Deployment
metadata:
  name: pgbouncer
spec:
  replicas: 2  # HA
  template:
    spec:
      containers:
        - name: pgbouncer
          image: edoburu/pgbouncer:latest
          env:
            - name: DB_HOST
              value: postgres
            - name: DB_NAME
              value: madhyamas
            - name: POOL_MODE
              value: transaction
            - name: MAX_CLIENT_CONN
              value: "1000"
            - name: DEFAULT_POOL_SIZE
              value: "25"
```

With PgBouncer, proxy instances connect to PgBouncer (port 6432)
instead of PostgreSQL directly. PgBouncer multiplexes 1000 client
connections onto 25 server connections. PostgreSQL only sees 25
connections regardless of how many proxy instances are running.

### 6.12 Read replicas

For deployments with many concurrent users viewing traffic, read
queries can saturate the primary database. Read replicas offload
read traffic:

```mermaid
graph TD
    LB["Load Balancer"] --> INST1["Proxy Instance 1"]
    LB --> INST2["Proxy Instance 2"]
    LB --> INST3["Proxy Instance 3"]

    INST1 -->|"writes + reads"| PG_PRIMARY[("PostgreSQL Primary")]
    INST2 -->|"writes + reads"| PG_PRIMARY
    INST3 -->|"writes + reads"| PG_PRIMARY

    PG_PRIMARY -->|"streaming replication"| REPLICA1[("Read Replica 1")]
    PG_PRIMARY -->|"streaming replication"| REPLICA2[("Read Replica 2")]

    INST1 -.->|"traffic list reads"| REPLICA1
    INST2 -.->|"traffic list reads"| REPLICA1
    INST3 -.->|"traffic list reads"| REPLICA2
```

```rust
pub struct DualPool {
    write: PgPool,  // Primary — for inserts, updates, config changes
    read: PgPool,   // Replica — for traffic list, export, analytics
}

impl DualPool {
    pub async fn new(write_url: &str, read_url: &str) -> Result<Self> {
        let write = PgPoolOptions::new()
            .max_connections(5)
            .connect(write_url)
            .await?;

        let read = PgPoolOptions::new()
            .max_connections(10)  // More read connections (read-heavy workload)
            .connect(read_url)
            .await?;

        Ok(Self { write, read })
    }

    pub fn write(&self) -> &PgPool { &self.write }
    pub fn read(&self) -> &PgPool { &self.read }
}

// Usage:
// Write: pool.write().execute("INSERT INTO traffic_entries ...")
// Read:  pool.read().fetch_all("SELECT * FROM traffic_entries WHERE ...")
```

**Replication lag consideration:** Read replicas have a small lag
(typically < 100ms with streaming replication). For traffic list
views, this is acceptable — the user sees entries that are at most
100ms old. For the user's own just-captured entry, the write goes to
the primary and the event is broadcast via Redis (not via the read
replica), so the user sees it immediately via WebSocket.

### 6.13 WebSocket message storage

WebSocket messages can be extremely high-volume (thousands per
second for chatty connections). The current schema stores each
message as a separate row. At scale, this is unsustainable.

**Solution: Batch WebSocket messages into array columns.**

```sql
-- Instead of one row per message:
CREATE TABLE ws_messages (
    id UUID PRIMARY KEY,
    connection_id UUID NOT NULL,
    direction VARCHAR(10) NOT NULL,
    message_type VARCHAR(20) NOT NULL,
    payload_raw BYTEA,
    payload_text TEXT,
    opcode SMALLINT NOT NULL,
    is_final BOOLEAN NOT NULL DEFAULT true,
    mask INTEGER,
    timestamp TIMESTAMPTZ NOT NULL
);

-- Batch messages into one row per connection per minute:
CREATE TABLE ws_message_batches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    connection_id UUID NOT NULL REFERENCES ws_connections(id) ON DELETE CASCADE,
    batch_start TIMESTAMPTZ NOT NULL,
    batch_end TIMESTAMPTZ NOT NULL,
    message_count INTEGER NOT NULL,
    -- Array of messages (JSONB array — compact, queryable)
    messages JSONB NOT NULL,
    -- Aggregated stats
    bytes_sent BIGINT NOT NULL DEFAULT 0,
    bytes_received BIGINT NOT NULL DEFAULT 0
);

CREATE INDEX idx_ws_msg_batch_conn
    ON ws_message_batches (connection_id, batch_start DESC);
```

```rust
// Batch WS messages: flush every 1 second or 100 messages
const WS_BATCH_SIZE: usize = 100;
const WS_BATCH_TIMEOUT: Duration = Duration::from_secs(1);

// Each batch row contains up to 100 messages as a JSONB array
// Reduces row count by 100x and insert round-trips by 100x
```

### 6.14 Vacuum and bloat management

PostgreSQL uses MVCC, which means deleted rows aren't immediately
reclaimed — they're marked dead and cleaned up by `VACUUM`. With
high-volume traffic and FIFO pruning, there's a lot of churn.

| Setting | Default | Recommended | Reason |
|---|---|---|---|
| `autovacuum` | on | on | Must be on |
| `autovacuum_vacuum_scale_factor` | 0.2 | 0.05 | Vacuum when 5% of rows are dead (not 20%) |
| `autovacuum_analyze_scale_factor` | 0.1 | 0.02 | Analyze when 2% of rows change |
| `autovacuum_vacuum_cost_limit` | 200 | 1000 | Allow vacuum to do more work per cycle |
| `autovacuum_naptime` | 60s | 15s | Check more frequently |
| `fillfactor` | 100 | 90 | Leave room for HOT updates (session counters) |

```sql
-- Per-table autovacuum tuning for traffic_entries (high-churn table)
ALTER TABLE traffic_entries SET (
    autovacuum_vacuum_scale_factor = 0.05,
    autovacuum_analyze_scale_factor = 0.02,
    autovacuum_vacuum_cost_limit = 2000,
    fillfactor = 90
);

-- Sessions table (frequent counter updates — needs aggressive vacuum)
ALTER TABLE sessions SET (
    autovacuum_vacuum_scale_factor = 0.01,
    fillfactor = 80  -- More room for HOT updates to entry_count
);
```

**Partitioning helps vacuum:** Instead of vacuuming one 100GB table,
PostgreSQL vacuums 14 weekly partitions of ~7GB each. Each partition
vacuums independently and faster.

### 6.15 Summary: database optimization changes

| # | Optimization | Impact | Effort | Phase |
|---|---|---|---|---|
| 1 | Tiered body storage (inline / TOAST / S3) | **Critical** — prevents table bloat | Medium | DB-1 |
| 2 | Body compression (zstd) | 5-10x storage reduction | Small | DB-1 |
| 3 | Write batching (100 entries / 500ms) | 100x fewer round-trips | Medium | DB-2 |
| 4 | GIN index on headers (JSONB) | Header filter: O(n) → O(log n) | Small | DB-1 |
| 5 | Trigram index on URL/path | Substring search: O(n) → O(log n) | Small | DB-1 |
| 6 | BRIN index on timestamp | 10x smaller than B-tree | Small | DB-1 |
| 7 | Session counter (eliminate COUNT(*)) | Eliminates 1000 queries/sec | Small | DB-2 |
| 8 | Cursor-based pagination | O(1) regardless of page depth | Small | DB-2 |
| 9 | Lazy body loading | 125x less data for list view | Small | DB-2 |
| 10 | Table partitioning (weekly) | Fast pruning, parallel scan | Medium | DB-3 |
| 11 | pg_partman (auto partition + retention) | Automated retention | Small | DB-3 |
| 12 | PgBouncer (external pooler) | Supports 50+ instances | Small | DB-3 |
| 13 | Read replicas | Offload read traffic | Medium | DB-4 |
| 14 | WS message batching | 100x fewer WS message rows | Medium | DB-2 |
| 15 | Autovacuum tuning | Reduced bloat, faster cleanup | Small | DB-1 |
| 16 | S3 body storage for large bodies | Prevents table bloat from 20MB bodies | Medium | DB-1 |

### 6.16 Implementation phases

```mermaid
gantt
    title Database Optimization Phases
    dateFormat YYYY-MM-DD
    axisFormat %b %d

    section DB-1: Schema & indexing
    Tiered body storage            :db1a, 3d
    Body compression (zstd)        :db1b, 2d
    GIN/BRIN/trigram indexes       :db1c, 2d
    Autovacuum tuning              :db1d, 1d
    S3 body storage integration    :db1e, 3d

    section DB-2: Query optimization
    Write batching                 :db2a, after db1a, 3d
    Session counter (no COUNT(*))  :db2b, after db2a, 1d
    Cursor-based pagination        :db2c, after db2b, 2d
    Lazy body loading              :db2d, after db2c, 2d
    WS message batching            :db2e, after db2d, 2d

    section DB-3: Scale
    Table partitioning (weekly)    :db3a, after db2e, 3d
    pg_partman setup               :db3b, after db3a, 1d
    PgBouncer deployment           :db3c, after db3b, 2d

    section DB-4: HA
    Read replicas                  :db4a, after db3c, 3d
    Dual pool (write/read split)   :db4b, after db4a, 2d
```

### 6.17 Expected performance after optimization

| Metric | Before (naive PG port) | After optimization | Improvement |
|---|---|---|---|
| Writes/sec (sustained) | ~2,000 | ~10,000+ | 5x |
| List view latency (50 entries) | ~200ms (with bodies) | ~5ms (metadata only) | 40x |
| List view at page 1000 | ~1500ms (OFFSET) | ~5ms (cursor) | 300x |
| Header filter latency (100k entries) | ~500ms (LIKE scan) | ~10ms (GIN) | 50x |
| URL substring search (100k entries) | ~500ms (LIKE scan) | ~20ms (trigram) | 25x |
| Storage per 100k entries | ~5 GB (uncompressed bodies) | ~500 MB (compressed + tiered) | 10x |
| Vacuum time (100GB table) | ~30 min | ~2 min (per-partition) | 15x |
| PostgreSQL connections (20 instances) | 200 (exceeds max) | 25 (via PgBouncer) | 8x |

---

## 7. Multi-Instance Performance and Security

### 7.1 Multi-instance performance considerations

| Concern | Impact | Solution |
|---|---|---|
| Redis pub/sub latency | +1ms per event | Pipeline publishes; use bincode |
| PostgreSQL write contention | Concurrent writes from N instances | MVCC handles this; use appropriate indexes |
| Config reconciliation | +5ms every 30s per instance | Non-blocking background task |
| Instance heartbeat | +1ms every 10s per instance | Non-blocking background task |
| WebSocket cross-instance | Events from other instances via Redis | Event deduplication by instance_id |
| Load balancer overhead | +1-2ms per request | Use L4 LB (ALB) for lower latency |

### 7.2 Multi-instance security considerations

| Concern | Impact | Solution |
|---|---|---|
| Redis on shared network | Event leak, injection | Redis password + TLS (§3.10) |
| Shared CA private key | Key exposure if volume is compromised | K8s Secret + restricted access |
| Instance impersonation | Fake instance publishes events | Instance ID in events + registry verification |
| Cross-instance session hijacking | JWT valid on all instances | Expected behavior (shared JWT secret) |
| Audit log from multiple instances | Concurrent inserts | PostgreSQL handles; hash chaining (§3.13) |
| Database migration race | Multiple instances migrate simultaneously | Advisory lock (MI doc §10) |
| Instance registry spoofing | Fake instance registers | Heartbeat with shared secret |

### 7.3 Instance authentication

In a multi-instance deployment, instances communicate via Redis and
PostgreSQL. An attacker who gains network access could publish fake
events or register a fake instance.

**Remediation:**

```rust
// Each instance signs its events with a shared cluster key
pub struct SignedEvent {
    pub event: ClusterEvent,
    pub signature: Vec<u8>,  // HMAC-SHA256(event, cluster_key)
}

// On publish
let sig = hmac_sha256(&cluster_key, &serialized_event);
// On receive
let expected_sig = hmac_sha256(&cluster_key, &serialized_event);
if sig != expected_sig {
    warn!("Received unsigned event from {}, dropping", instance_id);
    return;
}
```

The cluster key is stored in a K8s Secret and mounted as an env var.
All instances share the same key. This prevents external attackers
from injecting events, while allowing legitimate instances to
communicate.

---

## 8. Security Checklist

### 8.1 Pre-launch security checklist

| # | Item | Status | Doc reference |
|---|---|---|---|
| 1 | JWT clock skew tolerance (±60s) | TODO | §3.1 |
| 2 | JWT algorithm pinned to HS256 | TODO | §3.2 |
| 3 | CSP headers on proxy web UI | TODO | §3.3 |
| 4 | WebSocket authentication | TODO | §3.6 |
| 5 | Redis password + TLS | TODO | §3.10 |
| 6 | PostgreSQL TLS (`sslmode=require`) | TODO | §3.11 |
| 7 | Audit log hash chaining + append-only | TODO | §3.13 |
| 8 | Password complexity enforcement | TODO | §3.14 |
| 9 | Session idle timeout / refresh token rotation | TODO | §3.15 |
| 10 | API key scopes | TODO | §3.16 |
| 11 | SSRF protection for SSO callbacks | TODO | §3.8 |
| 12 | Rate limiting on login (5/min/IP) | Planned | AUTH_RBAC §1.5 |
| 13 | Account lockout (10 failed attempts) | Planned | LICENSING_SERVER §13 |
| 14 | argon2id password hashing | Planned | OVERVIEW §10.1 |
| 15 | Ed25519 license signing | Planned | LICENSING_SERVER §13 |
| 16 | Stripe webhook signature verification | Planned | LICENSING_SERVER §13 |
| 17 | CORS origin allowlist | Implemented | `madhyamas-api/src/lib.rs` |
| 18 | IP allowlist for proxy | Implemented | `access_control.rs` |
| 19 | No secrets in logs | Planned | AUTH_RBAC §1.5 |
| 20 | Secrets in K8s Secrets / AWS Secrets Manager | Planned | CICD §8 |
| 21 | `cargo audit` in CI (both tiers) | Planned | CICD §4.3 |
| 22 | npm audit in CI | Implemented | `ci.yml` |
| 23 | CodeQL analysis | Implemented | `codeql.yml` |
| 24 | Docker image vulnerability scanning | TODO | §7.2 |
| 25 | Instance event signing (multi-instance) | TODO | §6.3 |

### 8.2 Additional security recommendations

| # | Recommendation | Priority |
|---|---|---|
| 1 | Add `trivy` or `grype` scan to CI for Docker images | High |
| 2 | Add SAST (static analysis) beyond CodeQL — `cargo clippy` security lints | Medium |
| 3 | Add DAST (dynamic analysis) — OWASP ZAP scan in staging | Low |
| 4 | Dependency review on PR (`dependabot` is configured) | Implemented |
| 5 | Penetration test before enterprise launch | High |
| 6 | Bug bounty program (post-launch) | Medium |
| 7 | Security.txt file on `madhyamas.ai` | Low |
| 8 | Responsible disclosure policy | Medium |

---

## 9. Performance Checklist

### 9.1 Pre-launch performance checklist

| # | Item | Status | Doc reference |
|---|---|---|---|
| 1 | PostgreSQL connection pool sizing | TODO | §5.5 |
| 2 | Audit log batch inserts | TODO | §5.3 |
| 3 | Regex pre-compilation on reload | TODO | §5.2 |
| 4 | `Arc<TrafficEvent>` in broadcast channel | TODO | §5.4 |
| 5 | Body compression (zstd) | TODO | §6.4 |
| 6 | Redis bincode serialization (production) | TODO | §5.6 |
| 7 | JWT token cache (short TTL) | TODO | §5.9 |
| 8 | Memory pressure GC (existing) | Implemented | PERFORMANCE.md |
| 9 | Traffic recording limits (max_entries, max_body_size) | Implemented | RECORDING_LIMITS.md |
| 10 | WebSocket polling fallback | Implemented | Web UI |
| 11 | Load testing (k6 / wrk) | TODO | §9.2 |
| 12 | PgBouncer for 10+ instances | TODO | §6.11 |
| 13 | Tiered body storage (inline/TOAST/S3) | TODO | §6.3 |
| 14 | Write batching (100 entries / 500ms) | TODO | §6.5 |
| 15 | GIN index on headers (JSONB) | TODO | §6.6 |
| 16 | Trigram index on URL/path | TODO | §6.6 |
| 17 | Session counter (eliminate COUNT(*)) | TODO | §6.7 |
| 18 | Cursor-based pagination | TODO | §6.9 |
| 19 | Lazy body loading (metadata-only list view) | TODO | §6.10 |
| 20 | Table partitioning (weekly) | TODO | §6.8 |
| 21 | pg_partman (auto partition + retention) | TODO | §6.8 |
| 22 | Read replicas (dual pool) | TODO | §6.12 |
| 23 | WS message batching | TODO | §6.13 |
| 24 | Autovacuum tuning | TODO | §6.14 |

### 9.2 Load testing plan

| Test | Tool | Target | Metric |
|---|---|---|---|
| Proxy throughput | `wrk` through proxy | 1000 req/sec sustained | p99 latency < 100ms |
| API throughput | `k6` against `/api/traffic` | 500 req/sec sustained | p99 latency < 50ms |
| WebSocket fan-out | Custom WS client | 100 concurrent WS clients | Event delivery < 100ms |
| Multi-instance scaling | `k6` against LB | 3 instances, 1000 req/sec | Even distribution, no hotspots |
| PostgreSQL write contention | `pgbench` | 100 concurrent writers | No deadlock, < 10ms write latency |
| PostgreSQL batch insert | Custom benchmark | 100 entries/batch, 500ms | 10k writes/sec sustained |
| PostgreSQL list query (cursor) | `pgbench` custom | 100k entries, page 1000 | < 5ms |
| PostgreSQL list query (OFFSET) | `pgbench` custom | 100k entries, page 1000 | < 150ms (baseline for comparison) |
| PostgreSQL header filter (GIN) | `pgbench` custom | 100k entries, filter by header | < 10ms |
| PostgreSQL URL filter (trigram) | `pgbench` custom | 100k entries, ILIKE '%api%' | < 20ms |
| PostgreSQL vacuum time | Manual | 100GB table (partitioned) | < 2min per partition |
| S3 body upload latency | Custom benchmark | 100KB-20MB bodies | < 100ms for 100KB, < 2s for 20MB |
| Body compression ratio | Custom benchmark | JSON/HTML/binary bodies | 5-10x for text, 1x for images |
| Redis pub/sub latency | Custom benchmark | 1000 events/sec | < 2ms publish-to-receive |
| Memory under load | `valgrind` / `heaptrack` | 10k traffic entries | < 500MB RSS |
| Startup time | Manual | Cold start | < 5s (simple), < 10s (enterprise with PG) |

---

## See Also

- [Enterprise Analysis Overview](ENTERPRISE_OVERVIEW.md) — Master document (risk analysis in §10)
- [Enterprise Licensing Server](ENTERPRISE_LICENSING_SERVER.md) — Security in §13, risk analysis in §17
- [Enterprise Auth, RBAC, and IdP](ENTERPRISE_AUTH_RBAC.md) — Security requirements in §1.5
- [Enterprise Multi-Instance](ENTERPRISE_MULTI_INSTANCE.md) — Multi-instance security in §6
- [Enterprise Storage Traits](ENTERPRISE_STORAGE_TRAITS.md) — Storage trait design
- [Enterprise CI/CD](ENTERPRISE_CICD.md) — Secrets management in §8
- [Enterprise Web UI](ENTERPRISE_WEB_UI.md) — Frontend security (JWT storage, CSP)
- [PERFORMANCE.md](PERFORMANCE.md) — Current performance monitoring implementation
- [PLUGIN_SECURITY.md](PLUGIN_SECURITY.md) — Plugin signing security (same Ed25519 crypto)

# Enterprise Multi-Instance Deployment

> Part of: [Enterprise Analysis Overview](ENTERPRISE_OVERVIEW.md)

This document analyzes the issues that arise when deploying multiple
`madhyamas-enterprise` instances behind a load balancer — including
containerized deployments, context-path/subdomain routing, state
synchronization, and atomic config propagation. Each issue is listed
with its root cause, impact, and proposed solution.

---

## Table of Contents

1. [Deployment Topologies](#1-deployment-topologies)
2. [Docker Image Requirements](#2-docker-image-requirements)
3. [Load Balancer Routing: Context Path and Subdomain](#3-load-balancer-routing-context-path-and-subdomain)
4. [Issue Catalog](#4-issue-catalog)
5. [State Synchronization Strategy](#5-state-synchronization-strategy)
6. [Atomic Config Propagation](#6-atomic-config-propagation)
7. [WebSocket Real-Time Updates Across Instances](#7-websocket-real-time-updates-across-instances)
8. [TLS Certificate Authority Sharing](#8-tls-certificate-authority-sharing)
9. [License Enforcement Across Instances](#9-license-enforcement-across-instances)
10. [Database Migration Coordination](#10-database-migration-coordination)
11. [Health Checks and Graceful Shutdown](#11-health-checks-and-graceful-shutdown)
12. [Kubernetes Deployment Manifest](#12-kubernetes-deployment-manifest)
13. [Implementation Phases](#13-implementation-phases)

---

## 1. Deployment Topologies

### 1.1 Single instance (current)

```mermaid
graph TD
    USER["Developer browser"] -->|"http://host:3001"| PROXY["Madhyamas<br/>(single instance)"]
    PROXY -->|"proxy :8888"| CLIENTS["Client apps"]
    PROXY --> SQLITE[("SQLite<br/>traffic.db")]
```

All state is local: SQLite file on disk, in-memory intercept handlers,
local CA certificate, local WebSocket connections. This works for a
single developer or small team.

### 1.2 Multi-instance behind a load balancer (target)

```mermaid
graph TD
    USER["Team of developers"] --> LB["Load Balancer<br/>(nginx / ALB / Traefik)"]
    LB -->|"round-robin"| INST1["Madhyamas #1"]
    LB -->|"round-robin"| INST2["Madhyamas #2"]
    LB -->|"round-robin"| INST3["Madhyamas #3"]
    INST1 --> PG[("PostgreSQL<br/>(shared)")]
    INST2 --> PG
    INST3 --> PG
    INST1 --> REDIS[("Redis<br/>(pub/sub)")]
    INST2 --> REDIS
    INST3 --> REDIS
    INST1 -->|"proxy :8888"| CLIENTS["Client apps"]
    INST2 -->|"proxy :8888"| CLIENTS
    INST3 -->|"proxy :8888"| CLIENTS
```

Multiple instances share a PostgreSQL database and Redis for
real-time event propagation. The load balancer distributes API and
web UI requests across instances. Proxy traffic (port 8888) can be
directed to a specific instance or also load-balanced.

### 1.3 When multi-instance is needed

| Scenario | Single instance OK? | Why |
|---|---|---|
| Solo developer | Yes | One user, low traffic |
| Small team (2-5 devs) | Yes | Low concurrent API requests |
| Medium team (10-50 devs) | Maybe | API requests scale with users; WebSocket connections per user |
| Large team (50+ devs) | No | API throughput, WebSocket connection limits, fault tolerance |
| High proxy traffic volume | No | Single proxy port is a bottleneck; CPU-bound TLS interception |
| High availability requirement | No | Single instance = single point of failure |
| Geographic distribution | No | Latency; need instances close to users |

---

## 2. Docker Image Requirements

### 2.1 Current Docker image

The current `Dockerfile` produces a single binary with embedded web
UI. It works for both simple and enterprise tiers (via
`BUILD_ENTERPRISE` build arg — see
[ENTERPRISE_CICD.md §6](ENTERPRISE_CICD.md#6-docker-changes)).

### 2.2 What's missing for enterprise multi-instance

| Requirement | Current state | What's needed |
|---|---|---|
| External database support | SQLite only (file path via `MADHYAMAS_DATA_DIR`) | `DATABASE_URL` env var for PostgreSQL connection |
| Redis support | None | `REDIS_URL` env var for pub/sub event bus |
| Config via environment variables | Partial (host, port, log level) | All enterprise config (auth, RBAC, JWT secret, license file) |
| Config via config file | None | `MADHYAMAS_CONFIG_FILE` for YAML/TOML config (mounted as ConfigMap) |
| Health check endpoint | `GET /health` (returns "OK") | `GET /health/detailed` (returns DB connectivity, Redis connectivity, license status) |
| Readiness vs liveness | Single health check | Separate `/health/live` (process alive) and `/health/ready` (DB connected, license valid) |
| Graceful shutdown | None | SIGTERM handler: stop accepting new connections, drain WebSocket, close DB pool |
| Non-root user | Yes (`madhyamas` user) | OK — no change |
| TLS termination | Self-signed CA for proxy interception | TLS for API/web UI terminated at load balancer (pass-through to backend) |
| Signal handling | Default | SIGTERM for graceful shutdown, SIGHUP for config reload |

### 2.3 Enterprise Dockerfile additions

```dockerfile
# Dockerfile — enterprise additions (on top of existing multi-stage build)

# In runtime stage:
ENV MADHYAMAS_DB_BACKEND=postgresql \
    MADHYAMAS_REDIS_URL="" \
    MADHYAMAS_CONFIG_FILE=/etc/madhyamas/config.yaml \
    MADHYAMAS_LICENSE_FILE=/etc/madhyamas/license.json \
    MADHYAMAS_JWT_SECRET_FILE=/etc/madhyamas/jwt-secret \
    MADHYAMAS_CA_CERT_FILE=/etc/madhyamas/ca-cert.pem \
    MADHYAMAS_CA_KEY_FILE=/etc/madhyamas/ca-key.pem

# Mount points for Kubernetes volumes
VOLUME ["/data", "/etc/madhyamas"]

# Health check (detailed — checks DB + Redis + license)
HEALTHCHECK --interval=10s --timeout=5s --start-period=10s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:3001/health/ready || exit 1
```

### 2.4 Docker image tags for enterprise

| Tag | Purpose |
|---|---|
| `ghcr.io/{org}/madhyamas-enterprise:latest` | Latest stable |
| `ghcr.io/{org}/madhyamas-enterprise:v{ver}` | Pinned version |
| `ghcr.io/{org}/madhyamas-enterprise:{major}.{minor}` | Minor track |

---

## 3. Load Balancer Routing: Context Path and Subdomain

### 3.1 The problem

When Madhyamas is deployed behind a load balancer, it may need to be
served from:
- A **subdomain**: `madhyamas.company.internal` (root path)
- A **context path**: `company.internal/madhyamas/` (non-root path)
- A **port**: `company.internal:8080/` (root path, non-standard port)

The current frontend assumes root-path deployment:
- Vite config has no `base` option (defaults to `/`)
- API client uses `const API_BASE = '/api'` (relative, root)
- WebSocket connects to `ws://${window.location.host}/ws` (root)
- Embedded assets reference `/assets/index-*.js` (root)

### 3.2 Impact

| Deployment | Current behavior | Problem |
|---|---|---|
| Subdomain (`madhyamas.company.internal`) | Works | No issue — root path |
| Context path (`company.internal/madhyamas/`) | Broken | Assets load from `/assets/...` instead of `/madhyamas/assets/...` |
| Behind reverse proxy with path stripping | Works if proxy strips path | Fragile — depends on proxy config |

### 3.3 Solution: Configurable base path

#### 3.3.1 Vite build-time base path

For context-path deployments, set Vite's `base` option at build time:

```ts
// web/vite.config.ts
export default defineConfig({
  base: process.env.VITE_BASE_PATH || '/',
  // ... rest unchanged
});
```

```bash
# Build for root path (default — subdomain deployment)
npm run build

# Build for context path
VITE_BASE_PATH=/madhyamas/ npm run build
```

This makes Vite emit assets with the correct prefix:
`/madhyamas/assets/index-*.js` instead of `/assets/index-*.js`.

#### 3.3.2 Runtime base path (preferred)

Build-time base path requires producing different Docker images for
different deployments. A better approach is **runtime base path
injection**:

1. Build the frontend with `base='/'` (default)
2. At runtime, the backend injects a `<base>` tag into `index.html`
   before serving it
3. The API client and WebSocket URL read the base path from a
   `<meta>` tag or `window.__MADHYAMAS_BASE__`

```html
<!-- index.html (served by backend with injected base tag) -->
<base href="/madhyamas/">
<meta name="madhyamas-base-path" content="/madhyamas/">
```

```ts
// web/src/lib/api/client.ts
const BASE_PATH = document.querySelector('meta[name="madhyamas-base-path"]')
  ?.getAttribute('content') || '/';
const API_BASE = `${BASE_PATH.replace(/\/$/, '')}/api`;

// web/src/lib/websocket.ts
const WS_URL = `${window.location.protocol === 'https:' ? 'wss' : 'ws'}://${window.location.host}${BASE_PATH}ws`;
```

```rust
// crates/madhyamas-api/src/embedded_assets.rs — inject base tag at runtime
fn serve_index_html(base_path: &str) -> impl IntoResponse {
    let html = WebAssets::get("index.html").unwrap();
    let mut html = String::from_utf8(html.data.into()).unwrap();

    // Inject <base> tag and meta tag after <head>
    let injection = format!(
        r#"<base href="{}"><meta name="madhyamas-base-path" content="{}">"#,
        base_path, base_path
    );
    html = html.replacen("<head>", &format!("<head>{}", injection), 1);

    Response::builder()
        .header(CONTENT_TYPE, "text/html; charset=utf-8")
        .body(Body::from(html))
        .unwrap()
}
```

The `base_path` is configured via `MADHYAMAS_BASE_PATH` env var
(defaults to `/`). The load balancer routes `/madhyamas/*` to the
backend instances, which serve assets with the correct prefix.

#### 3.3.3 Load balancer configuration examples

**nginx (subdomain):**
```nginx
server {
    listen 443 ssl;
    server_name madhyamas.company.internal;

    ssl_certificate     /etc/ssl/madhyamas.crt;
    ssl_certificate_key /etc/ssl/madhyamas.key;

    location / {
        proxy_pass http://madhyamas-backend;
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $remote_addr;
        proxy_set_header X-Forwarded-Proto $scheme;

        # WebSocket upgrade
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";

        # Sticky session for WebSocket
        proxy_set_header X-Real-IP $remote_addr;
    }
}

upstream madhyamas-backend {
    ip_hash;  # Sticky sessions by client IP
    server madhyamas-1:3001;
    server madhyamas-2:3001;
    server madhyamas-3:3001;
}
```

**nginx (context path):**
```nginx
server {
    listen 443 ssl;
    server_name company.internal;

    location /madhyamas/ {
        proxy_pass http://madhyamas-backend/;  # Note trailing slash — strips /madhyamas prefix
        proxy_set_header Host $host;
        proxy_set_header X-Forwarded-For $remote_addr;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header X-Forwarded-Prefix /madhyamas;

        # WebSocket upgrade
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";

        # Sticky session
        ip_hash;
    }
}

upstream madhyamas-backend {
    ip_hash;
    server madhyamas-1:3001;
    server madhyamas-2:3001;
    server madhyamas-3:3001;
}
```

**AWS ALB (subdomain):**
- Create ALB with HTTPS listener on 443
- Forward rule: `host header = madhyamas.company.internal` → target group
- Target group: madhyamas instances on port 3001
- Sticky sessions: enabled (duration-based, 1 hour cookie)
- WebSocket: ALB supports WebSocket natively (no special config)

**Traefik (context path, Docker labels):**
```yaml
# docker-compose.yml with Traefik
services:
  madhyamas-1:
    image: ghcr.io/org/madhyamas-enterprise:latest
    environment:
      - MADHYAMAS_BASE_PATH=/madhyamas/
    labels:
      - "traefik.enable=true"
      - "traefik.http.routers.madhyamas.rule=PathPrefix(`/madhyamas`)"
      - "traefik.http.routers.madhyamas.middlewares=strip-madhyamas"
      - "traefik.http.middlewares.strip-madhyamas.stripprefix.prefixes=/madhyamas"
      - "traefik.http.services.madhyamas.loadbalancer.sticky.cookie=true"
```

### 3.4 Proxy port routing

The proxy listener (port 8888) is a different concern:

| Topology | Proxy port handling |
|---|---|
| Single instance | Direct access to `:8888` |
| Multi-instance, shared proxy | Load balancer on `:8888` → round-robin to instances |
| Multi-instance, per-instance proxy | Each instance exposes its own `:8888` on a different host port |
| Multi-instance, no proxy | API/web UI only; proxy disabled (enterprise management mode) |

For most enterprise multi-instance deployments, the **proxy port is
not load-balanced** — each developer configures their client to use a
specific instance's proxy port. The load balancer handles only the
API/web UI traffic (port 3001). This avoids the complexity of
cross-instance traffic correlation.

---

## 4. Issue Catalog

### 4.1 Complete list of multi-instance issues

| # | Issue | Severity | Root cause | Affected component |
|---|---|---|---|---|
| 1 | **SQLite is per-instance** | Critical | `traffic.db` is a local file; no shared storage | Traffic store, intercept store, config store, script/plugin persistence |
| 2 | **WebSocket connections are per-instance** | Critical | WS handler holds in-memory `broadcast::Receiver`; no cross-instance pub/sub | Real-time traffic view, live updates |
| 3 | **Intercept rules are per-instance** | Critical | Block list, rewrites, mocks, breakpoints, throttle stored in local SQLite + in-memory `RwLock` | Intercept pipeline |
| 4 | **Breakpoints are per-instance** | High | Breakpoint pause/resume is in-memory; request paused on instance A can't be resumed from instance B | Breakpoint handler |
| 5 | **CA certificate is per-instance** | High | `CertificateManager` generates or loads CA from local disk; each instance has a different CA | TLS interception |
| 6 | **Config changes don't propagate** | High | `PATCH /api/config` updates local SQLite; other instances don't see the change | Config store, capture settings, focus hosts |
| 7 | **Sessions are per-instance** | Medium | Traffic sessions stored in local SQLite; session list differs per instance | Session management |
| 8 | **Audit logs are per-instance** | Medium | Audit events written to local SQLite; no central aggregation | Audit logging |
| 9 | **License seat count is per-instance** | Medium | Each instance verifies its own license independently; no coordination on concurrent seats | License enforcement |
| 10 | **Database migrations race** | Medium | If multiple instances start simultaneously, each may try to run migrations | Database initialization |
| 11 | **Plugin/script state is per-instance** | Medium | Plugin state and script persistence in local SQLite | Plugin system, scripting system |
| 12 | **Rate limiting is per-instance** | Low | Tower Governor state is in-memory per instance | API rate limiting |
| 13 | **WebSocket sticky sessions required** | Low | Without sticky sessions, WS reconnects may land on different instance | Load balancer config |
| 14 | **Health check doesn't verify dependencies** | Low | `GET /health` returns "OK" without checking DB/Redis/license | Health checks |
| 15 | **No graceful shutdown** | Low | No SIGTERM handler; abrupt termination drops WebSocket connections | Container orchestration |
| 16 | **Frontend base path is hardcoded to `/`** | Medium | Vite `base` defaults to `/`; API client uses `/api` | Context-path deployment |
| 17 | **Proxy traffic is per-instance** | Low | Each instance intercepts only traffic flowing through its own proxy port | Traffic correlation |
| 18 | **Metrics are per-instance** | Low | Metrics collector is in-memory | Performance monitoring |

### 4.2 Severity assessment

```mermaid
graph TD
    classDef critical fill:#fee,stroke:#c00,color:#c00
    classDef high fill:#fed,stroke:#c80,color:#c80
    classDef medium fill:#ffd,stroke:#cc0,color:#cc0
    classDef low fill:#dfd,stroke:#0a0,color:#0a0

    I1["#1 SQLite per-instance"]:::critical
    I2["#2 WebSocket per-instance"]:::critical
    I3["#3 Intercept rules per-instance"]:::critical
    I4["#4 Breakpoints per-instance"]:::high
    I5["#5 CA cert per-instance"]:::high
    I6["#6 Config doesn't propagate"]:::high
    I7["#7 Sessions per-instance"]:::medium
    I8["#8 Audit logs per-instance"]:::medium
    I9["#9 License seat count"]:::medium
    I10["#10 Migration race"]:::medium
    I11["#11 Plugin/script state"]:::medium
    I12["#12 Rate limiting"]:::low
    I13["#13 WS sticky sessions"]:::low
    I14["#14 Health check"]:::low
    I15["#15 No graceful shutdown"]:::low
    I16["#16 Base path hardcoded"]:::medium
    I17["#17 Proxy traffic per-instance"]:::low
    I18["#18 Metrics per-instance"]:::low
```

---

## 5. State Synchronization Strategy

### 5.1 The core problem: SQLite → PostgreSQL + Redis

The current architecture stores all state in a local SQLite file.
In a multi-instance deployment, instances must share state. The
solution has two layers:

| Layer | Technology | Purpose |
|---|---|---|
| Persistent shared state | PostgreSQL | Traffic, sessions, intercept rules, config, audit logs, users, RBAC |
| Real-time event bus | Redis Pub/Sub | Cross-instance event propagation (traffic events, config changes, breakpoint notifications) |

```mermaid
graph TD
    subgraph "Instance 1"
        API1["API server"] --> POOL1["PgPool"]
        API1 --> SUB1["Redis subscriber"]
        API1 --> PUB1["Redis publisher"]
    end

    subgraph "Instance 2"
        API2["API server"] --> POOL2["PgPool"]
        API2 --> SUB2["Redis subscriber"]
        API2 --> PUB2["Redis publisher"]
    end

    subgraph "Shared infrastructure"
        PG[("PostgreSQL")]
        REDIS[("Redis")]
    end

    POOL1 --> PG
    POOL2 --> PG
    PUB1 -->|"publish events"| REDIS
    REDIS -->|"subscribe events"| SUB2
    PUB2 -->|"publish events"| REDIS
    REDIS -->|"subscribe events"| SUB1
```

### 5.2 What moves to PostgreSQL

All persistent state that is currently in SQLite moves to
PostgreSQL. This is already planned in
[ENTERPRISE_STORAGE_TRAITS.md](ENTERPRISE_STORAGE_TRAITS.md) — the
storage trait abstraction allows swapping SQLite for PostgreSQL.

| Current SQLite table | PostgreSQL table | Multi-instance behavior |
|---|---|---|
| `sessions` | `sessions` | All instances see all sessions |
| `requests` | `requests` | Each instance writes its own; all can read |
| `responses` | `responses` | Each instance writes its own; all can read |
| `ws_connections` | `ws_connections` | Each instance writes its own; all can read |
| `ws_messages` | `ws_messages` | Each instance writes its own; all can read |
| `focus_hosts` | `focus_hosts` | Shared — all instances see same focus list |
| `mock_rules` | `mock_rules` | Shared — all instances use same mock rules |
| `rewrite_rules` | `rewrite_rules` | Shared — all instances use same rewrite rules |
| `breakpoint_rules` | `breakpoint_rules` | Shared — all instances use same breakpoint rules |
| `throttle_profile` | `throttle_profile` | Shared — all instances use same throttle settings |
| `block_list_entries` | `block_list_entries` | Shared — all instances use same block list |
| `config` | `config` | Shared — all instances read same config |
| `scripts` | `scripts` | Shared — all instances use same scripts |
| `script_executions` | `script_executions` | Each instance writes its own; all can read |
| `plugin_state` | `plugin_state` | Shared — all instances use same plugin state |
| `plugin_invocations` | `plugin_invocations` | Each instance writes its own; all can read |
| Enterprise: `users` | `users` | Shared |
| Enterprise: `roles` | `roles` | Shared |
| Enterprise: `audit_events` | `audit_events` | Shared — central audit log |

### 5.3 What uses Redis Pub/Sub

Real-time events that must propagate across instances use Redis
Pub/Sub:

| Channel | Publisher | Subscriber | Purpose |
|---|---|---|---|
| `traffic_events` | Instance that captured the traffic | All instances | Broadcast new traffic entries to all WebSocket clients |
| `config_changes` | Instance that received the config update | All instances | Reload intercept rules, capture settings, focus hosts from PostgreSQL |
| `breakpoint_events` | Instance that hit a breakpoint | All instances | Notify all WebSocket clients that a breakpoint was hit |
| `session_changes` | Instance that created/switched/deleted a session | All instances | Update session list on all clients |
| `license_events` | Instance that detected license change | All instances | Reload license state |
| `instance_heartbeat` | Each instance (every 10s) | All instances + admin dashboard | Track which instances are alive |

### 5.4 What stays in-memory (per-instance)

| State | Why it stays local |
|---|---|
| Active WebSocket connections | TCP connections are per-instance by nature |
| Regex cache | Performance optimization; rebuilt from PostgreSQL rules |
| In-flight request state | Request being proxied through this instance |
| Active breakpoint pauses | Paused request is on this instance's proxy port |
| Metrics collector | Per-instance metrics; aggregated via Prometheus scraping |
| Rate limiter state | Per-instance rate limiting is acceptable |

---

## 6. Atomic Config Propagation

### 6.1 The problem

When an admin updates a config setting (e.g., adds a block list entry,
changes throttle profile, enables capture), the change must:
1. Be persisted atomically (all-or-nothing)
2. Propagate to all instances
3. Take effect on all instances at the same logical time

Currently, `PATCH /api/config` updates local SQLite and in-memory
state. Other instances never see the change.

### 6.2 Solution: Write-through PostgreSQL + Redis notification

```mermaid
sequenceDiagram
    participant Admin as Admin browser
    participant LB as Load Balancer
    participant I1 as Instance 1
    participant PG as PostgreSQL
    participant Redis as Redis Pub/Sub
    participant I2 as Instance 2
    participant I3 as Instance 3

    Admin->>LB: PATCH /api/config {capture: true}
    LB->>I1: PATCH /api/config {capture: true}

    Note over I1: 1. Write to PostgreSQL (atomic)
    I1->>PG: UPDATE config SET capture = true
    PG-->>I1: OK (committed)

    Note over I1: 2. Publish config change event
    I1->>Redis: PUBLISH config_changes {"key": "capture", "value": true}

    Note over I2,I3: 3. All instances receive event
    Redis-->>I2: config_changes {"key": "capture", "value": true}
    Redis-->>I3: config_changes {"key": "capture", "value": true}
    Redis-->>I1: config_changes (echo)

    Note over I2: 4. Reload from PostgreSQL (authoritative)
    I2->>PG: SELECT * FROM config
    PG-->>I2: {capture: true, ...}
    I2->>I2: Update in-memory state

    Note over I3: 4. Reload from PostgreSQL (authoritative)
    I3->>PG: SELECT * FROM config
    PG-->>I3: {capture: true, ...}
    I3->>I3: Update in-memory state

    I1-->>Admin: 200 OK

    Note over Admin: All instances now have the new config
```

### 6.3 Atomicity guarantee

The write to PostgreSQL is atomic (single transaction). The Redis
pub/sub notification is fire-and-forget — if a subscriber misses it,
that's OK because:

1. **On startup**, each instance loads the full config from
   PostgreSQL (authoritative source).
2. **On config change event**, each instance reloads from PostgreSQL
   (not from the event payload — the event is just a notification).
3. **Periodic reconciliation**: Every 30 seconds, each instance
   compares its in-memory config hash against PostgreSQL. If they
   differ, it reloads. This catches missed pub/sub events.

```rust
// crates/madhyamas-core/src/config/sync.rs

use tokio::sync::watch;

pub struct ConfigSync {
    pg_pool: PgPool,
    redis: redis::Client,
    current_config: watch::Sender<Arc<Config>>,
}

impl ConfigSync {
    /// Called when a config_changes event is received from Redis.
    pub async fn on_config_change_notification(&self) {
        // Reload from PostgreSQL (authoritative)
        let new_config = self.load_config_from_pg().await.unwrap_or_default();
        let _ = self.current_config.send(Arc::new(new_config));
    }

    /// Periodic reconciliation — catches missed pub/sub events.
    pub async fn reconcile_loop(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        loop {
            interval.tick().await;
            let pg_config = self.load_config_from_pg().await.unwrap_or_default();
            let pg_hash = hash_config(&pg_config);
            let current_hash = hash_config(&self.current_config.borrow());

            if pg_hash != current_hash {
                info!("Config drift detected (local={}, pg={}), reloading", current_hash, pg_hash);
                let _ = self.current_config.send(Arc::new(pg_config));
            }
        }
    }
}
```

### 6.4 Intercept rule propagation

Intercept rules (block list, rewrites, mocks, breakpoints, throttle)
follow the same pattern:

| Rule type | PostgreSQL table | In-memory cache | Propagation |
|---|---|---|---|
| Block list | `block_list_entries` | `RwLock<Vec<BlockListEntry>>` | Redis `config_changes` → reload from PG |
| Rewrite rules | `rewrite_rules` | `RwLock<Vec<RewriteRule>>` | Redis `config_changes` → reload from PG |
| Mock rules | `mock_rules` | `RwLock<Vec<MockRule>>` | Redis `config_changes` → reload from PG |
| Breakpoint rules | `breakpoint_rules` | `RwLock<Vec<BreakpointRule>>` | Redis `config_changes` → reload from PG |
| Throttle profile | `throttle_profile` | `RwLock<ThrottleProfile>` | Redis `config_changes` → reload from PG |

The in-memory cache is a **read-through cache**: reads hit the cache
(fast), writes go to PostgreSQL first, then invalidate and reload the
cache.

### 6.5 Timing guarantees

| Operation | Latency |
|---|---|
| Config write to PostgreSQL | < 5ms |
| Redis pub/sub delivery | < 1ms |
| Instance reload from PostgreSQL | < 10ms |
| Total propagation time | < 20ms (typical), < 100ms (worst case) |
| Periodic reconciliation catch-up | < 30s (if pub/sub missed) |

This is **near-atomic**: all instances converge within 100ms. For
true atomicity (all instances see the change at the same instant),
a distributed consensus protocol (Raft/Paxos) would be needed, but
that's overkill for a debugging proxy — near-atomic is sufficient.

---

## 7. WebSocket Real-Time Updates Across Instances

### 7.1 The problem

The current WebSocket handler broadcasts traffic events via an
in-memory `tokio::sync::broadcast` channel. Only clients connected
to the same instance that captured the traffic see the update.

### 7.2 Solution: Redis Pub/Sub bridge

```mermaid
graph TD
    subgraph "Instance 1"
        PROXY1["Proxy engine"] -->|"TrafficEvent"| BC1["broadcast channel<br/>(local)"]
        BC1 -->|"local WS"| WS1["WebSocket clients<br/>on instance 1"]
        BC1 -->|"publish"| REDIS_PUB["Redis publisher"]
    end

    subgraph "Instance 2"
        REDIS_SUB["Redis subscriber"] -->|"TrafficEvent"| BC2["broadcast channel<br/>(local)"]
        BC2 -->|"local WS"| WS2["WebSocket clients<br/>on instance 2"]
    end

    REDIS[("Redis Pub/Sub<br/>channel: traffic_events")]

    REDIS_PUB -->|"PUBLISH"| REDIS
    REDIS -->|"SUBSCRIBE"| REDIS_SUB
```

Each instance:
1. Publishes its local traffic events to Redis (`PUBLISH traffic_events <json>`)
2. Subscribes to Redis (`SUBSCRIBE traffic_events`)
3. Forwards received events to its local WebSocket clients

```rust
// crates/madhyamas-api/src/ws_bridge.rs

use tokio::sync::broadcast;
use redis::Commands;

pub struct WsBridge {
    local_tx: broadcast::Sender<TrafficEvent>,
    redis: redis::Client,
}

impl WsBridge {
    /// Start the Redis → local broadcast bridge.
    /// Received events from other instances are forwarded to local WS clients.
    pub async fn start_redis_to_local(&self) {
        let mut pubsub = self.redis.get_async_pubsub().await.unwrap();
        pubsub.subscribe("traffic_events").await.unwrap();

        loop {
            if let Ok(msg) = pubsub.on_message().next().await {
                if let Ok(event) = serde_json::from_str::<TrafficEvent>(&msg) {
                    let _ = self.local_tx.send(event);
                }
            }
        }
    }

    /// Start the local broadcast → Redis bridge.
    /// Local traffic events are published to Redis for other instances.
    pub async fn start_local_to_redis(&self) {
        let mut rx = self.local_tx.subscribe();
        let mut conn = self.redis.get_async_connection().await.unwrap();

        loop {
            if let Ok(event) = rx.recv().await {
                if let Ok(json) = serde_json::to_string(&event) {
                    let _: () = conn.publish("traffic_events", json).await.unwrap();
                }
            }
        }
    }
}
```

### 7.3 Sticky sessions

WebSocket connections require sticky sessions at the load balancer.
Without sticky sessions, a client reconnecting after a network blip
may land on a different instance, losing its subscription state.

| LB type | Sticky session method |
|---|---|
| nginx | `ip_hash` (by client IP) |
| AWS ALB | Duration-based cookie (1 hour) |
| Traefik | `traefik.http.services.xxx.loadbalancer.sticky.cookie=true` |
| HAProxy | `balance source` + `cookie` |

With the Redis Pub/Sub bridge, sticky sessions are **less critical**
— even if a client lands on a different instance, it will still
receive traffic events from all instances via Redis. However, sticky
sessions reduce unnecessary Redis traffic and avoid duplicate events.

### 7.4 Event deduplication

When an instance publishes a traffic event to Redis, it also receives
its own event back via the subscription. To avoid double-delivery to
local WebSocket clients:

```rust
// Each event includes the originating instance ID
#[derive(Serialize, Deserialize)]
pub struct TrafficEvent {
    pub instance_id: String,  // UUID of the originating instance
    pub event_type: TrafficEventType,
    pub data: TrafficEntry,
}

// In the Redis → local bridge, skip events from self
if event.instance_id == self.instance_id {
    continue;  // Already delivered locally
}
let _ = self.local_tx.send(event);
```

---

## 8. TLS Certificate Authority Sharing

### 8.1 The problem

Each instance generates its own CA certificate for HTTPS interception.
In a multi-instance deployment, this means:
- Instance 1's CA is different from instance 2's CA
- Client apps must trust multiple CAs
- Leaf certificates signed by instance 1 won't validate against
  instance 2's CA
- If a client switches proxy instances (due to LB round-robin), the
  leaf cert presented by the new instance won't match the previously
  cached cert

### 8.2 Solution: Shared CA via mounted volume or PostgreSQL

#### Option A: Shared filesystem volume (simplest)

All instances mount the same volume containing `ca-cert.pem` and
`ca-key.pem`:

```yaml
# Kubernetes: shared CA via PersistentVolumeClaim
volumes:
  - name: ca-storage
    persistentVolumeClaim:
      claimName: madhyamas-ca-pvc
volumeMounts:
  - name: ca-storage
    mountPath: /etc/madhyamas/ca
```

```bash
# Environment variables
MADHYAMAS_CA_CERT_FILE=/etc/madhyamas/ca/ca-cert.pem
MADHYAMAS_CA_KEY_FILE=/etc/madhyamas/ca/ca-key.pem
```

The `CertificateManager` already supports loading an existing CA
from disk (see `certificate.rs` line 59: "Loading existing CA
certificate"). With shared CA files, all instances use the same CA
to sign leaf certificates.

**Leaf certificate cache:** Each instance still maintains its own
in-memory leaf cert cache (by hostname). This is fine — the leaf
certs are signed by the same CA, so clients trust them regardless of
which instance generated them.

#### Option B: CA in PostgreSQL (for auto-provisioning)

Store the CA cert and key in PostgreSQL (encrypted at rest). On
first startup, if no CA exists in the database, one instance
generates it and stores it. Other instances load it from the
database.

```sql
CREATE TABLE ca_certificates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    cert_pem TEXT NOT NULL,
    key_pem_encrypted BYTEA NOT NULL,  -- encrypted with MADHYAMAS_CA_KEY
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    rotation_due_at TIMESTAMPTZ  -- for key rotation
);
```

Use `SELECT ... FOR UPDATE` to ensure only one instance generates the
CA on first startup:

```rust
// Atomically claim CA generation
let row: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM ca_certificates LIMIT 1 FOR UPDATE")
    .fetch_optional(&pool)
    .await?;

if row.is_none() {
    // This instance generates the CA
    let (cert, key) = generate_ca()?;
    sqlx::query("INSERT INTO ca_certificates (cert_pem, key_pem_encrypted) VALUES ($1, $2)")
        .bind(cert)
        .bind(encrypt(key, &ca_encryption_key))
        .execute(&pool)
        .await?;
}
```

**Recommendation:** Option A (shared volume) for simplicity. Option B
only if you need automatic CA provisioning without manual setup.

### 8.3 CA rotation

When the CA needs rotation (e.g., key compromise, expiry):

1. Generate new CA cert + key
2. Store in the shared volume (Option A) or PostgreSQL (Option B)
3. Publish `ca_rotated` event via Redis
4. All instances reload the CA
5. Old leaf certs are evicted from in-memory caches
6. Clients see new leaf certs signed by the new CA
7. Clients must re-trust the new CA (distribute via MDM/Group Policy)

---

## 9. License Enforcement Across Instances

### 9.1 The problem

Each instance verifies its own license file independently. If the
license allows 50 seats (concurrent users), and 3 instances are
running, there's no coordination — 150 users could potentially
connect (50 per instance).

### 9.2 Solution: Centralized seat tracking in PostgreSQL

```sql
-- Track active sessions (users currently connected)
CREATE TABLE active_sessions (
    session_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id),
    instance_id UUID NOT NULL,  -- which instance the user is connected to
    connected_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_heartbeat_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL  -- heartbeat + 5 minutes
);

CREATE INDEX idx_active_sessions_user ON active_sessions(user_id);
CREATE INDEX idx_active_sessions_instance ON active_sessions(instance_id);
CREATE INDEX idx_active_sessions_expires ON active_sessions(expires_at);
```

### 9.3 Seat counting

```rust
pub async fn count_active_seats(pool: &PgPool) -> Result<i64> {
    // Count distinct users with non-expired sessions
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(DISTINCT user_id) FROM active_sessions WHERE expires_at > NOW()"
    )
    .fetch_one(pool)
    .await?;
    Ok(count.0)
}

pub async fn check_seat_available(pool: &PgPool, max_seats: i64) -> Result<bool> {
    let current = count_active_seats(pool).await?;
    Ok(current < max_seats)
}
```

### 9.4 Session lifecycle

```mermaid
sequenceDiagram
    participant User
    participant LB as Load Balancer
    participant Instance
    participant PG as PostgreSQL

    User->>LB: Login (JWT)
    LB->>Instance: POST /auth/login
    Instance->>PG: INSERT INTO active_sessions (user_id, instance_id, expires_at=NOW()+5min)
    Instance-->>User: JWT token

    Note over User,Instance: Heartbeat every 60 seconds
    User->>LB: GET /api/health/detailed (with JWT)
    LB->>Instance: GET /api/health/detailed
    Instance->>PG: UPDATE active_sessions SET last_heartbeat_at=NOW(), expires_at=NOW()+5min WHERE user_id=$1
    Instance-->>User: 200 OK

    Note over PG: Background job: DELETE expired sessions
    PG->>PG: DELETE FROM active_sessions WHERE expires_at < NOW()

    Note over User: User logs out or disconnects
    User->>LB: POST /auth/logout
    LB->>Instance: POST /auth/logout
    Instance->>PG: DELETE FROM active_sessions WHERE user_id=$1 AND instance_id=$2
```

### 9.5 Grace period

If an instance crashes, its sessions won't be explicitly deleted.
The `expires_at` column (heartbeat + 5 minutes) ensures stale
sessions are automatically cleaned up. A background job on one
instance (elected via PostgreSQL advisory lock) runs every minute:

```sql
DELETE FROM active_sessions WHERE expires_at < NOW();
```

### 9.6 Over-seat handling

When the seat limit is reached:

1. New login attempts receive `403 Forbidden` with
   `{"error": "seat_limit_reached", "max_seats": 50, "current_seats": 50}`
2. The web UI shows a "All seats in use" message
3. Admins can force-disconnect a user via `DELETE /api/admin/sessions/{id}`
4. The licensing server is notified (for analytics, not enforcement)

---

## 10. Database Migration Coordination

### 10.1 The problem

When multiple instances start simultaneously (e.g., after a rolling
deploy), each may try to run database migrations. This can cause:
- Race conditions (two instances creating the same table)
- Partial migrations (one instance fails mid-migration)
- Schema version mismatches

### 10.2 Solution: Advisory lock + single migrator

Use PostgreSQL advisory lock to ensure only one instance runs
migrations at a time:

```rust
pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    // Acquire advisory lock (session-level, auto-released on disconnect)
    sqlx::query("SELECT pg_advisory_lock(72727272)")  // magic number for madhyamas
        .execute(pool)
        .await?;

    // Run migrations
    sqlx::migrate!("./migrations")
        .run(pool)
        .await?;

    // Release lock
    sqlx::query("SELECT pg_advisory_unlock(72727272)")
        .execute(pool)
        .await?;

    Ok(())
}
```

Other instances wait for the lock to be released, then verify the
schema version matches:

```rust
// After lock is released (another instance finished migrations)
let version: (i64,) = sqlx::query_as("SELECT version FROM _sqlx_migrations ORDER BY version DESC LIMIT 1")
    .fetch_one(pool)
    .await?;

if version.0 < EXPECTED_MIGRATION_VERSION {
    return Err("Database schema is older than expected".into());
}
```

### 10.3 Kubernetes init container

For Kubernetes deployments, run migrations as an init container
before the main containers start:

```yaml
initContainers:
  - name: migrator
    image: ghcr.io/org/madhyamas-enterprise:latest
    command: ["madhyamas", "migrate"]
    env:
      - name: DATABASE_URL
        valueFrom:
          secretKeyRef:
            name: madhyamas-secrets
            key: database-url
```

The `madhyamas migrate` subcommand runs migrations and exits. The
main containers start only after migrations are complete.

---

## 11. Health Checks and Graceful Shutdown

### 11.1 Health check endpoints

| Endpoint | Purpose | LB use | Checks |
|---|---|---|---|
| `GET /health/live` | Liveness — process is alive | Restart pod if failing | None (always 200 if process is running) |
| `GET /health/ready` | Readiness — can serve requests | Remove from LB if failing | DB connection, Redis connection, license valid |
| `GET /health/detailed` | Detailed status | Admin dashboard | DB, Redis, license, instance ID, version, uptime, active connections |

```rust
async fn health_ready(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let mut checks = Vec::new();

    // Check PostgreSQL
    match sqlx::query("SELECT 1").execute(&state.pg_pool).await {
        Ok(_) => checks.push(("database", "ok")),
        Err(_) => checks.push(("database", "error")),
    }

    // Check Redis
    match state.redis.get_async_connection().await {
        Ok(_) => checks.push(("redis", "ok")),
        Err(_) => checks.push(("redis", "error")),
    }

    // Check license
    match &state.license {
        Some(lic) if lic.is_valid() => checks.push(("license", "ok")),
        Some(_) => checks.push(("license", "expired")),
        None => checks.push(("license", "missing")),
    }

    let all_ok = checks.iter().all(|(_, s)| *s == "ok");
    let status = if all_ok { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };

    (status, Json(json!({ "checks": checks.iter().collect::<Vec<_>>() })))
}
```

### 11.2 Graceful shutdown

```rust
pub async fn shutdown_signal(app_state: AppState) {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received, draining connections...");

    // 1. Stop accepting new connections (axum handle shutdown)
    // 2. Drain WebSocket connections (send close frame)
    // 3. Wait for in-flight proxy requests to complete (max 30s)
    // 4. Remove self from instance registry (DELETE FROM active_instances)
    // 5. Close PostgreSQL pool
    // 6. Close Redis connection
    // 7. Exit

    info!("All connections drained, shutting down");
}
```

### 11.3 Kubernetes lifecycle

```yaml
lifecycle:
  preStop:
    exec:
      command: ["sh", "-c", "kill -TERM 1"]  # Send SIGTERM to process

terminationGracePeriodSeconds: 30  # Wait up to 30s for graceful shutdown
```

---

## 12. Kubernetes Deployment Manifest

### 12.1 Full deployment manifest

```yaml
# k8s/madhyamas-enterprise.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: madhyamas-config
data:
  config.yaml: |
    proxy:
      port: 8888
      https: true
    api:
      port: 3001
      base_path: /madhyamas/
    auth:
      mode: jwt
      required: true
      jwt_secret_file: /etc/madhyamas/jwt-secret
    database:
      backend: postgresql
    redis:
      url: redis://madhyamas-redis:6379
    license:
      file: /etc/madhyamas/license.json
    ca:
      cert_file: /etc/madhyamas/ca/ca-cert.pem
      key_file: /etc/madhyamas/ca/ca-key.pem
---
apiVersion: v1
kind: Secret
metadata:
  name: madhyamas-secrets
type: Opaque
stringData:
  database-url: postgres://madhyamas:password@madhyamas-pg:5432/madhyamas
  jwt-secret: <base64-encoded-secret>
---
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: madhyamas-ca-pvc
spec:
  accessModes:
    - ReadWriteMany  # Multiple pods can read simultaneously
  resources:
    requests:
      storage: 100Mi
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: madhyamas-enterprise
spec:
  replicas: 3
  selector:
    matchLabels:
      app: madhyamas-enterprise
  template:
    metadata:
      labels:
        app: madhyamas-enterprise
    spec:
      initContainers:
        - name: migrator
          image: ghcr.io/org/madhyamas-enterprise:latest
          command: ["madhyamas", "migrate"]
          env:
            - name: DATABASE_URL
              valueFrom:
                secretKeyRef:
                  name: madhyamas-secrets
                  key: database-url
      containers:
        - name: madhyamas
          image: ghcr.io/org/madhyamas-enterprise:latest
          ports:
            - containerPort: 3001
              name: api
            - containerPort: 8888
              name: proxy
          env:
            - name: MADHYAMAS_CONFIG_FILE
              value: /etc/madhyamas/config.yaml
            - name: DATABASE_URL
              valueFrom:
                secretKeyRef:
                  name: madhyamas-secrets
                  key: database-url
            - name: MADHYAMAS_REDIS_URL
              value: redis://madhyamas-redis:6379
            - name: MADHYAMAS_LICENSE_FILE
              value: /etc/madhyamas/license.json
            - name: MADHYAMAS_JWT_SECRET_FILE
              value: /etc/madhyamas/jwt-secret
            - name: MADHYAMAS_BASE_PATH
              value: /madhyamas/
            - name: MADHYAMAS_CA_CERT_FILE
              value: /etc/madhyamas/ca/ca-cert.pem
            - name: MADHYAMAS_CA_KEY_FILE
              value: /etc/madhyamas/ca/ca-key.pem
            - name: MADHYAMAS_INSTANCE_ID
              valueFrom:
                fieldRef:
                  fieldPath: metadata.uid
          volumeMounts:
            - name: config
              mountPath: /etc/madhyamas/config.yaml
              subPath: config.yaml
            - name: secrets
              mountPath: /etc/madhyamas/jwt-secret
              subPath: jwt-secret
            - name: license
              mountPath: /etc/madhyamas/license.json
              subPath: license.json
            - name: ca-storage
              mountPath: /etc/madhyamas/ca
            - name: data
              mountPath: /data
          readinessProbe:
            httpGet:
              path: /madhyamas/health/ready
              port: 3001
            initialDelaySeconds: 10
            periodSeconds: 5
          livenessProbe:
            httpGet:
              path: /madhyamas/health/live
              port: 3001
            initialDelaySeconds: 15
            periodSeconds: 10
          lifecycle:
            preStop:
              exec:
                command: ["sh", "-c", "kill -TERM 1"]
          terminationGracePeriodSeconds: 30
          resources:
            requests:
              cpu: 500m
              memory: 512Mi
            limits:
              cpu: 2000m
              memory: 2Gi
      volumes:
        - name: config
          configMap:
            name: madhyamas-config
        - name: secrets
          secret:
            secretName: madhyamas-secrets
        - name: license
          secret:
            secretName: madhyamas-license
        - name: ca-storage
          persistentVolumeClaim:
            claimName: madhyamas-ca-pvc
        - name: data
          emptyDir: {}
---
apiVersion: v1
kind: Service
metadata:
  name: madhyamas-enterprise
spec:
  selector:
    app: madhyamas-enterprise
  ports:
    - name: api
      port: 3001
      targetPort: 3001
    - name: proxy
      port: 8888
      targetPort: 8888
---
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: madhyamas-enterprise
  annotations:
    nginx.ingress.kubernetes.io/rewrite-target: /$2
    nginx.ingress.kubernetes.io/proxy-body-size: "100m"
    nginx.ingress.kubernetes.io/proxy-read-timeout: "3600"
    nginx.ingress.kubernetes.io/proxy-send-timeout: "3600"
    nginx.ingress.kubernetes.io/upstream-hash-by: "$remote_addr"
    nginx.ingress.kubernetes.io/configuration-snippet: |
      proxy_set_header Upgrade $http_upgrade;
      proxy_set_header Connection "upgrade";
spec:
  tls:
    - hosts:
        - company.internal
      secretName: madhyamas-tls
  rules:
    - host: company.internal
      http:
        paths:
          - path: /madhyamas(/|$)(.*)
            pathType: Prefix
            backend:
              service:
                name: madhyamas-enterprise
                port:
                  number: 3001
```

### 12.2 Redis and PostgreSQL (managed or self-hosted)

```yaml
# Redis (simple single-instance — for production use Redis Sentinel/Cluster)
apiVersion: apps/v1
kind: Deployment
metadata:
  name: madhyamas-redis
spec:
  replicas: 1
  selector:
    matchLabels:
      app: madhyamas-redis
  template:
    metadata:
      labels:
        app: madhyamas-redis
    spec:
      containers:
        - name: redis
          image: redis:7-alpine
          ports:
            - containerPort: 6379
          resources:
            requests:
              cpu: 100m
              memory: 128Mi
            limits:
              cpu: 500m
              memory: 512Mi
---
apiVersion: v1
kind: Service
metadata:
  name: madhyamas-redis
spec:
  selector:
    app: madhyamas-redis
  ports:
    - port: 6379
      targetPort: 6379
```

For PostgreSQL, use a managed service (AWS RDS, Google Cloud SQL,
Azure Database) or the PostgreSQL Helm chart.

---

## 13. Implementation Phases

### Phase MI-1: PostgreSQL migration (prerequisite)

**Prerequisite:** Phase 0 (crate extraction) + storage trait migration
([ENTERPRISE_STORAGE_TRAITS.md](ENTERPRISE_STORAGE_TRAITS.md))

| Task | Component |
|---|---|
| Implement PostgreSQL backend for all stores | `madhyamas-enterprise` |
| Add `DATABASE_URL` env var support | `madhyamas-enterprise` |
| Add `madhyamas migrate` subcommand | `madhyamas-cli` |
| Test all stores against PostgreSQL | Tests |

**Effort:** Large. This is the foundation for multi-instance.

### Phase MI-2: Redis Pub/Sub integration

| Task | Component |
|---|---|
| Add Redis dependency (`redis` crate with tokio support) | `madhyamas-enterprise` |
| Implement `WsBridge` (Redis ↔ local broadcast) | `madhyamas-api` |
| Implement `ConfigSync` (Redis notification → PG reload) | `madhyamas-enterprise` |
| Add `REDIS_URL` env var support | `madhyamas-enterprise` |
| Add instance ID generation (UUID per instance) | `madhyamas-enterprise` |
| Event deduplication (skip self-originated events) | `madhyamas-api` |

**Effort:** Medium. Redis pub/sub is straightforward.

### Phase MI-3: Config propagation

| Task | Component |
|---|---|
| Write-through pattern: API → PostgreSQL → Redis publish | `madhyamas-enterprise` |
| Subscribe pattern: Redis → reload from PostgreSQL | `madhyamas-enterprise` |
| Periodic reconciliation loop (30s) | `madhyamas-enterprise` |
| Intercept rule reload on config change event | `madhyamas-core` |

**Effort:** Medium.

### Phase MI-4: Shared CA

| Task | Component |
|---|---|
| Support `MADHYAMAS_CA_CERT_FILE` / `MADHYAMAS_CA_KEY_FILE` env vars | `madhyamas-core` |
| Load CA from shared volume on startup | `madhyamas-core` |
| CA rotation via Redis event | `madhyamas-core` |

**Effort:** Small. The CA loading code already exists.

### Phase MI-5: License seat tracking

| Task | Component |
|---|---|
| `active_sessions` table in PostgreSQL | `madhyamas-enterprise` |
| Session registration on login | `madhyamas-enterprise` |
| Heartbeat updates (60s) | `madhyamas-enterprise` |
| Background job: delete expired sessions | `madhyamas-enterprise` |
| Seat limit enforcement on login | `madhyamas-enterprise` |
| Admin endpoint: list/disconnect sessions | `madhyamas-api` |

**Effort:** Medium.

### Phase MI-6: Health checks and graceful shutdown

| Task | Component |
|---|---|
| `GET /health/live` endpoint | `madhyamas-api` |
| `GET /health/ready` endpoint (checks DB, Redis, license) | `madhyamas-api` |
| SIGTERM handler (drain connections, close pools) | `madhyamas` |
| `madhyamas migrate` subcommand | `madhyamas-cli` |
| Advisory lock for migration coordination | `madhyamas-enterprise` |

**Effort:** Small.

### Phase MI-7: Frontend base path support

| Task | Component |
|---|---|
| Runtime base path injection in `index.html` | `madhyamas-api` |
| `MADHYAMAS_BASE_PATH` env var | `madhyamas-api` |
| Frontend: read base path from `<meta>` tag | `web/src/lib/` |
| Frontend: API client uses base path | `web/src/lib/api/client.ts` |
| Frontend: WebSocket URL uses base path | `web/src/lib/` |

**Effort:** Small.

### Phase MI-8: Kubernetes manifests and documentation

| Task | Component |
|---|---|
| Kubernetes deployment manifest | `k8s/madhyamas-enterprise.yaml` |
| Redis deployment manifest | `k8s/` |
| Ingress manifest (context path + WebSocket) | `k8s/` |
| Docker Compose for local multi-instance testing | `docker/docker-compose.multi.yml` |
| Admin guide: multi-instance deployment | `docs-site/` |

**Effort:** Medium.

### Roadmap

```mermaid
gantt
    title Multi-Instance Implementation Phases
    dateFormat YYYY-MM-DD
    axisFormat %b %d

    section Foundation
    Phase MI-1: PostgreSQL migration     :mi1, after p0, 14d
    Phase MI-2: Redis pub/sub            :mi2, after mi1, 7d

    section Config and state
    Phase MI-3: Config propagation       :mi3, after mi2, 5d
    Phase MI-4: Shared CA                :mi4, after mi1, 3d
    Phase MI-5: License seat tracking    :mi5, after mi2, 5d

    section Operations
    Phase MI-6: Health + shutdown        :mi6, after mi2, 3d
    Phase MI-7: Frontend base path       :mi7, 3d
    Phase MI-8: K8s manifests            :mi8, after mi6, 5d
```

Phase MI-1 is the largest and is a prerequisite for most other phases.
Phase MI-7 (frontend base path) is independent and can start
immediately. Phases MI-3 through MI-6 depend on MI-1 and MI-2.

---

## See Also

- [Enterprise Analysis Overview](ENTERPRISE_OVERVIEW.md) — Master document
- [Enterprise Storage Traits](ENTERPRISE_STORAGE_TRAITS.md) — PostgreSQL migration foundation
- [Enterprise CI/CD](ENTERPRISE_CICD.md) — Docker image build for enterprise tier
- [Enterprise Web UI](ENTERPRISE_WEB_UI.md) — Frontend design (base path affects this)
- [Enterprise Licensing Server](ENTERPRISE_LICENSING_SERVER.md) — License issuance and seat tracking
- [Enterprise Auth, RBAC, and IdP](ENTERPRISE_AUTH_RBAC.md) — Auth design (session tracking)
- [Enterprise Performance & Security](ENTERPRISE_PERF_SECURITY.md) — Multi-instance security (Redis auth/TLS, instance event signing) in §6
- [PERSISTENCE.md](PERSISTENCE.md) — Current SQLite schema (per-instance, pre-migration)
- [PERFORMANCE.md](PERFORMANCE.md) — Metrics (per-instance, needs Prometheus aggregation)

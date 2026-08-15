---
title: Multi-Instance Deployment
description: Deploy Madhyamas Enterprise with multiple instances behind a load balancer using PostgreSQL, Redis, shared CA, and Docker Compose or Kubernetes.
---

# Multi-Instance Deployment

Madhyamas Enterprise supports horizontal scaling with multiple instances behind a load balancer. All instances share a single PostgreSQL database and Redis pub/sub bus, ensuring consistent state across the cluster.

## Architecture

```
                    ┌──────────────────┐
                    │  Load Balancer   │
                    │  (nginx / ALB)   │
                    └────────┬─────────┘
                             │
              ┌──────────────┼──────────────┐
              ▼              ▼              ▼
    ┌─────────────┐  ┌─────────────┐  ┌─────────────┐
    │ Instance 1  │  │ Instance 2  │  │ Instance N  │
    │ API :3001   │  │ API :3001   │  │ API :3001   │
    │ Proxy :8888 │  │ Proxy :8888 │  │ Proxy :8888 │
    └──────┬──────┘  └──────┬──────┘  └──────┬──────┘
           │                │                │
           └────────┬───────┴────────┬───────┘
                    ▼                ▼
           ┌────────────────┐ ┌──────────────┐
           │  PostgreSQL    │ │    Redis     │
           │  (shared DB)   │ │  (pub/sub)   │
           └────────────────┘ └──────────────┘
```

## Quick Start: Docker Compose

The fastest way to deploy multi-instance is with the included Docker Compose file:

```bash
git clone https://github.com/ShristiLabs/madhyamas.git
cd madhyamas
./startup-local.sh --tier enterprise
```

### Services

| Service | Port | Description |
|---------|------|-------------|
| nginx (load balancer) | `14000` (API), `8888` (proxy) | Round-robin routing with sticky WebSocket |
| madhyamas-1 | `14001` | Instance 1 (API + proxy) |
| madhyamas-2 | `14002` | Instance 2 (API + proxy) |
| PostgreSQL | `15432` | Shared database |
| Redis | `16379` | Pub/sub + seat coordination |

### Default Credentials

- **Admin user**: `admin`
- **Admin password**: `testpass123`
- **JWT secret**: `multi-instance-dev-secret`
- **PostgreSQL**: `madhyamas:madhyamas@localhost:15432/madhyamas`
- **Redis**: `localhost:16379`

::: warning Change defaults
These credentials are for development only. Override them with environment variables in production.
:::

### Stopping

```bash
./stop-local.sh --tier enterprise
# or
docker compose -f docker/docker-compose.multi.yml down
```

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `MADHYAMAS_DATABASE_URL` | — | PostgreSQL connection URL (required) |
| `MADHYAMAS_REDIS_URL` | — | Redis connection URL (required for multi-instance) |
| `MADHYAMAS_ENABLE_AUTH` | `true` | Enable authentication |
| `MADHYAMAS_JWT_SECRET` | — | JWT signing secret (must be same across instances) |
| `MADHYAMAS_ADMIN_USERNAME` | `admin` | Bootstrap admin username |
| `MADHYAMAS_ADMIN_PASSWORD` | — | Bootstrap admin password |
| `MADHYAMAS_LICENSE_FILE` | — | Path to license file |
| `MADHYAMAS_CA_CERT_FILE` | `/data/certs/ca-cert.pem` | Shared CA certificate path |
| `MADHYAMAS_CA_KEY_FILE` | `/data/certs/ca-key.pem` | Shared CA private key path |
| `MADHYAMAS_INSTANCE_ID` | auto-generated | Unique instance identifier |
| `MADHYAMAS_BASE_PATH` | `/` | Base path for context-path routing |

### Critical: Shared Values

These values **must be identical** across all instances:

- `MADHYAMAS_JWT_SECRET` — Otherwise tokens from one instance are rejected by another
- `MADHYAMAS_DATABASE_URL` — All instances share the same database
- `MADHYAMAS_REDIS_URL` — All instances connect to the same Redis
- `MADHYAMAS_CA_CERT_FILE` / `MADHYAMAS_CA_KEY_FILE` — All instances use the same TLS CA

## Shared CA Certificate

All instances must use the same TLS Certificate Authority for HTTPS interception. The Docker Compose configuration uses a shared volume:

```yaml
volumes:
  ca_certs:  # shared volume

services:
  madhyamas-1:
    volumes:
      - ca_certs:/data/certs
    environment:
      MADHYAMAS_CA_CERT_FILE: /data/certs/ca-cert.pem
      MADHYAMAS_CA_KEY_FILE: /data/certs/ca-key.pem
```

The first instance to start generates the CA and writes it to the shared volume. Subsequent instances find the existing CA and load it.

### Manual CA Generation

```bash
openssl req -x509 -newkey rsa:2048 \
  -keyout ca-key.pem -out ca-cert.pem \
  -days 365 -nodes \
  -subj "/CN=Madhyamas Shared CA"

# Distribute ca-cert.pem and ca-key.pem to all instances
```

## Load Balancer Configuration

### nginx

The included nginx configuration (`docker/nginx-multi.conf`) uses:

- **Round-robin** for API and proxy requests
- **`ip_hash`** for WebSocket connections (sticky sessions)
- **Health checks** against `/health`

```nginx
upstream madhyamas_api {
    server madhyamas-1:3001;
    server madhyamas-2:3001;
}

upstream madhyamas_ws {
    ip_hash;  # Sticky sessions for WebSocket
    server madhyamas-1:3001;
    server madhyamas-2:3001;
}

server {
    listen 14000;
    location / {
        proxy_pass http://madhyamas_api;
    }
    location /ws {
        proxy_pass http://madhyamas_ws;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```

### Cloud Load Balancers

| Provider | Configuration |
|----------|--------------|
| **AWS ALB** | Sticky sessions enabled for WebSocket target group |
| **GCP LB** | Session affinity = CLIENT_IP for WebSocket backend |
| **Traefik** | `sticky=true` for WebSocket router |
| **HAProxy** | `balance source` for WebSocket backend |

### Context-Path Routing

To serve Madhyamas under a non-root path (e.g., `/madhyamas/`):

```bash
madhyamas --base-path /madhyamas
```

The web UI and API will be served at `https://yourcompany.com/madhyamas/`.

## Redis Pub/Sub Channels

Redis is used for cross-instance communication:

| Channel | Purpose |
|---------|---------|
| `madhyamas:events` | WebSocket traffic events (request/response captured) |
| `madhyamas:config` | Configuration changes |
| `madhyamas:intercept` | Intercept rule changes (mocks, rewrites, breakpoints, throttle) |
| `madhyamas:seats` | License seat coordination |

When an instance captures traffic or changes config, it publishes to the appropriate channel. All other instances subscribe and apply the changes locally.

::: tip Event deduplication
Traffic events include a unique event ID. Subscribers deduplicate by ID to prevent infinite event loops (a bug that was discovered and fixed during multi-instance testing).
:::

## Race Condition Safety

Multi-instance deployments face race conditions that single-instance deployments don't. Madhyamas Enterprise addresses these:

| Race Condition | Fix |
|----------------|-----|
| Concurrent DDL (CREATE EXTENSION/TABLE) | PostgreSQL advisory lock (`pg_advisory_xact_lock`) |
| Concurrent admin user bootstrap | `ON CONFLICT (username) DO NOTHING` |
| Double-prune of entry limits | Atomic `DELETE ... RETURNING` in advisory-locked transaction |
| Audit hash chain breakage | Advisory-locked insertion sequence |
| License seat over-registration | Redis Lua script for atomic `ZADD + EXPIRE` |
| Focus host duplicate patterns | `ON CONFLICT (pattern) DO NOTHING` |
| Session desync across instances | Shared `instance_state` table + periodic sync |
| Health check before DB ready | `/health` endpoint pings database before reporting OK |

## Kubernetes Deployment

### Manifests

Kubernetes manifests are documented in `docs/ENTERPRISE_MULTI_INSTANCE.md`. Key resources:

| Resource | Purpose |
|----------|---------|
| Deployment | Madhyamas instances with rolling updates |
| Service | Cluster IP for internal access |
| Ingress | External access with TLS termination |
| ConfigMap | Non-secret configuration (database URL, Redis URL) |
| Secret | Sensitive data (JWT secret, admin password, license file) |

### Health Probes

```yaml
livenessProbe:
  httpGet:
    path: /health
    port: 3001
  initialDelaySeconds: 10
  periodSeconds: 30
readinessProbe:
  httpGet:
    path: /api/health/detailed
    port: 3001
  initialDelaySeconds: 5
  periodSeconds: 10
```

### Graceful Shutdown

On SIGTERM, each instance:

1. Stops accepting new connections
2. Closes active WebSocket connections
3. Releases its license seat via Redis
4. Flushes pending audit log entries
5. Exits

The readiness probe fails immediately on SIGTERM, so the load balancer stops routing traffic before the instance shuts down.

## Monitoring

See [Performance & Monitoring](./monitoring) for:
- Health check endpoints
- Cluster metrics API
- Instance registry API
- Docker/K8s health probes

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Instances can't connect to PostgreSQL | Check `MADHYAMAS_DATABASE_URL` and network connectivity |
| Instances can't connect to Redis | Check `MADHYAMAS_REDIS_URL` and network connectivity |
| WebSocket events not propagating | Verify Redis pub/sub is working: `redis-cli PUBSUB CHANNELS` |
| Duplicate users on startup | This was a race condition — ensure you're running the latest version |
| `pg_type_typname_nsp_index` error | This was a DDL race — ensure you're running the latest version |
| Seat limit exceeded | Stop unused instances or upgrade your license plan |
| CA certificate mismatch | Ensure all instances use the same `MADHYAMAS_CA_CERT_FILE` volume |

## See Also

- [Getting Started](./getting-started) — First-run setup
- [Configuration](./configuration) — All CLI flags and environment variables
- [Performance & Monitoring](./monitoring) — Health checks and cluster metrics
- [Licensing](./licensing) — Seat management and coordination

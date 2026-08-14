---
title: Enterprise Configuration
description: Complete reference for Madhyamas Enterprise CLI flags, environment variables, production checklist, and configuration import/export.
---

# Enterprise Configuration

This page is a complete reference for all enterprise-specific CLI flags and environment variables. For general (OSS) configuration, see the [Configuration](../configuration) page.

## Authentication

| Flag | Environment Variable | Default | Description |
|------|----------------------|---------|-------------|
| `--enable-auth` | `MADHYAMAS_ENABLE_AUTH` | `false` | Enable authentication middleware |
| `--jwt-secret` | `MADHYAMAS_JWT_SECRET` | dev secret | HMAC-SHA256 secret for JWT signing |
| `--jwt-expiration-secs` | `MADHYAMAS_JWT_EXPIRATION_SECS` | `3600` | Access token lifetime (seconds) |
| `--refresh-interval-secs` | `MADHYAMAS_REFRESH_INTERVAL_SECS` | `300` | Web UI token refresh interval |
| `--api-key-header` | `MADHYAMAS_API_KEY_HEADER` | `X-API-Key` | Header name for API key auth |
| `--auth-mode` | `MADHYAMAS_AUTH_MODE` | `local` | Auth mode: `local`, `oidc`, `header`, `ldap`, `saml` |
| `--proxy-auth` | `MADHYAMAS_PROXY_AUTH` | `false` | Require auth for proxy connections |

## OIDC

| Flag | Environment Variable | Default | Description |
|------|----------------------|---------|-------------|
| `--oidc-issuer` | `MADHYAMAS_OIDC_ISSUER` | — | OIDC issuer URL |
| `--oidc-client-id` | `MADHYAMAS_OIDC_CLIENT_ID` | — | OIDC client ID |
| `--oidc-client-secret` | `MADHYAMAS_OIDC_CLIENT_SECRET` | — | OIDC client secret |
| `--oidc-redirect-uri` | `MADHYAMAS_OIDC_REDIRECT_URI` | — | OIDC callback URL |

## LDAP

| Flag | Environment Variable | Default | Description |
|------|----------------------|---------|-------------|
| `--ldap-url` | `MADHYAMAS_LDAP_URL` | — | LDAP server URL |
| `--ldap-base-dn` | `MADHYAMAS_LDAP_BASE_DN` | — | Base DN for user search |
| `--ldap-bind-dn` | `MADHYAMAS_LDAP_BIND_DN` | — | Service account DN |
| `--ldap-bind-password` | `MADHYAMAS_LDAP_BIND_PASSWORD` | — | Service account password |
| `--ldap-user-filter` | `MADHYAMAS_LDAP_USER_FILTER` | — | User search filter (use `{username}`) |

## Header-based Auth

| Flag | Environment Variable | Default | Description |
|------|----------------------|---------|-------------|
| `--auth-header` | `MADHYAMAS_AUTH_HEADER` | `X-Forwarded-User` | Header containing username |
| `--auth-header-role` | `MADHYAMAS_AUTH_HEADER_ROLE` | `X-Forwarded-Role` | Header containing role |

## Database

| Flag | Environment Variable | Default | Description |
|------|----------------------|---------|-------------|
| `--database-url` | `MADHYAMAS_DATABASE_URL` | SQLite | PostgreSQL connection URL |
| `--database-read-url` | `MADHYAMAS_DATABASE_READ_URL` | — | Read replica URL (optional) |
| `--database-url-file` | `MADHYAMAS_DATABASE_URL_FILE` | — | Path to file containing DB URL (for secret managers) |

::: tip Secret managers
Use `--database-url-file` with Kubernetes Secrets, AWS Secrets Manager, or HashiCorp Vault to avoid exposing the database URL in environment variables.
:::

### PostgreSQL URL Format

```
postgres://username:password@host:port/database?sslmode=require
```

Examples:

```bash
# Local PostgreSQL
--database-url postgres://madhyamas:password@localhost:5432/madhyamas

# With TLS
--database-url "postgres://madhyamas:password@db.internal:5432/madhyamas?sslmode=require"

# Read replica
--database-url postgres://primary.internal:5432/madhyamas \
--database-read-url postgres://replica.internal:5432/madhyamas
```

## Redis

| Flag | Environment Variable | Default | Description |
|------|----------------------|---------|-------------|
| `--redis-url` | `MADHYAMAS_REDIS_URL` | — | Redis URL for multi-instance pub/sub |
| `--redis-ca-cert` | `MADHYAMAS_REDIS_CA_CERT` | system CA | Path to CA cert for Redis TLS verification |

### Redis URL Format

```
redis://host:port          # Plain TCP
rediss://host:port         # TLS
redis://:password@host:port  # With password
```

## License

| Flag | Environment Variable | Default | Description |
|------|----------------------|---------|-------------|
| `--license-file` | `MADHYAMAS_LICENSE_FILE` | — | Path to Ed25519-signed license file |
| `--instance-id` | `MADHYAMAS_INSTANCE_ID` | auto-generated | Unique instance ID for license enforcement |

## Admin Bootstrap

| Flag | Environment Variable | Default | Description |
|------|----------------------|---------|-------------|
| `--admin-username` | `MADHYAMAS_ADMIN_USERNAME` | `admin` | Bootstrap admin username |
| `--admin-password` | `MADHYAMAS_ADMIN_PASSWORD` | random | Bootstrap admin password |

## TLS / CA Certificate

| Flag | Environment Variable | Default | Description |
|------|----------------------|---------|-------------|
| `--ca-cert-file` | `MADHYAMAS_CA_CERT_FILE` | generated | Path to shared CA certificate |
| `--ca-key-file` | `MADHYAMAS_CA_KEY_FILE` | generated | Path to shared CA private key |

## Load Balancer

| Flag | Environment Variable | Default | Description |
|------|----------------------|---------|-------------|
| `--base-path` | `MADHYAMAS_BASE_PATH` | `/` | Base path for context-path routing |

## CLI / MCP Auth

| Flag | Environment Variable | Default | Description |
|------|----------------------|---------|-------------|
| `--api-key` | `MADHYAMAS_API_KEY` | — | API key for CLI/MCP authentication |
| `--token` | `MADHYAMAS_TOKEN` | — | JWT token for CLI/MCP authentication |

## Configuration Import/Export

### Export

```bash
curl -H "Authorization: Bearer <token>" \
  http://localhost:3001/api/config/export > config-backup.json
```

### Import

```bash
curl -X POST http://localhost:3001/api/config/import \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  -d @config-backup.json
```

The export includes all intercept rules (mocks, rewrites, breakpoints, throttle, block list), focus hosts, capture settings, and auto-save configuration. It does **not** include users, API keys, audit logs, or license information.

## Production Checklist

### Security

- [ ] **Change JWT secret** — Set `--jwt-secret` to a strong, random value (32+ characters)
- [ ] **Set admin password** — Set `--admin-password` to a strong password
- [ ] **Enable auth** — Set `--enable-auth true`
- [ ] **Install license** — Set `--license-file` to your license
- [ ] **Enable TLS** — Use `sslmode=require` in the database URL
- [ ] **Redis TLS** — Use `rediss://` for Redis connections
- [ ] **Restrict access** — Use `--allowed-ip` to limit proxy access
- [ ] **Proxy auth** — Set `--proxy-auth` if the proxy is exposed

### Database

- [ ] **PostgreSQL 16+** — Use a supported version
- [ ] **Connection pooling** — Use PgBouncer for high-traffic deployments
- [ ] **Backups** — Set up regular PostgreSQL backups
- [ ] **Read replica** — Configure `--database-read-url` for query offloading

### Multi-Instance

- [ ] **Shared JWT secret** — All instances must use the same `--jwt-secret`
- [ ] **Shared CA** — All instances must use the same `--ca-cert-file` volume
- [ ] **Unique instance IDs** — Each instance must have a unique `--instance-id`
- [ ] **Health checks** — Configure load balancer health probes to `/health`
- [ ] **WebSocket stickiness** — Configure `ip_hash` or session affinity for WebSocket

### Monitoring

- [ ] **Health check monitoring** — Monitor `/health` and `/api/health/detailed`
- [ ] **Log aggregation** — Forward logs to your log management system
- [ ] **Alerting** — Set up alerts for unhealthy instances
- [ ] **Audit log retention** — Configure audit log retention policy

## Complete Example

```bash
madhyamas \
  --database-url "postgres://madhyamas:secure-password@db.internal:5432/madhyamas?sslmode=require" \
  --database-read-url "postgres://madhyamas:secure-password@replica.internal:5432/madhyamas?sslmode=require" \
  --redis-url "rediss://:redis-password@redis.internal:6379" \
  --enable-auth \
  --jwt-secret "your-32-char-production-secret" \
  --admin-username admin \
  --admin-password "your-secure-admin-password" \
  --license-file /secrets/license.json \
  --ca-cert-file /certs/ca-cert.pem \
  --ca-key-file /certs/ca-key.pem \
  --instance-id madhyamas-prod-1 \
  --proxy-auth \
  --allowed-ip 10.0.0.0/8 \
  --api-port 3001 \
  --proxy-port 8888
```

## See Also

- [Getting Started](./getting-started) — First-run setup guide
- [Authentication](./authentication) — Auth modes and configuration
- [Multi-Instance Deployment](./deployment) — Docker Compose and Kubernetes
- [Configuration (OSS)](../configuration) — General CLI flags and env vars

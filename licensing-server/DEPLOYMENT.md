# Deployment Guide

This guide covers deploying the Madhyamas licensing server in development
(Docker Compose) and production (Kubernetes).

## Quick Start (Docker Compose)

### Prerequisites

- Docker and Docker Compose
- An Ed25519 keypair (or let the server generate one for development)

### Steps

1. **Generate keys** (skip for development — the server will auto-generate):

   ```sh
   docker compose run --rm licensing-server \
       generate-keys --output-dir /keys
   ```

   The key files are written to `./keys/` (mounted as a volume).

2. **Start all services:**

   ```sh
   docker compose up -d
   ```

   This starts PostgreSQL, Redis, and the licensing server.

3. **Verify the server is running:**

   ```sh
   curl http://localhost:8080/health
   # {"status":"ok"}
   ```

4. **Issue a test license:**

   ```sh
   curl -X POST http://localhost:8080/api/licenses \
       -H "Content-Type: application/json" \
       -H "X-Admin-Key: dev" \
       -d '{
           "customer_id": "cust_test",
           "plan": "enterprise",
           "seats": 10,
           "expires_at": "2027-01-01T00:00:00Z",
           "features": ["auth", "rbac", "audit"]
       }'
   ```

5. **Stop services:**

   ```sh
   docker compose down
   ```

   To remove data volumes: `docker compose down -v`

## Production Deployment (Kubernetes)

### Prerequisites

- A Kubernetes cluster (1.27+)
- `kubectl` configured to access the cluster
- A container registry with the `madhyamas/licensing-server` image
- An Ed25519 keypair stored in a secrets manager
- A PostgreSQL instance (managed or self-hosted)

### Steps

1. **Build and push the Docker image:**

   ```sh
   docker build -t madhyamas/licensing-server:latest \
       -f licensing-server/Dockerfile .
   docker push madhyamas/licensing-server:latest
   ```

2. **Create the namespace:**

   ```sh
   kubectl apply -f deploy/kubernetes/namespace.yaml
   ```

3. **Create secrets:**

   Edit `deploy/kubernetes/secret.yaml` and replace the placeholder values
   with your actual database URL, admin key, and Ed25519 keys. Then apply:

   ```sh
   kubectl apply -f deploy/kubernetes/secret.yaml
   ```

4. **Create the config map:**

   ```sh
   kubectl apply -f deploy/kubernetes/configmap.yaml
   ```

5. **Deploy the server:**

   ```sh
   kubectl apply -f deploy/kubernetes/deployment.yaml
   kubectl apply -f deploy/kubernetes/service.yaml
   ```

6. **Configure ingress** (if using an ingress controller):

   Edit `deploy/kubernetes/ingress.yaml` to set your domain name and TLS
   certificate. Then apply:

   ```sh
   kubectl apply -f deploy/kubernetes/ingress.yaml
   ```

7. **Verify the deployment:**

   ```sh
   kubectl get pods -n madhyamas-licensing
   kubectl get svc -n madhyamas-licensing
   curl https://licensing.madhyamas.ai/health
   ```

### Scaling

The deployment is set to 2 replicas by default. Scale horizontally:

```sh
kubectl scale deployment licensing-server -n madhyamas-licensing --replicas=4
```

The licensing server is stateless (all state is in PostgreSQL), so it scales
horizontally without issues.

### PostgreSQL

For production, use a managed PostgreSQL service (AWS RDS, Google Cloud SQL,
Azure Database for PostgreSQL) rather than running PostgreSQL in Kubernetes.

If running PostgreSQL in Kubernetes, use the
[CloudNativePG operator](https://cloudnative-pg.io/) or
[Zalando Postgres operator](https://github.com/zalando/postgres-operator)
for automated backups, failover, and scaling.

## Configuration Reference

### CLI flags

| Flag | Env var | Default | Description |
|---|---|---|---|
| `--database-url` | `DATABASE_URL` | — | PostgreSQL connection URL (required) |
| `--port` | `PORT` | `8080` | HTTP listen port |
| `--bind-addr` | `BIND_ADDR` | `0.0.0.0` | Bind address |
| `--ed25519-private-key-file` | `ED25519_PRIVATE_KEY_FILE` | — | Path to Ed25519 private key file |
| `--ed25519-public-key-file` | `ED25519_PUBLIC_KEY_FILE` | — | Path to Ed25519 public key file |
| `--admin-key` | `ADMIN_KEY` | `dev` | Admin API key for X-Admin-Key auth |

### Subcommands

| Command | Description |
|---|---|
| `generate-keys --output-dir <dir>` | Generate a fresh Ed25519 keypair |

### Environment variables

| Variable | Default | Description |
|---|---|---|
| `RUST_LOG` | `info` | Logging level (trace/debug/info/warn/error) |
| `MADHYAMAS_LICENSE_PUBLIC_KEY` | — | Set on **proxy instances** (not the licensing server) to the base64-encoded public key |

## Integration with the Proxy

The licensing server issues licenses that the Madhyamas proxy binary verifies
offline. To integrate:

1. **Generate a keypair** on the licensing server (see
   [KEY_MANAGEMENT.md](KEY_MANAGEMENT.md)).

2. **Set the public key on proxy instances:**

   ```sh
   export MADHYAMAS_LICENSE_PUBLIC_KEY="<base64-encoded-public-key>"
   madhyamas serve
   ```

   Or set it in the proxy's systemd service file, Docker environment, or
   Kubernetes secret.

3. **Issue a license** via the licensing server API:

   ```sh
   curl -X POST http://licensing.madhyamas.ai/api/licenses \
       -H "Content-Type: application/json" \
       -H "X-Admin-Key: <your-admin-key>" \
       -d '{
           "customer_id": "cust_acme",
           "customer_name": "Acme Corp",
           "plan": "enterprise",
           "seats": 50,
           "expires_at": "2027-01-01T00:00:00Z",
           "features": ["auth", "rbac", "audit", "multi_instance", "oidc"]
       }'
   ```

   The response is a JSON license file (claims + signature).

4. **Save the license file** and provide it to the proxy:

   ```sh
   madhyamas serve --license-file /path/to/license.json
   ```

   The proxy verifies the Ed25519 signature offline at startup. No network
   access to the licensing server is required at runtime.

5. **Optional: online verification.** The proxy can call
   `POST /api/licenses/verify` on the licensing server for online
   verification (e.g., to check revocation status). This is optional — the
   default is offline verification.

## API Endpoints

| Method | Path | Description |
|---|---|---|
| `GET` | `/health` | Health check |
| `POST` | `/api/licenses` | Issue a new license |
| `GET` | `/api/licenses` | List licenses (optional `?customer_id=` filter) |
| `GET` | `/api/licenses/:id` | Get a license by ID |
| `POST` | `/api/licenses/:id/revoke` | Revoke a license |
| `POST` | `/api/licenses/verify` | Verify a license file |
| `POST` | `/api/seats/register` | Register an instance seat |
| `POST` | `/api/seats/heartbeat` | Refresh a seat heartbeat |
| `POST` | `/api/seats/deregister` | Deregister an instance seat |
| `GET` | `/api/seats/:license_id` | List seats for a license |

All endpoints except `/health` require the `X-Admin-Key` header.

## Monitoring

- **Health check:** `GET /health` returns `{"status":"ok"}`.
- **Logs:** Structured logs via `tracing` (set `RUST_LOG=debug` for verbose
  output, `RUST_LOG=info` for production).
- **Metrics:** (Future) Prometheus metrics endpoint at `/metrics`.

## Security considerations

- Set a strong `ADMIN_KEY` in production (use `openssl rand -hex 32`).
- Use TLS (configure in the ingress controller or a reverse proxy).
- Restrict network access to the licensing server (firewall, security
  groups).
- Store the Ed25519 private key in a secrets manager, not on disk.
- See [KEY_MANAGEMENT.md](KEY_MANAGEMENT.md) for key security.
- See [BACKUP.md](BACKUP.md) for backup and disaster recovery.

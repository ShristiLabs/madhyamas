# Madhyamas Enterprise demo server

A reproducible public demo of Madhyamas Enterprise on a single ARM64 host
(developed against an OrangePi, Ubuntu 24.04) with a Cloudflare Tunnel in
front. Concrete hostnames used by this deployment:

- Web UI + API: `https://madhyamas-demo.shristilabs.com` (via tunnel)
- Public proxy:  `https://madhyamas-proxy.shristilabs.com:8888` (direct)

```
                    Cloudflare edge (TLS)
                          |
   madhyamas-demo.shristilabs.com ── tunnel ──> cloudflared (host service)
                                                └─> localhost:3001 (web UI + API)
                          |
   Internet clients ──────+── direct ──> madhyamas-proxy.shristilabs.com:8888
                          |              (grey-cloud DNS + router port-forward,
                          |               TLS listener + required credentials)
```

Why the split: Cloudflare's public hostnames proxy normal HTTP/HTTPS only —
an HTTP forward proxy (CONNECT to arbitrary origins) cannot traverse the
edge. So the web UI/API go through the tunnel, while the proxy listener is
exposed directly with its own TLS certificate and mandatory authentication.

| Component     | Port | Purpose                                          |
| ------------- | ---- | ------------------------------------------------ |
| `madhyamas` (container, host network) | 3001 | Web UI + API (incl. public `/api/devices/enroll`) |
| `madhyamas` (container, host network) | 8888 | HTTP(S) proxy, credential-enforced               |
| `cloudflared` (host systemd service)  | —    | Token-managed tunnel connector                   |

The server runs in **unlicensed enterprise mode** — auth/RBAC/audit work
without a license file; only Redis seat coordination would need one.

## Prerequisites

- ARM64 Linux host with ≥ 15 GB free disk, Docker + compose plugin, git
- A GitHub fine-grained PAT: **Contents:read on `ShristiLabs/licensing`**
  (only needed to build; the private `licensing-core` dep is fetched with
  it as a BuildKit secret and never persisted in the image)
- A Cloudflare zone + a running token-managed cloudflared service on the
  host (this host: tunnel `c9d22cc8-a067-45bc-8f66-0cd82a5ae39d`)
- Router access for one port-forward (public proxy)

## 1. Build the image

Default path — **workflow-built binary** (no on-host compile; this is what
an SBC demo host should use):

```bash
# from any checkout with gh auth (e.g. your laptop):
gh workflow run demo-build.yml            # Actions: "Demo Build (enterprise aarch64)"
gh run watch                              # note the run id when it finishes
gh run download <run-id> \
  -n madhyamas-enterprise-demo-aarch64-unknown-linux-gnu -D /tmp/demo
tar -xzf /tmp/demo/*.tar.gz -C /tmp/demo
scp /tmp/demo/madhyamas-enterprise-demo-aarch64-unknown-linux-gnu/madhyamas \
  <demo-host>:madhyamas/deploy/demo/       # binary lands next to Dockerfile.demo

# on the demo host (inside a clone of this repo):
cd ~/madhyamas/deploy/demo
cp .env.example .env && chmod 600 .env     # fill in secrets
docker compose build madhyamas            # thin image, seconds
```

Alternative — **full in-repo Docker build** (needs the fine-grained PAT
with Contents:read on `ShristiLabs/licensing` as `LICENSING_TOKEN` in
`.env`; slow on an SBC — the link step needs several GB of RAM):

```bash
git clone https://github.com/ShristiLabs/madhyamas.git ~/madhyamas
cd ~/madhyamas/deploy/demo
cp .env.example .env && chmod 600 .env     # fill in LICENSING_TOKEN, secrets
# point the madhyamas service build at the repo root Dockerfile:
#   build: { context: ../.., dockerfile: Dockerfile, secrets: [licensing_token] }
# (and re-add the licensing_token secret block at the bottom)
docker compose build madhyamas
```

## 2. Cloudflare Tunnel (web UI + API)

The host's existing `cloudflared` systemd service runs a token-managed
tunnel, configured entirely from the dashboard:

1. Zero Trust dashboard → **Networks → Tunnels** → open the tunnel the
   host connector belongs to.
2. Add a **Public Hostname**: `madhyamas-demo.shristilabs.com` → `HTTP` →
   `localhost:3001`. The dashboard creates the proxied (orange-cloud)
   CNAME for you.

Then start the stack and log in at `https://madhyamas-demo.shristilabs.com`
with the bootstrap admin credentials from `.env`:

```bash
docker compose up -d
```

Set `MADHYAMAS_PUBLIC_IP` in `.env` to the host's LAN IP for now (it is
the host baked into the pairing QR); switch it to
`madhyamas-proxy.shristilabs.com` in step 4.

On a fresh host without cloudflared, either install the service
(`cloudflared service install <token>`) or run a `cloudflared` container
alongside — any connector for a tunnel with that public hostname works.

## 3. (Verified) access-control baseline

After first boot, confirm the two guards that make public exposure
defensible:

```bash
# API is guarded (proves MADHYAMAS_ENABLE_AUTH):
curl -s -o /dev/null -w '%{http_code}\n' https://madhyamas-demo.shristilabs.com/api/traffic
# -> 401

# Proxy rejects missing credentials (proves MADHYAMAS_REQUIRE_PROXY_AUTH):
curl -x http://<lan-ip>:8888 -s -o /dev/null -w '%{http_code}\n' https://example.com
# -> 407
```

## 4. Public proxy listener

1. **DNS**: `madhyamas-proxy.shristilabs.com` A record → your public IP,
   **DNS-only (grey cloud)**. Dynamic IP? Re-point it, or script it
   against the CF API.
2. **Router**: forward external 8888 (or any port) → host :8888.
3. **Certificate**: Let's Encrypt via DNS-01 (works with grey-cloud DNS,
   no inbound port 80 needed). On the host (needs sudo):

   ```bash
   sudo apt install certbot python3-certbot-dns-cloudflare
   # CF API token with Zone:DNS:Edit on the zone:
   sudo install -m 600 /dev/null /etc/letsencrypt/cloudflare.ini
   echo "dns_cloudflare_api_token = <token>" | sudo tee -a /etc/letsencrypt/cloudflare.ini >/dev/null
   mkdir -p ~/madhyamas/deploy/demo/certs/proxy-tls
   sudo certbot certonly --dns-cloudflare \
     --dns-cloudflare-credentials /etc/letsencrypt/cloudflare.ini \
     -d madhyamas-proxy.shristilabs.com \
     --deploy-hook "install -m 644 /etc/letsencrypt/live/madhyamas-proxy.shristilabs.com/*.pem $HOME/madhyamas/deploy/demo/certs/proxy-tls/"
   ```

4. Uncomment the two `MADHYAMAS_PROXY_TLS_CERT_FILE` /
   `MADHYAMAS_PROXY_TLS_KEY_FILE` lines in `docker-compose.yml`, set
   `MADHYAMAS_PUBLIC_IP=madhyamas-proxy.shristilabs.com`, then
   `docker compose up -d madhyamas` (restart aborts if the cert is
   unreadable — that is the startup validation working).

Clients now use the proxy as `https://madhyamas-proxy.shristilabs.com:8888`
with credentials (browser/App settings), or scan the QR from the web UI
(Devices panel), which encodes
`host=madhyamas-proxy.shristilabs.com&port=8888&tls=1`.

**Proxy credentials** — what works today:
- `Proxy-Authorization: Bearer <JWT>` (login token) — curl/CLI users
- Device keys (`mdy_dev_…`) via Bearer **or** Basic (either the username
  or password half) — browsers (proxy user/password fields), Android app
- Plain `username:password` Basic is **not accepted** on the proxy yet —
  `AuthManager::authenticate_password` is an unimplemented stub, so that
  arm always 407s. Create a device in the web UI and use its key instead.

Renewal is automatic (`certbot` systemd timer); the deploy-hook refreshes
the mounted copies, but the container must be restarted to reload the
cert — e.g. a weekly `docker compose restart madhyamas` cron/timer.

## Security notes (public demo)

- **Open-relay abuse is the #1 risk of public proxies.** Both
  `MADHYAMAS_PROXY_AUTH` and `MADHYAMAS_REQUIRE_PROXY_AUTH` are set:
  missing *or* invalid proxy credentials get `407`. Never remove them
  while the port is forwarded.
- `MADHYAMAS_ENABLE_AUTH` guards the entire `/api` (default is **off** —
  this stack always sets it).
- `/api/devices/enroll` is intentionally public (single-use, 15-minute
  enrollment tokens) — that is the Android/companion onboarding path.
- The bootstrap admin password and JWT secret are strong randoms from
  `.env` (never rely on the dev defaults).
- Grey-cloud DNS exposes the host's public IP, and a forwarded port
  *will* be scanned. Expect probe noise; review the audit log in the UI.
  Consider a non-standard external port and time-boxing the demo.
- The MITM interception CA lives in the `/data` volume
  (`/data/certs/ca-cert.pem`) — devices that want HTTPS interception must
  trust it; the TLS listener certificate above is unrelated (transport
  TLS for the proxy itself).
- Laptops that must not expose anything extra can skip the port-forward
  entirely and use a tunnel **TCP app** instead:
  `cloudflared access tcp --hostname <tcp-app-hostname> --url localhost:8888`.

## Maintenance

```bash
docker compose logs -f madhyamas        # tail logs
git pull && docker compose build madhyamas && docker compose up -d madhyamas
```

Data (traffic.db, enterprise.db, CA) persists in the `madhyamas_data`
volume; `docker compose down` keeps it (add `-v` to wipe — that also
resets users, devices, and the trusted CA).

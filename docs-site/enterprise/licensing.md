---
title: Licensing
description: License installation, verification, seat management, renewal, pricing tiers, and the Madhyamas license portal.
---

# Licensing

Madhyamas Enterprise requires a valid license file for production use. Licenses are Ed25519-signed JSON documents that encode the customer, plan, seat count, expiry, and enabled features.

## License Installation

### Via CLI Flag

```bash
madhyamas --license-file /path/to/license.json
```

### Via Environment Variable

```bash
export MADHYAMAS_LICENSE_FILE=/path/to/license.json
madhyamas
```

### Via Docker

```yaml
# docker-compose.yml
services:
  madhyamas:
    environment:
      MADHYAMAS_LICENSE_FILE: /licenses/license.json
    volumes:
      - ./licenses:/licenses:ro
```

### Via Kubernetes Secret

```bash
kubectl create secret generic madhyamas-license \
  --from-file=license.json=/path/to/license.json
```

```yaml
# deployment.yaml
spec:
  containers:
    - name: madhyamas
      env:
        - name: MADHYAMAS_LICENSE_FILE
          value: /licenses/license.json
      volumeMounts:
        - name: license
          mountPath: /licenses
          readOnly: true
  volumes:
    - name: license
      secret:
        secretName: madhyamas-license
```

## License Verification

On startup, Madhyamas verifies the license:

1. **Signature verification** — Ed25519 signature is checked against the embedded public key
2. **Expiry check** — License must not be expired
3. **Instance ID check** — If the license specifies an instance ID, it must match `--instance-id`
4. **Feature check** — Enabled features are loaded from the license

If verification fails, Madhyamas starts in **unlicensed mode** with a warning. All features continue to work, but a banner is displayed in the web UI.

## License Details

The License admin panel shows full license information:

![Enterprise license panel](/screenshots/enterprise-license-panel.png)

### Accessing the Panel

1. Log in as an admin
2. Click the **License** icon in the navigation rail

### Information Displayed

| Field | Description |
|-------|-------------|
| License ID | Unique license identifier |
| Customer | Licensed organization name |
| Plan | Pricing tier (Trial, Starter, Pro, Enterprise, Academic) |
| Instance ID | Bound instance ID (if any) |
| Issued | Issue date |
| Expires | Expiration date |
| Days Remaining | Days until expiration (with warning if < 30) |
| Seats | Number of licensed seats |
| Seat Usage | Current seats used / total |
| Enabled Features | List of features enabled by the license |

## Seat Management

Seats are the number of concurrent Madhyamas instances allowed by the license. Each running instance consumes one seat.

### How Seat Tracking Works

1. **Registration** — On startup, the instance registers itself in Redis with a TTL of 120 seconds
2. **Heartbeat** — The instance sends a heartbeat every 60 seconds, refreshing the TTL
3. **Enforcement** — If `active_seats >= license_seats`, the new instance fails to start
4. **Release** — On graceful shutdown (SIGTERM), the instance deregisters and releases its seat
5. **Auto-reap** — If an instance crashes without deregistering, its seat is released when the TTL expires

::: tip Atomic registration
Seat registration uses a Redis Lua script for atomic `ZADD + EXPIRE`, preventing race conditions where two instances could register simultaneously and exceed the seat limit.
:::

### CLI

```bash
madhyamas license info
```

### API

```bash
curl -H "Authorization: Bearer <token>" \
  http://localhost:3001/api/license/info
```

## Pricing Tiers

| Plan | Seats | Price | Features |
|------|-------|-------|----------|
| **Trial** | 5 | Free (30 days) | All enterprise features |
| **Starter** | 10 | $49/mo or $490/yr | All enterprise features |
| **Pro** | 50 | $199/mo or $1,990/yr | All enterprise features + priority support |
| **Enterprise** | Unlimited | $499/mo or $4,990/yr | All features + SSO + dedicated support |
| **Academic** | Unlimited | Free | Requires `.edu` email |

## License Portal

Licenses are managed through the [Madhyamas license portal](https://madhyamas.ai):

1. **Register** — Create an organization account
2. **Select a plan** — Choose a pricing tier
3. **Pay** — Stripe Checkout for credit card payment
4. **Download** — Get your Ed25519-signed license file
5. **Install** — Point `--license-file` at the downloaded file

### Air-Gapped / Offline Licensing

For environments without internet access:

1. Contact sales@madhyamas.ai to purchase a license
2. Receive the license file via secure email or physical media
3. Install via `--license-file`

Offline licenses are verified entirely locally using the embedded Ed25519 public key — no phone-home or online verification required.

## Renewal

Licenses can be renewed through the portal:

1. Log in to [madhyamas.ai](https://madhyamas.ai)
2. Navigate to your license dashboard
3. Click **Renew** and complete payment
4. Download the new license file
5. Replace the old file and restart Madhyamas

::: tip Zero-downtime renewal
In a multi-instance deployment, replace the license file on each instance one at a time. Instances with the old license continue working until restarted.
:::

## Transfer

Licenses can be transferred between organizations by contacting support@madhyamas.ai. The old license is revoked and a new one is issued.

## Troubleshooting

| Problem | Solution |
|---------|----------|
| `License not found` | Check that `--license-file` points to a valid file |
| `Invalid license signature` | The license file was modified or corrupted. Re-download from the portal. |
| `License expired` | Renew the license at madhyamas.ai |
| `Seat limit exceeded` | Stop an unused instance or upgrade your plan |
| `Instance ID mismatch` | Set `--instance-id` to match the value in the license, or remove the binding |
| License banner in UI | License is missing, expired, or invalid. Check the License panel for details. |

## See Also

- [Getting Started](./getting-started) — First-run setup including license installation
- [Multi-Instance Deployment](./deployment) — Seat coordination across instances
- [Configuration](./configuration) — License-related CLI flags
- [CLI & MCP Tools](./cli-mcp) — License info via CLI and MCP

# Network Configuration & IP Detection

## Overview

Madhyamas can be configured to work in different network scenarios:
- **Local development**: Proxy accessible only on the same machine
- **Local network**: Proxy accessible by other devices on the same network (phones, tablets, etc.)
- **Remote hosting**: Proxy hosted on a remote server or cloud instance

The web UI automatically detects and displays the appropriate IP address for
configuring client devices, prioritizing **private network IPs** over public
IPs so devices on the same local network can connect.

## Configuration Options

### 1. Local Development (Default)

By default, Madhyamas binds to `127.0.0.1` (localhost), making it accessible
only from the same machine.

```bash
./madhyamas
```

### 2. Local Network Access

To allow other devices on your network to use the proxy:

**Using Docker Compose:**
```bash
# The default docker-compose.yml already uses --host 0.0.0.0
docker compose up -d
```

**Using CLI directly:**
```bash
./madhyamas --host 0.0.0.0
```

The web UI will automatically detect your machine's local IP address (e.g.,
`192.168.1.100`) and display it in the certificate helper.

### 3. Manual IP Override

If you need to specify a custom IP address (useful for remote hosting or
complex network setups):

**Using Environment Variable:**
```bash
export MADHYAMAS_PUBLIC_IP=192.168.1.100
./madhyamas --host 0.0.0.0
```

**Using Docker Compose** — edit `docker-compose.yml` and uncomment the
`MADHYAMAS_PUBLIC_IP` line:
```yaml
environment:
  - MADHYAMAS_PUBLIC_IP=192.168.1.100
```

**Using CLI Argument:**
```bash
./madhyamas --host 0.0.0.0 --public-ip 192.168.1.100
```

## How IP Detection Works

### Backend Configuration

The backend returns configuration via `/api/config`:

```json
{
  "host": "0.0.0.0",
  "proxy_port": 8888,
  "api_port": 3001,
  "public_ip": null
}
```

- **`host`**: The interface the proxy binds to (`0.0.0.0` for all interfaces,
  `127.0.0.1` for localhost only)
- **`public_ip`**: Optional manual override for the displayed IP address

### Frontend Detection Priority

The frontend follows this priority order:

1. **Manual Override** (`public_ip` from backend) — if `MADHYAMAS_PUBLIC_IP` is
   set, always used.
2. **Backend Host** — if it's a usable private IP (e.g., `192.168.1.100`).
3. **WebRTC Detection** — collects all available IP addresses via WebRTC,
   **prioritizing private IPs** over public IPs.
   - Private IP ranges: `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`
4. **Hostname Fallback** — uses `window.location.hostname` if it's a usable IP.
5. **Placeholder** — shows "Your computer's IP" if all detection methods fail.

### Private IP detection algorithm

```typescript
const isPrivateIP = (ip: string): boolean => {
  const parts = ip.split('.').map(Number);
  if (parts[0] === 10) return true;                          // 10.0.0.0/8
  if (parts[0] === 172 && parts[1] >= 16 && parts[1] <= 31)  // 172.16.0.0/12
    return true;
  if (parts[0] === 192 && parts[1] === 168) return true;     // 192.168.0.0/16
  return false;
};
```

### WebRTC detection process

1. Create `RTCPeerConnection` with a STUN server
2. Collect all ICE candidates (IP addresses)
3. Filter out localhost and invalid IPs
4. Prioritize private IPs over public IPs
5. Return the best IP for display

The STUN server is used to discover all network interfaces (WiFi, Ethernet,
VPN, public) for better reliability across configurations, even though private
IPs are preferred for display.

## Detection by Deployment Method

| Deployment | IP Shown | Why |
|------------|----------|-----|
| **Local** (`startup-local.sh`) | Private IP (e.g., `192.168.1.100`) | WebRTC detects host IPs, prioritizes private |
| **Docker** (`startup.sh`) | Private IP (e.g., `192.168.1.100`) | WebRTC runs in browser on host, detects host IPs (not container IPs) |
| **Remote server** | Public IP (manually set) | `MADHYAMAS_PUBLIC_IP` environment variable |
| **Manual override** | Configured IP | `MADHYAMAS_PUBLIC_IP` takes precedence |

Docker works because WebRTC detection happens in the browser on the host
machine, not inside the container — so it detects the host's network
interfaces.

## Use Cases

### Home Network

```bash
docker compose up -d
# Access web UI at http://localhost:3001
# Mobile devices connect using the displayed private IP
```

### Office Network with Multiple Interfaces

```bash
# Machine has: 192.168.1.100 (WiFi), 10.0.0.50 (Ethernet)
# To force a specific IP:
export MADHYAMAS_PUBLIC_IP=10.0.0.50
./startup-local.sh
```

### Remote Server (AWS, DigitalOcean, etc.)

```bash
export MADHYAMAS_PUBLIC_IP=203.0.113.45
docker compose up -d
# Clients anywhere can connect (if firewall allows)
```

### Corporate Network with VPN

```bash
export MADHYAMAS_PUBLIC_IP=10.8.0.5
./startup-local.sh
# VPN clients can connect using this IP
```

## Troubleshooting

### Web UI shows "Your computer's IP"
- The frontend couldn't detect your IP automatically (WebRTC blocked or failed)
- Check that your browser allows WebRTC
- Disable VPN temporarily during detection
- Manually set `MADHYAMAS_PUBLIC_IP` environment variable

### Public IP shown instead of private IP
- Hard refresh the browser (Cmd+Shift+R / Ctrl+Shift+F5)
- Clear browser cache
- Manually set the private IP: `export MADHYAMAS_PUBLIC_IP=192.168.1.100`

### Wrong private IP displayed (multiple interfaces)
- Manually specify the correct IP:
  ```bash
  export MADHYAMAS_PUBLIC_IP=192.168.1.100
  ./startup-local.sh
  ```

### Docker shows container IP instead of host IP
- This shouldn't happen (WebRTC runs in the browser on the host)
- If you see a container IP (like `172.17.0.x`), manually set the host IP:
  ```bash
  export MADHYAMAS_PUBLIC_IP=192.168.1.100
  ./startup.sh
  ```

### Mobile devices can't connect
- Ensure firewall allows connections on ports 8888 and 3001
- Verify you're using `--host 0.0.0.0` (not `127.0.0.1`)
- Check that the mobile device is on the same network
- Try manually setting the IP with `--public-ip`

## See Also

- [DEPLOYMENT.md](DEPLOYMENT.md) — Docker, Kubernetes, cloud deployment
- [DEVELOPMENT.md](DEVELOPMENT.md) — Local development setup
- [ACCESS_CONTROL.md](ACCESS_CONTROL.md) — CIDR-based IP allowlist
- [GETTING_STARTED.md](GETTING_STARTED.md) — User-facing getting started guide

# Network Configuration Guide

## Overview

ProxyForge can be configured to work in different network scenarios:
- **Local development**: Proxy accessible only on the same machine
- **Local network**: Proxy accessible by other devices on the same network (phones, tablets, etc.)
- **Remote hosting**: Proxy hosted on a remote server or cloud instance

## Configuration Options

### 1. Local Development (Default)

By default, ProxyForge binds to `127.0.0.1` (localhost), making it accessible only from the same machine.

```bash
./proxyforge
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
./proxyforge --host 0.0.0.0
```

The web UI will automatically detect your machine's local IP address (e.g., `192.168.1.100`) and display it in the certificate helper.

### 3. Manual IP Override

If you need to specify a custom IP address (useful for remote hosting or complex network setups):

**Using Environment Variable:**
```bash
export PROXYFORGE_PUBLIC_IP=192.168.1.100
./proxyforge --host 0.0.0.0
```

**Using Docker Compose:**
Edit `docker-compose.yml` and uncomment the `PROXYFORGE_PUBLIC_IP` line:
```yaml
environment:
  - PROXYFORGE_PUBLIC_IP=192.168.1.100
```

**Using CLI Argument:**
```bash
./proxyforge --host 0.0.0.0 --public-ip 192.168.1.100
```

## How It Works

1. **Backend Configuration**: The `public_ip` field in `ProxyConfig` allows manual override
2. **API Response**: The `/api/config` endpoint returns either:
   - The configured `public_ip` if set
   - The `host` value for client-side detection
3. **Frontend Detection**: The web UI:
   - Uses the `public_ip` if provided by the backend
   - Falls back to WebRTC-based local IP detection if `host` is `0.0.0.0` or `127.x.x.x`
   - Displays the detected IP in the certificate helper for easy mobile device setup

## Use Cases

### Home Network
```bash
# Let ProxyForge auto-detect your local IP
docker compose up -d
# Access web UI at http://localhost:3001
# Mobile devices can connect using the displayed IP
```

### Remote Server
```bash
# Set your server's public IP
export PROXYFORGE_PUBLIC_IP=203.0.113.45
./proxyforge --host 0.0.0.0
```

### Complex Network Setup
```bash
# Specify the exact IP to display
./proxyforge --host 0.0.0.0 --public-ip 10.0.1.50
```

## Troubleshooting

### Web UI shows "Your computer's IP"
- The frontend couldn't detect your IP automatically
- Manually set `PROXYFORGE_PUBLIC_IP` environment variable
- Check that your browser allows WebRTC (required for auto-detection)

### Mobile devices can't connect
- Ensure firewall allows connections on ports 8888 and 3001
- Verify you're using `--host 0.0.0.0` (not `127.0.0.1`)
- Check that mobile device is on the same network
- Try manually setting the IP with `--public-ip`

### Wrong IP displayed
- Set the correct IP using `PROXYFORGE_PUBLIC_IP` environment variable
- Or use `--public-ip` CLI argument

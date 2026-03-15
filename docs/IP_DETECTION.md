# IP Address Detection and Management

## Overview

Madhyamas automatically detects and displays the appropriate IP address for configuring client devices. The system prioritizes **private network IPs** over public IPs to ensure devices on the same local network can connect.

## How IP Detection Works

### 1. Backend Configuration

The backend returns configuration via `/api/config` endpoint:

```json
{
  "host": "0.0.0.0",
  "proxy_port": 8888,
  "api_port": 3001,
  "public_ip": null
}
```

- **`host`**: The interface the proxy binds to (`0.0.0.0` for all interfaces, `127.0.0.1` for localhost only)
- **`public_ip`**: Optional manual override for the displayed IP address

### 2. Frontend Detection Logic

The frontend follows this priority order:

1. **Manual Override** (`public_ip` from backend)
   - If `MADHYAMAS_PUBLIC_IP` environment variable is set
   - Highest priority - always used if configured

2. **Backend Host** (if it's a usable private IP)
   - If backend returns a specific private IP like `192.168.1.100`
   - Used directly without further detection

3. **WebRTC Detection** (automatic)
   - Collects all available IP addresses via WebRTC
   - **Prioritizes private IPs** over public IPs
   - Private IP ranges:
     - `10.0.0.0/8` (10.0.0.0 - 10.255.255.255)
     - `172.16.0.0/12` (172.16.0.0 - 172.31.255.255)
     - `192.168.0.0/16` (192.168.0.0 - 192.168.255.255)

4. **Hostname Fallback**
   - Uses `window.location.hostname` if it's a usable IP

5. **Placeholder**
   - Shows "Your computer's IP" if all detection methods fail

## IP Detection by Deployment Method

### Running Locally (startup-local.sh)

**Scenario**: Madhyamas running directly on host machine

```bash
./startup-local.sh
```

**Detection Flow**:
1. Backend returns `host: "0.0.0.0"` (not usable for display)
2. Frontend uses WebRTC to detect all IPs
3. **Private IP is prioritized** (e.g., `192.168.1.100`)
4. Public IP is ignored even if detected
5. UI displays: `192.168.1.100:8888`

**Result**: ✅ Shows private network IP for local network access

### Running in Docker (startup.sh)

**Scenario**: Madhyamas running in Docker container

```bash
./startup.sh
```

**Detection Flow**:
1. Backend returns `host: "0.0.0.0"` (container binds to all interfaces)
2. Frontend uses WebRTC to detect host machine's IPs
3. **Private IP is prioritized** (e.g., `192.168.1.100`)
4. Public IP is ignored
5. UI displays: `192.168.1.100:8888`

**Result**: ✅ Shows private network IP for local network access

**Why it works**: Even though Madhyamas runs in a container, the WebRTC detection happens in the browser on the host machine, so it detects the host's network interfaces.

### Remote Server Deployment

**Scenario**: Madhyamas running on a cloud server or VPS

```bash
export MADHYAMAS_PUBLIC_IP=203.0.113.45
./startup.sh
```

**Detection Flow**:
1. Backend returns `public_ip: "203.0.113.45"` (manually configured)
2. Frontend uses this IP directly
3. UI displays: `203.0.113.45:8888`

**Result**: ✅ Shows configured public IP for remote access

## Configuration Examples

### Example 1: Home Network (Default)

**Setup**:
```bash
# No configuration needed
./startup-local.sh
# or
./startup.sh
```

**Result**:
- Detects private IP: `192.168.1.100`
- Mobile devices on same network can connect to `192.168.1.100:8888`

### Example 2: Office Network with Multiple Interfaces

**Setup**:
```bash
# Machine has multiple IPs: 192.168.1.100 (WiFi), 10.0.0.50 (Ethernet)
./startup-local.sh
```

**Result**:
- WebRTC detects both IPs
- First private IP found is used (order depends on network interface priority)
- To force specific IP, use manual override:
  ```bash
  export MADHYAMAS_PUBLIC_IP=10.0.0.50
  ./startup-local.sh
  ```

### Example 3: Cloud Server (AWS, DigitalOcean, etc.)

**Setup**:
```bash
# Server has public IP: 203.0.113.45
export MADHYAMAS_PUBLIC_IP=203.0.113.45
docker compose up -d
```

**Result**:
- UI displays: `203.0.113.45:8888`
- Clients anywhere can connect (if firewall allows)

### Example 4: Corporate Network with VPN

**Setup**:
```bash
# VPN IP: 10.8.0.5, Local IP: 192.168.1.100
export MADHYAMAS_PUBLIC_IP=10.8.0.5
./startup-local.sh
```

**Result**:
- UI displays: `10.8.0.5:8888`
- VPN clients can connect using this IP

## Troubleshooting

### Issue: Public IP Shown Instead of Private IP

**Cause**: WebRTC detection found public IP first

**Solution**: The updated frontend now prioritizes private IPs. If you still see a public IP:
1. Refresh the browser (hard refresh: Cmd+Shift+R or Ctrl+Shift+F5)
2. Clear browser cache
3. Manually set the IP:
   ```bash
   export MADHYAMAS_PUBLIC_IP=192.168.1.100
   ./startup-local.sh
   ```

### Issue: Wrong Private IP Displayed

**Cause**: Multiple network interfaces detected

**Solution**: Manually specify the correct IP:
```bash
export MADHYAMAS_PUBLIC_IP=192.168.1.100
./startup-local.sh
```

### Issue: "Your computer's IP" Placeholder Shown

**Cause**: WebRTC detection failed or blocked

**Solutions**:
1. Check browser permissions (WebRTC must be enabled)
2. Disable VPN temporarily during detection
3. Manually set IP:
   ```bash
   export MADHYAMAS_PUBLIC_IP=192.168.1.100
   ./startup-local.sh
   ```

### Issue: Docker Shows Container IP Instead of Host IP

**Cause**: This shouldn't happen with current implementation

**Solution**: 
- WebRTC runs in browser on host, not in container
- If you see container IP (like `172.17.0.x`), manually set host IP:
  ```bash
  export MADHYAMAS_PUBLIC_IP=192.168.1.100
  ./startup.sh
  ```

## Technical Details

### Private IP Detection Algorithm

```typescript
const isPrivateIP = (ip: string): boolean => {
  const parts = ip.split('.').map(Number);
  
  // 10.0.0.0/8
  if (parts[0] === 10) return true;
  
  // 172.16.0.0/12
  if (parts[0] === 172 && parts[1] >= 16 && parts[1] <= 31) return true;
  
  // 192.168.0.0/16
  if (parts[0] === 192 && parts[1] === 168) return true;
  
  return false;
};
```

### WebRTC Detection Process

1. Create RTCPeerConnection with STUN server
2. Collect all ICE candidates (IP addresses)
3. Filter out localhost and invalid IPs
4. **Prioritize private IPs** over public IPs
5. Return best IP for display

### Why STUN Server is Still Used

Even though we prioritize private IPs, the STUN server helps discover all available network interfaces, including:
- Multiple private IPs (WiFi, Ethernet, VPN)
- Public IP (for awareness, but deprioritized)
- Better reliability across different network configurations

## Summary

| Deployment | IP Shown | Why |
|------------|----------|-----|
| **Local (startup-local.sh)** | Private IP (e.g., 192.168.1.100) | WebRTC detects host IPs, prioritizes private |
| **Docker (startup.sh)** | Private IP (e.g., 192.168.1.100) | WebRTC runs in browser on host, detects host IPs |
| **Remote Server** | Public IP (manually set) | `MADHYAMAS_PUBLIC_IP` environment variable |
| **Manual Override** | Configured IP | `MADHYAMAS_PUBLIC_IP` takes precedence |

The system is designed to **automatically show the correct private IP** for local network access in both local and Docker deployments, while allowing manual override for special cases like remote servers or VPN scenarios.

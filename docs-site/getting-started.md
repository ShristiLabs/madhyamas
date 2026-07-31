# Getting Started

Welcome to **Madhyamas** — an open-source HTTP/HTTPS debugging proxy that lets you inspect, modify, and replay network traffic between your applications and the servers they talk to. Whether you're debugging an API, testing error handling, or simulating slow networks, Madhyamas gives you full visibility and control.

![Madhyamas Dashboard](/screenshots/app-overview.png)

## What You'll Need

- **macOS**, **Windows**, or **Linux** computer
- A terminal/command prompt
- The application or device you want to debug (browser, mobile app, API client, etc.)

## Installation

### Option 1: Homebrew (macOS)

```bash
brew install madhyamas
```

### Option 2: Cargo (Rust)

```bash
cargo install madhyamas
```

### Option 3: Pre-built Binary

Download the latest release for your platform from the [GitHub Releases page](https://github.com/ShristiLabs/madhyamas/releases), extract the archive, and move the binary to your PATH:

```bash
# macOS / Linux
tar -xzf madhyamas-*.tar.gz
sudo mv madhyamas /usr/local/bin/

# Windows
# Extract the .zip and add madhyamas.exe to your PATH
```

## Starting the Proxy

Open a terminal and run:

```bash
madhyamas serve
```

You'll see output like this:

```
Madhyamas is ready!
Proxy: http://0.0.0.0:8888
Web UI: http://0.0.0.0:3001
```

This starts two services:

| Service | Port | Purpose |
|---------|------|---------|
| **Proxy** | 8888 | Receives HTTP/HTTPS traffic from your applications |
| **Web UI** | 3001 | Browser-based dashboard for inspecting and controlling traffic |

Open **http://localhost:3001** in your browser to see the Madhyamas dashboard.

## Connecting Your First Client

### Browser (Chrome / Firefox / Safari)

Configure your browser to use the proxy at **localhost:8888**:

- **Firefox**: Settings → Network Settings → Manual proxy configuration → HTTP Proxy: `localhost`, Port: `8888`
- **Chrome**: Launch with `--proxy-server=localhost:8888` or use system proxy settings
- **Safari**: System Settings → Network → Advanced → Proxies → Web Proxy (HTTP): `localhost:8888`

Once configured, visit any website. You'll see the traffic appear in the Madhyamas dashboard in real time.

### Command-Line Tools (curl)

```bash
curl -x http://localhost:8888 http://httpbin.org/get
```

### Mobile Devices

See the [Mobile Setup](./mobile-setup) guide for detailed instructions on connecting phones and tablets.

## HTTPS Interception

By default, Madhyamas intercepts HTTPS traffic by generating a local Certificate Authority (CA) and creating certificates on the fly. To avoid browser warnings, you need to install the Madhyamas CA certificate on your system.

The easiest way is to click the **Setup** button in the top toolbar — it provides platform-specific instructions and a download link.

![Setup Dialog](/screenshots/setup-dialog.png)

For detailed certificate installation instructions, see the [HTTPS & Certificates](./https-certificates) guide.

## What's Next?

- [Traffic Inspection](./traffic-inspection) — Learn to filter, search, and analyze captured traffic
- [HTTPS & Certificates](./https-certificates) — Set up certificate trust for HTTPS interception
- [Breakpoints](./breakpoints) — Pause and modify requests in real time
- [Mocks](./mocks) — Create fake API responses for testing
- [Mobile Setup](./mobile-setup) — Connect your phone or tablet

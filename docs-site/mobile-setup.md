---
title: Mobile Setup
description: Connect iPhones, iPads, and Android devices to the Madhyamas proxy to debug mobile app traffic — Wi-Fi proxy config, CA certificate install, companion VPN app, and troubleshooting.
---

# Mobile Setup

One of the most powerful uses of Madhyamas is debugging traffic from **mobile apps** — something browser DevTools can't do. This guide walks you through connecting your phone or tablet to the Madhyamas proxy.

## Prerequisites

- Madhyamas running on your computer (see [Getting Started](./getting-started))
- Your phone/tablet and computer on the **same Wi-Fi network**
- Your computer's local IP address (shown in the Madhyamas web UI header or via `ifconfig` / `ipconfig`)

::: tip Bind to all interfaces
By default, Madhyamas only listens on `localhost`. To accept connections from mobile devices, start it with:
```bash
madhyamas serve --host 0.0.0.0
```
:::

## Finding Your Computer's IP Address

### macOS
```bash
ipconfig getifaddr en0   # Wi-Fi
# or
ipconfig getifaddr en1   # alternate Wi-Fi interface
```

### Windows
```powershell
ipconfig | findstr IPv4
```

### Linux
```bash
hostname -I
```

The Madhyamas web UI also displays the IP address you should use in its header bar.

## iPhone / iPad Setup

### Step 1: Configure Wi-Fi Proxy

1. Open **Settings → Wi-Fi**
2. Tap the **(i)** next to your connected network
3. Scroll to **HTTP Proxy** → tap **Manual**
4. Set:
   - **Server**: Your computer's IP address (e.g., `192.168.1.100`)
   - **Port**: `8888`
   - **Authentication**: Off
5. Tap **Save**

### Step 2: Install the CA Certificate

1. Open Safari and visit `http://<your-computer-ip>:3001/api/cert/ca`
2. Tap **Allow** to download the profile
3. Go to **Settings → Profile Downloaded** → tap **Install**
4. Enter your passcode if prompted
5. Go to **Settings → General → About → Certificate Trust Settings**
6. Enable trust for **Madhyamas Root CA**

### Step 3: Verify

Open any app or website on your phone. You should see the traffic appear in the Madhyamas dashboard in real time.

### Step 4: Disable Proxy When Done

Return to **Settings → Wi-Fi → (i) → HTTP Proxy** and set it to **Off** when you're done debugging.

## Android Setup

### Step 1: Configure Wi-Fi Proxy

1. Open **Settings → Network & Internet → Wi-Fi**
2. Long-press your connected network → **Modify network**
3. Check **Advanced options**
4. Set **Proxy**: **Manual**
5. Set:
   - **Proxy hostname**: Your computer's IP address
   - **Proxy port**: `8888`
   - **Bypass**: Leave empty
6. **Save**

### Step 2: Install the CA Certificate

1. Open your browser and visit `http://<your-computer-ip>:3001/api/cert/ca`
2. Download the certificate file
3. Go to **Settings → Security → Install a certificate → CA certificate**
4. Select the downloaded file

::: warning Android 7.0+
Apps don't trust user-installed CA certificates by default. To intercept HTTPS from specific apps, you may need to:
- Use a rooted device and install the cert in the system store
- Use the Madhyamas Android companion VPN app (no root required)
- Modify the app's network security config (if you have the source code)
:::

### Step 3: Verify

Open any app or website. Traffic should appear in the Madhyamas dashboard.

### Step 4: Disable Proxy When Done

Return to **Settings → Wi-Fi → [network] → Advanced → Proxy** and set it to **None**.

## Android Companion VPN App

For devices where manual proxy configuration is problematic, Madhyamas includes an Android companion app that creates a local VPN to route traffic through the proxy transparently — no manual Wi-Fi proxy settings needed.

```bash
# Build the companion app (requires Android SDK)
cd android
echo "sdk.dir=$HOME/Library/Android/sdk" > local.properties
./gradlew assembleDebug
adb install app/build/outputs/apk/debug/app-debug.apk
```

Open the app, enter your computer's IP address and port 8888, and tap **Start**. All app traffic will be routed through Madhyamas.

## Troubleshooting Mobile Connections

### "Connection Refused" or No Traffic Appears

1. **Check the bind address**: Make sure you started Madhyamas with `--host 0.0.0.0`
2. **Check the IP address**: Make sure you entered your computer's correct LAN IP
3. **Check the port**: The default proxy port is 8888
4. **Check Wi-Fi**: Make sure both devices are on the same network
5. **Check firewall**: Temporarily disable your computer's firewall to test

### HTTPS Sites Don't Load

The CA certificate isn't installed or trusted. Repeat the certificate installation steps above. On iOS, make sure you enabled trust in **Certificate Trust Settings** (not just installed the profile).

### Specific Apps Don't Work

The app may use **certificate pinning** — it rejects the proxy's CA certificate. See the [HTTPS & Certificates](./https-certificates) guide for more information.

### Traffic Appears but No HTTPS Content

HTTPS interception may be disabled. Check that `intercept_https` is enabled in the configuration.

### Office/Corporate Network Issues

Corporate Wi-Fi networks often have **client isolation** enabled, which prevents devices on the same network from communicating with each other. If this is the case, try:
- Using a personal hotspot from your phone
- Connecting your computer via Ethernet and your phone via Wi-Fi
- Asking your IT department to disable client isolation for your devices

## See also

- [HTTPS & Certificates](./https-certificates) — CA installation and certificate pinning
- [Configuration](./configuration) — `--host 0.0.0.0` and `--public-ip` flags
- [Access Control](./access-control) — restrict which devices can connect
- [Troubleshooting](./troubleshooting) — mobile connection fixes

# Madhyamas VPN — Android Companion App

A VPN-based companion app for routing Android app traffic to the Madhyamas
debugging proxy. Uses Android's `VpnService` API to transparently capture
TCP traffic from selected apps and forward it to the Madhyamas proxy via
HTTP CONNECT — no root required.

## Features

- **Transparent proxy routing**: No need to manually configure WiFi proxy
  settings. The VPN captures traffic at the network layer.
- **Per-app filtering**: Select specific apps to intercept, or route all
  apps. Exclude system apps by default.
- **CA certificate installation**: Download and install the Madhyamas CA
  certificate directly from the app with one tap.
- **Live statistics**: Monitor active connections, total connections, and
  data transfer in real-time.
- **No root required**: Uses Android's built-in VpnService API.

## Architecture

```
Android App  ──TCP──►  VpnService (TUN)  ──►  TCP Relay  ──CONNECT──►  Madhyamas Proxy
                         │                                                    │
                         │             ◄──────  TCP Relay  ◄────  200 OK  ◄──┘
                         │
                    App receives
                    intercepted TLS
```

1. **VpnService** creates a TUN interface that captures IP packets from
   selected apps
2. **Packet parser** extracts TCP source/destination ports and IP addresses
3. **TcpRelay** connects to the Madhyamas proxy and sends an HTTP CONNECT
   request for the destination
4. Once the proxy responds with `200 Connection Established`, data is
   relayed bidirectionally
5. The Madhyamas proxy performs TLS interception using its CA certificate

## Building

### Prerequisites

- Android SDK (API 24+, compile SDK 35)
- JDK 17
- Android Studio or Gradle 8.9+

### Build from command line

```bash
cd android

# Set SDK location (if not using Android Studio)
echo "sdk.dir=$HOME/Library/Android/sdk" > local.properties

# Build debug APK
./gradlew assembleDebug

# Output: android/app/build/outputs/apk/debug/app-debug.apk
```

### Build with Android Studio

1. Open Android Studio
2. File → Open → Select the `android/` directory
3. Wait for Gradle sync to complete
4. Run → Run 'app'

## Usage

### 1. Start the Madhyamas proxy

```bash
madhyamas serve
# Proxy listens on :8888, API on :3001
```

### 2. Install the companion app

```bash
adb install android/app/build/outputs/apk/debug/app-debug.apk
```

### 3. Configure the proxy address

Open the app → Settings (gear icon) → Set proxy host/port to match your
Madhyamas instance. If Madhyamas is running on your computer, use your
computer's IP address (not `127.0.0.1`, since the phone needs to reach it
over the network).

### 4. Install the CA certificate

Tap "Install CA Certificate" → The app downloads the CA from the Madhyamas
API and launches the Android certificate installer.

**Important**: After installing the CA as a user certificate, Android will
warn that it's not trusted by all apps. For apps that use the system trust
store (no pinning), this is sufficient. For apps with certificate pinning,
see the next section.

### 5. Select apps to intercept

Tap the app selector to choose which apps' traffic to route through the
VPN. You can select specific apps or leave it on "All Apps".

### 6. Start the VPN

Tap "Start VPN" → Android will show a VPN connection permission dialog.
Approve it, and the VPN starts routing traffic to the Madhyamas proxy.

## Certificate Pinning

The VPN companion app handles **traffic routing** — it does not bypass
certificate pinning by itself. For apps that implement pinning, use one of
these approaches in combination with the VPN:

### No root — APK patching

Patch the APK to trust user certificates and disable OkHttp pinning:

```bash
# Using apk-mitm (Node.js)
npm install -g apk-mitm
apk-mitm target-app.apk
adb install target-app-patched.apk
```

### Rooted — Frida

Use Frida to hook SSL verification at runtime:

```bash
# Install frida-server on device
adb push frida-server /data/local/tmp/
adb shell chmod +x /data/local/tmp/frida-server
adb shell /data/local/tmp/frida-server &

# Run unpinning script
frida -U -f com.target.app \
  -l https://raw.githubusercontent.com/httptoolkit/frida-interception-and-unpinning/main/android/android-certificate-unpinning.js
```

### Rooted — Magisk module

Install the Madhyamas CA into the system certificate store:

```bash
# Generate a Magisk module that installs the CA
# (See docs/ANDROID_CERT_PINNING.md for details)
```

See [docs/ANDROID_CERT_PINNING.md](../docs/ANDROID_CERT_PINNING.md) for a
comprehensive guide on all pinning bypass approaches.

## Limitations

- **IPv4 only**: The current implementation handles IPv4 packets only.
  IPv6 support is planned.
- **TCP only**: UDP traffic (e.g., QUIC, DNS) is not intercepted. Apps
  using QUIC/HTTP3 will fall back to TCP in most cases.
- **SNI passthrough**: The CONNECT request uses IP addresses, not
  hostnames. A DNS cache is needed for proper SNI/hostname support.
- **TCP state management**: The current packet handling is simplified.
  Production use requires proper TCP sequence number tracking, ACK
  handling, and flow control.
- **Certificate pinning**: The VPN routes traffic but does not bypass
  pinning. Use additional tools (Frida, APK patching) for pinned apps.

## Tech Stack

- **Language**: Kotlin
- **UI**: Jetpack Compose (Material 3)
- **VPN**: Android VpnService API
- **Networking**: Java NIO sockets
- **Storage**: DataStore Preferences
- **Min SDK**: 24 (Android 7.0)
- **Target SDK**: 35 (Android 15)

# Madhyamas iOS Companion

The iOS counterpart of the Android companion app (`../android/`): pairs
with a Madhyamas enterprise server via the `madhyamas://connect` QR deep
link, then captures device traffic through a Network Extension and
re-originates every TCP connection to the proxy with an app-authored
HTTP `CONNECT` carrying the paired device credential.

```
app flow ──► NEAppProxyProvider ──► NWConnection (TLS if tls=1) ──CONNECT──► Madhyamas proxy
   ▲                                                      └─ Proxy-Authorization: Basic base64(key + ":")
   └── keychain credential + app-group config (pairing state)
```

## Layout

- `Core/` — SPM package `MadhyamasCore`: ALL pairing/relay logic, no
  UI/NetworkExtension imports — unit-testable with plain `swift test`.
- `MadhyamasiOS/` — SwiftUI app (status, pairing, settings, CA download).
- `Tunnel/` — `NEAppProxyProvider` extension (`MadhyamasProxyProvider`,
  `FlowPipe`).
- `project.yml` — XcodeGen manifest; the generated `Madhyamas.xcodeproj`
  is committed.

## Building & testing (no Apple Developer account needed)

```bash
brew install xcodegen
cd ios
xcodegen generate
cd Core && swift test    # 71 tests, incl. a real-socket pairing E2E
cd ..
xcodebuild -project Madhyamas.xcodeproj -scheme MadhyamasiOS \
  -destination 'generic/platform=iOS Simulator' build \
  CODE_SIGNING_ALLOWED=NO CODE_SIGNING_REQUIRED=NO
```

CI runs the same steps (`.github/workflows/ios-ci.yml`).

## Running on a device (requires a paid Apple Developer account)

1. `brew install xcodegen && cd ios && xcodegen generate`, open
   `Madhyamas.xcodeproj`, set your `DEVELOPMENT_TEAM` in both targets'
   Signing & Capabilities, and let Xcode provision the App Group
   (`group.com.madhyamas.shared`), Keychain sharing
   (`com.madhyamas.shared`), and the Network Extensions
   (`app-proxy-provider`) capability.
2. Run the app, approve the VPN permission dialog.
3. Open the server's Devices panel (e.g.
   `https://madhyamas-demo.shristilabs.com`), create a device, scan the
   `madhyamas://connect` QR with the system Camera — the link opens the
   app, pairs it (redeeming the enrollment token when present), and
   capture starts.
4. For HTTPS interception, install the server's CA profile
   (`{api}/cert/ca` — the app's button opens it in Safari), then enable
   full trust in Settings → General → About → Certificate Trust Settings.

## Server contract (identical to the Android app)

- `POST {api}/devices/enroll` body `{"token"}` → `{device:{id,name}, key}`
- `GET {api}/cert/ca` (PEM)
- Proxy: `CONNECT host:port HTTP/1.1` + `Proxy-Authorization` +
  `Proxy-Connection: keep-alive` (ISO-8859-1); `200` establishes, `407`
  rejects (3 consecutive 407s trip the fail-fast breaker until a success)
- TLS to proxy: default trust + hostname verification, never downgraded

## Limitations (parity notes)

- **Route-all capture**: per-app VPN (`appRules`) is macOS-only API —
  on iOS it is only settable through MDM, so while the VPN is on all TCP
  flows are captured (the Android app's app allowlist has no iOS
  equivalent here).
- **UDP/QUIC/DNS are not captured** — UDP flows are denied, same as the
  Android app's IPv4/TCP-only behavior.
- **Stock iOS manual proxy is plaintext-only** — that limitation is why
  this app exists; it speaks TLS to the proxy itself (`tls=1`).
- **MITM CA**: user-installed CAs are trusted by Safari and most
  system-HTTP apps, but not by all third-party apps; pinned apps are not
  interceptable (same as Android 7+).
- Device runs are gated on a paid Apple Developer account; simulator and
  `swift test` work without one.

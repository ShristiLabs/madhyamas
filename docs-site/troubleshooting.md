---
title: Troubleshooting
description: Fixes for common Madhyamas problems — certificate errors, port conflicts, database locks, no traffic captured, HTTPS interception, mobile connections, MCP server issues, and performance.
---

# Troubleshooting

This page collects the most common Madhyamas issues and how to fix them. If your problem isn't listed here, check the [GitHub issues](https://github.com/ShristiLabs/madhyamas/issues) or file a new one.

## Certificate Errors

**Symptom**: HTTPS sites show certificate warnings or fail to load through the proxy.

**Cause**: The Madhyamas CA certificate is not installed in your system or browser trust store.

**Fix**:

1. Download the CA certificate:
   ```bash
   curl http://localhost:3001/api/cert/ca -o madhyamas-ca.pem
   ```
   or find it on disk at `~/.madhyamas/certs/madhyamas-ca.pem`.
2. Install it in your trust store — see [HTTPS & Certificates](./https-certificates) for platform-specific steps.
3. On iOS, also enable trust under **Settings → General → About → Certificate Trust Settings**.

## Certificate Pinning Failures

**Symptom**: Certain apps (especially mobile apps) fail to connect. Traffic shows `502` entries with TLS handshake failure messages.

**Cause**: The app uses certificate pinning and rejects the proxy's CA certificate.

**Fix**:

- Failed TLS handshakes are recorded as `502` traffic entries with explanatory error messages, so you can confirm pinning is the cause.
- On Android, see [HTTPS & Certificates](./https-certificates) for bypass options (Frida, APK patching, Magisk modules).
- Use the Madhyamas Android companion VPN app for transparent routing.
- Some apps cannot be intercepted without modifying the app itself.

## Port Conflicts

**Symptom**: The proxy fails to start with `address already in use`.

**Cause**: Ports `8888` (proxy) or `3001` (API) are already in use by another process.

**Fix**:

```bash
# Use different ports
madhyamas serve --proxy-port 8889 --api-port 3002

# Or find and stop the process using the port
lsof -i :8888
kill <PID>
```

## Database Locked

**Symptom**: `database is locked` error when starting the proxy.

**Cause**: Another Madhyamas instance is already running and holds the SQLite database lock.

**Fix**:

1. Stop the other instance with `./stop.sh` or `./stop-local.sh`, or `pkill madhyamas`.
2. Only one instance can run at a time against the same database. To run a second instance, point it at a different `--db-path`.

## No Traffic Captured

**Symptom**: The proxy is running but no traffic appears in the UI.

**Cause**: The client isn't configured to use the proxy, or capture is in Passthrough mode.

**Fix**:

1. Verify the client's proxy settings point to `localhost:8888` (or your host IP and port).
2. Check capture status:
   ```bash
   madhyamas capture status
   ```
3. Enable recording if it's off:
   ```bash
   madhyamas capture enable
   ```
4. Note that the proxy excludes its own API traffic from capture (normal behavior).

## HTTPS Interception Not Working

**Symptom**: HTTP traffic is captured but HTTPS traffic is not.

**Cause**: HTTPS interception is disabled, or the CA certificate isn't installed.

**Fix**:

1. Check the config:
   ```bash
   madhyamas config get
   ```
   Verify `intercept_https` is `true`.
2. Enable it if needed:
   ```bash
   madhyamas config update --intercept-https true
   ```
3. Install the CA certificate (see [HTTPS & Certificates](./https-certificates)).
4. Confirm the client is configured to tunnel HTTPS through the proxy (not bypass it).

## Connection Refused from Mobile Device

**Symptom**: A phone or tablet can't connect to the proxy.

**Cause**: The proxy is bound to `127.0.0.1` (loopback only) and isn't reachable from other devices.

**Fix**:

```bash
# Bind to all interfaces
madhyamas serve --host 0.0.0.0
# Or via environment variable
MADHYAMAS_HOST=0.0.0.0 madhyamas serve
```

Then configure the mobile device's Wi-Fi proxy to use the computer's LAN IP and port `8888`. See [Mobile Setup](./mobile-setup) for full instructions.

## Web UI Not Updating

**Symptom**: The web UI shows stale traffic or doesn't update in real time.

**Cause**: The WebSocket connection dropped, or the embedded assets are out of date.

**Fix**:

1. Refresh the browser page — the WebSocket reconnects automatically.
2. If you're running a locally built binary, rebuild the frontend and binary:
   ```bash
   cd web && npm run build
   cargo build --release -p madhyamas
   ```
3. Restart the proxy.

## MCP Server Not Connecting

**Symptom**: Your AI agent can't see Madhyamas MCP tools.

**Cause**: The proxy isn't running, or the MCP config is incorrect.

**Fix**:

1. Verify the proxy is healthy:
   ```bash
   curl http://localhost:3001/api/health
   ```
2. Check the MCP config JSON syntax (a trailing comma or missing brace will silently break it).
3. Verify the binary path is correct and executable:
   ```bash
   chmod +x /path/to/madhyamas
   ```
4. Ensure `MADHYAMAS_API_URL` points to the correct API endpoint.
5. Restart your AI agent after changing the config.

See [MCP & AI Agents](./mcp) for harness-specific setup.

## MCP Tools Not Appearing

**Symptom**: The MCP server connects but no tools are listed.

**Fix**:

1. Confirm the proxy is running and healthy.
2. Enable verbose logging:
   ```bash
   RUST_LOG=debug madhyamas mcp
   ```
3. Check stderr output from the MCP server for errors.
4. Verify `MADHYAMAS_API_URL` is reachable from the MCP server process.

## Large Response Bodies Truncated

**Symptom**: Response bodies are cut off at a certain size.

**Cause**: The `max_body_size` configuration limits body capture (default ~20 MB).

**Fix**:

```bash
madhyamas config update --max-body-size 104857600   # 100 MB
```

Or via the REST API:

```bash
curl -X PATCH http://localhost:3001/api/config \
  -H 'Content-Type: application/json' \
  -d '{"max_body_size":104857600}'
```

See [Recording Limits](./recording-limits) for the full set of bounds.

## Performance Issues

**Symptom**: The proxy is slow or consumes too much memory.

**Fix**:

1. Reduce the in-memory traffic limit:
   ```bash
   madhyamas config update --max-requests 5000
   ```
2. Switch to Passthrough mode when you aren't actively debugging:
   ```bash
   madhyamas capture disable
   ```
3. Clear old traffic:
   ```bash
   madhyamas traffic clear
   ```
4. Delete old sessions from the Sessions view.
5. Inspect memory and connection stats:
   ```bash
   curl http://localhost:3001/api/health/detailed
   ```

See [Recording Limits](./recording-limits) and [Configuration](./configuration) for tuning guidance.

## Office or Corporate Network Issues

**Symptom**: Mobile devices on the same corporate Wi-Fi can't reach the proxy.

**Cause**: Corporate Wi-Fi often enables **client isolation**, which blocks devices on the same network from talking to each other.

**Fix**:

- Use a personal hotspot from your phone.
- Connect your computer via Ethernet and the phone via Wi-Fi.
- Ask your IT department to disable client isolation for your devices.

## See also

- [Getting Started](./getting-started) — installation and first run
- [HTTPS & Certificates](./https-certificates) — CA installation and pinning
- [Mobile Setup](./mobile-setup) — connecting phones and tablets
- [Configuration](./configuration) — all CLI flags and environment variables
- [MCP & AI Agents](./mcp) — AI agent integration

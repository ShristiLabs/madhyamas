# Troubleshooting

## Common Issues

### Certificate Errors

**Symptom:** HTTPS sites show certificate warnings or fail to load.

**Cause:** The Madhyamas CA certificate is not installed in the system/browser trust store.

**Solution:**
1. Download the CA cert: `curl http://localhost:3001/api/cert/ca -o madhyamas-ca.pem`
2. Or find it at `~/.madhyamas/certs/madhyamas-ca.pem`
3. Install in trust store (see [setup.md](setup.md) for platform-specific instructions)

### Certificate Pinning Failures

**Symptom:** Certain apps (especially mobile apps) fail to connect through the proxy. Traffic shows 502 errors with TLS handshake failure messages.

**Cause:** The app uses certificate pinning and rejects the proxy's CA certificate.

**Solution:**
- Failed TLS handshakes are recorded as 502 traffic entries with explanatory error messages
- For Android, see `docs/ANDROID_CERT_PINNING.md` for bypass guides (Frida, APK patching, Magisk modules)
- Use the Android companion VPN app for transparent routing
- Some apps cannot be intercepted without modifying the app itself

### Port Conflicts

**Symptom:** Proxy fails to start with "address already in use" error.

**Cause:** Ports 8888 (proxy) or 3001 (API) are already in use.

**Solution:**
```bash
# Use different ports
madhyamas serve --proxy-port 8889 --api-port 3002

# Or find and kill the process using the port
lsof -i :8888
kill <PID>
```

### Database Locked

**Symptom:** "database is locked" error when starting the proxy.

**Cause:** Another Madhyamas instance is already running and holding the SQLite database lock.

**Solution:**
1. Stop the other instance: `./stop.sh` or `./stop-local.sh`
2. Or kill the process: `pkill madhyamas`
3. Only one instance can run at a time with the same database

### Web UI Not Updating

**Symptom:** Web UI shows stale traffic or doesn't update in real-time.

**Cause:** The web UI assets may be outdated or the WebSocket connection is broken.

**Solution:**
1. Rebuild the frontend: `cd web && npm run build`
2. Rebuild the Rust binary: `cargo build --release -p madhyamas`
3. Restart the proxy
4. Refresh the browser page

### MCP Server Not Connecting

**Symptom:** AI agent can't see Madhyamas MCP tools.

**Cause:** The proxy is not running, or the MCP config is incorrect.

**Solution:**
1. Verify proxy is running: `curl http://localhost:3001/api/health`
2. Check MCP config JSON syntax
3. Verify the binary path is correct and executable: `chmod +x /path/to/madhyamas`
4. Ensure `MADHYAMAS_API_URL` points to the correct API endpoint
5. Restart your AI agent after config changes

### MCP Tools Not Appearing

**Symptom:** MCP server connects but no tools are listed.

**Solution:**
1. Check that the proxy is running and healthy
2. Set `RUST_LOG=debug` for verbose MCP server logging
3. Check stderr output from the MCP server for errors
4. Verify `MADHYAMAS_API_URL` is reachable from the MCP server process

### HTTPS Interception Not Working

**Symptom:** HTTP traffic is captured but HTTPS traffic is not.

**Cause:** HTTPS interception may be disabled, or the CA certificate is not installed.

**Solution:**
1. Check config: `madhyamas config get` — verify `intercept_https` is `true`
2. Enable if needed: `madhyamas config update --intercept-https true`
3. Install the CA certificate (see [setup.md](setup.md))
4. Verify the client is configured to use the proxy for HTTPS

### No Traffic Captured

**Symptom:** Proxy is running but no traffic appears.

**Cause:** Client may not be configured to use the proxy, or capture is in passthrough mode.

**Solution:**
1. Verify client proxy settings point to `localhost:8888`
2. Check capture status: `madhyamas capture status`
3. Enable recording: `madhyamas capture enable`
4. Verify the proxy is excluding its own API traffic (normal behavior)

### Connection Refused from Mobile Device

**Symptom:** Mobile device can't connect to the proxy.

**Cause:** Proxy is bound to `127.0.0.1` (localhost only), not accessible from other devices.

**Solution:**
```bash
# Bind to all interfaces
madhyamas serve --host 0.0.0.0
# Or set environment variable
MADHYAMAS_HOST=0.0.0.0 madhyamas serve
```

Then configure the mobile device's Wi-Fi proxy to use the machine's IP address and port 8888.

### Large Response Bodies Truncated

**Symptom:** Response bodies are cut off at a certain size.

**Cause:** The `max_body_size` configuration limits body capture (default: 20MB).

**Solution:**
```bash
madhyamas config update --max-requests 100000
# Or set via PATCH /api/config with max_body_size field
```

### Performance Issues

**Symptom:** Proxy is slow or consumes too much memory.

**Solution:**
1. Reduce `max_requests` to limit in-memory traffic: `madhyamas config update --max-requests 5000`
2. Use passthrough mode when not actively debugging: `madhyamas capture disable`
3. Clear old traffic: `madhyamas traffic clear`
4. Use sessions to organize traffic and delete old sessions
5. Check `/api/health/detailed` for memory usage and connection stats

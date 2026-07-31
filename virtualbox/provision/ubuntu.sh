#!/usr/bin/env bash
# Ubuntu/Debian guest provisioning for Madhyamas testing.
#
# Run AFTER Madhyamas is started on the host (default: 192.168.56.1).
#
# This script:
#   1. Fetches the Madhyamas CA cert from the host's API
#   2. Installs it into the system trust store
#   3. Sets http(s)_proxy system-wide via /etc/environment
#
# Usage (inside the Ubuntu guest):
#   sudo bash provision/ubuntu.sh
#
# Env vars:
#   HOST_IP    Madhyamas host IP on the host-only net (default: 192.168.56.1)
#   API_PORT   Madhyamas API port (default: 3001)
#   PROXY_PORT Madhyamas proxy port (default: 8888)
set -euo pipefail

HOST_IP="${HOST_IP:-192.168.56.1}"
API_PORT="${API_PORT:-3001}"
PROXY_PORT="${PROXY_PORT:-8888}"
API_URL="http://${HOST_IP}:${API_PORT}"
PROXY_URL="http://${HOST_IP}:${PROXY_PORT}"

if [[ $EUID -ne 0 ]]; then
  echo "error: run with sudo" >&2
  exit 1
fi

echo "==> Checking host reachability..."
if ! ping -c1 -W2 "${HOST_IP}" >/dev/null 2>&1; then
  echo "error: cannot reach host at ${HOST_IP}." >&2
  echo "       Start Madhyamas on the host, then run network.sh, then re-run this." >&2
  exit 1
fi

echo "==> Fetching Madhyamas CA cert from ${API_URL}/api/cert/ca..."
# Hit the API directly, bypassing any proxy env vars.
TMP_CERT="$(mktemp)"
if ! curl -fsS --noproxy '*' "${API_URL}/api/cert/ca" -o "${TMP_CERT}"; then
  echo "error: could not fetch CA cert from ${API_URL}/api/cert/ca" >&2
  echo "       Is Madhyamas running on the host?" >&2
  exit 1
fi

echo "==> Installing CA cert into /usr/local/share/ca-certificates/..."
# update-ca-certificates expects a .crt extension and PEM format.
install -m 0644 "${TMP_CERT}" /usr/local/share/ca-certificates/madhyamas-ca.crt
update-ca-certificates
rm -f "${TMP_CERT}"

echo "==> Configuring system-wide proxy in /etc/environment..."
# Strip any prior madhyamas proxy block.
sed -i '/# BEGIN madhyamas/,/# END madhyamas/d' /etc/environment
cat >> /etc/environment <<EOF
# BEGIN madhyamas
http_proxy="${PROXY_URL}"
https_proxy="${PROXY_URL}"
HTTP_PROXY="${PROXY_URL}"
HTTPS_PROXY="${PROXY_URL}"
no_proxy="localhost,127.0.0.1,::1,${HOST_IP}"
NO_PROXY="localhost,127.0.0.1,::1,${HOST_IP}"
# END madhyamas
EOF

echo "==> APT proxy (so apt traffic flows through Madhyamas):"
install -d -m 0755 /etc/apt/apt.conf.d
cat > /etc/apt/apt.conf.d/95madhyamas <<EOF
Acquire::http::Proxy "${PROXY_URL}";
Acquire::https::Proxy "${PROXY_URL}";
EOF

cat <<DONE

Provisioning complete.

  CA cert:    /usr/local/share/ca-certificates/madhyamas-ca.crt
  Proxy:      ${PROXY_URL}
  No-proxy:   localhost,127.0.0.1,${HOST_IP}

Next:
  - Log out and back in (or run 'set -a; source /etc/environment; set +a')
    so the proxy env vars take effect in your shell.
  - Verify with:
      curl -v https://example.com     # should appear in Madhyamas traffic list
DONE

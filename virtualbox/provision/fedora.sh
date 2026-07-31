#!/usr/bin/env bash
# Fedora guest provisioning for Madhyamas testing.
#
# Run AFTER Madhyamas is started on the host (default: 192.168.56.1).
#
# This script:
#   1. Fetches the Madhyamas CA cert from the host's API
#   2. Installs it into the system trust store (with SELinux labels)
#   3. Sets http(s)_proxy system-wide via /etc/environment
#
# Usage (inside the Fedora guest):
#   sudo bash provision/fedora.sh
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
  exit 1
fi

echo "==> Fetching Madhyamas CA cert from ${API_URL}/api/cert/ca..."
TMP_CERT="$(mktemp)"
if ! curl -fsS --noproxy '*' "${API_URL}/api/cert/ca" -o "${TMP_CERT}"; then
  echo "error: could not fetch CA cert from ${API_URL}/api/cert/ca" >&2
  exit 1
fi

echo "==> Installing CA cert into /etc/pki/ca-trust/source/anchors/..."
install -m 0644 "${TMP_CERT}" /etc/pki/ca-trust/source/anchors/madhyamas-ca.pem
# SELinux: labelled so crypto policies can read it.
restorecon -v /etc/pki/ca-trust/source/anchors/madhyamas-ca.pem >/dev/null 2>&1 || true
update-ca-trust
rm -f "${TMP_CERT}"

echo "==> Configuring system-wide proxy in /etc/environment..."
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

echo "==> DNF proxy:"
install -d -m 0755 /etc/dnf
# Add/replace proxy setting in dnf.conf without destroying existing config.
# Remove any prior proxy line, then append under a marker block.
sed -i '/^proxy=/d' /etc/dnf/dnf.conf
sed -i '/# BEGIN madhyamas/,/# END madhyamas/d' /etc/dnf/dnf.conf
cat >> /etc/dnf/dnf.conf <<EOF
# BEGIN madhyamas
proxy="${PROXY_URL}"
sslverify=1
# END madhyamas
EOF

# Fedora systemd-resolved may need to be told to use the proxy too — leave it
# off by default; DNS itself doesn't go through HTTP proxies.
cat <<DONE

Provisioning complete.

  CA cert:    /etc/pki/ca-trust/source/anchors/madhyamas-ca.pem
  Proxy:      ${PROXY_URL}
  No-proxy:   localhost,127.0.0.1,${HOST_IP}

Next:
  - Log out and back in (or run 'set -a; source /etc/environment; set +a').
  - Verify with:
      curl -v https://example.com     # should appear in Madhyamas traffic list
DONE

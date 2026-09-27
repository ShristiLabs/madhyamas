#!/usr/bin/env bash
# Issue/renew the TLS certificate for the public proxy listener hostname
# via HTTP-01 (served by Caddy from the shared acme-web webroot), install
# it into certs/proxy-tls/, and restart the madhyamas container so the
# TLS listener reloads it. Safe to run repeatedly — cron it weekly:
#   0 3 * * 1  $HOME/madhyamas/deploy/demo/certbot-proxy.sh
set -euo pipefail
cd "$(dirname "$0")"

DOMAIN="${MADHYAMAS_PROXY_DOMAIN:-madhyamas-proxy.shristilabs.com}"
EMAIL="${CERTBOT_EMAIL:?set CERTBOT_EMAIL in .env or the environment}"
LE_DIR="$PWD/letsencrypt"
WEBROOT="$PWD/acme-web"

mkdir -p "$LE_DIR" "$WEBROOT"

docker run --rm \
  -v "$LE_DIR:/etc/letsencrypt" \
  -v "$WEBROOT:/var/www/acme" \
  certbot/certbot certonly --webroot -w /var/www/acme \
  -d "$DOMAIN" --email "$EMAIL" --agree-tos --non-interactive \
  --keep-until-expiring

mkdir -p certs/proxy-tls
install -m 644 "$LE_DIR/live/$DOMAIN/fullchain.pem" certs/proxy-tls/fullchain.pem
install -m 644 "$LE_DIR/live/$DOMAIN/privkey.pem" certs/proxy-tls/privkey.pem

docker compose restart madhyamas >/dev/null 2>&1 || true
echo "proxy TLS certificate installed for $DOMAIN"

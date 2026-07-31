#!/usr/bin/env bash
# Sets up the VirtualBox host-only network used by all Madhyamas test VMs.
#
# Creates vboxnet0 on 192.168.56.0/24 with the host at 192.168.56.1 and a DHCP
# server handing out 192.168.56.101-192.168.56.254. Guests reach the host's
# Madhyamas proxy at http://192.168.56.1:8888.
#
# Idempotent: safe to run repeatedly.
set -euo pipefail

NET_NAME="vboxnet0"
NET_ADDR="192.168.56.0/24"
NET_MASK="255.255.255.0"
HOST_IP="192.168.56.1"
DHCP_LOWER="192.168.56.101"
DHCP_UPPER="192.168.56.254"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
. "${SCRIPT_DIR}/common.sh"
vbox_preflight

# Create the host-only interface if it does not exist.
if ! VBoxManage list hostonlyifs | grep -q "^Name:.*${NET_NAME}"; then
  echo "Creating host-only interface ${NET_NAME}..."
  # NOTE: do not suppress output — failures here are usually environmental
  # (kext not loaded, permission denied) and the raw message is the best
  # diagnostic the user can get.
  VBoxManage hostonlyif create
fi

echo "Configuring ${NET_NAME} -> ${HOST_IP}/${NET_MASK}..."
VBoxManage hostonlyif ipconfig "${NET_NAME}" \
  --ip "${HOST_IP}" \
  --netmask "${NET_MASK}"

# Add (or recreate) the DHCP server for this interface.
if VBoxManage list dhcpservers | grep -q "^NetworkName:.*HostInterfaceNetworking-${NET_NAME}"; then
  echo "DHCP server already present on ${NET_NAME}, reconfiguring..."
  VBoxManage dhcpserver modify \
    --ifname "${NET_NAME}" \
    --ip "${HOST_IP}" \
    --netmask "${NET_MASK}" \
    --lowerip "${DHCP_LOWER}" \
    --upperip "${DHCP_UPPER}" \
    --enable
else
  echo "Adding DHCP server (${DHCP_LOWER}-${DHCP_UPPER})..."
  VBoxManage dhcpserver add \
    --ifname "${NET_NAME}" \
    --ip "${HOST_IP}" \
    --netmask "${NET_MASK}" \
    --lowerip "${DHCP_LOWER}" \
    --upperip "${DHCP_UPPER}" \
    --enable
fi

echo
echo "Host-only network ready."
echo "  Network:    ${NET_ADDR}"
echo "  Host IP:    ${HOST_IP}   <- point guests' proxy at this address"
echo "  DHCP range: ${DHCP_LOWER} - ${DHCP_UPPER}"
echo "  Proxy URL:  http://${HOST_IP}:8888"
echo "  API URL:    http://${HOST_IP}:3001"

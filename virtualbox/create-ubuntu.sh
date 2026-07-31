#!/usr/bin/env bash
# Creates an Ubuntu 24.04 LTS VM preconfigured for Madhyamas proxy testing.
#
# Usage:
#   UBUNTU_ISO=/path/to/ubuntu-24.04.iso ./create-ubuntu.sh [--install]
#
#   --install    Run VBoxManage unattended install after creating the VM.
#                Without it, the VM is created with the ISO attached and you
#                walk through the installer by hand on first boot.
#
# Env vars (all optional):
#   UBUNTU_ISO      Path to the Ubuntu 24.04 ISO. Required for --install.
#   VM_DIR          Where to place the VM (default: ~/VirtualBox VMs/madhyamas)
#   VM_NAME         VM name (default: madhyama-ubuntu)
#   VM_RAM_MB       RAM in MB (default: 2048)
#   VM_CPUS         vCPU count (default: 2)
#   VM_DISK_GB      Disk size in GB (default: 25)
#   GUEST_USER      Username for unattended install (default: madhyama)
#   GUEST_PASSWORD  Password for unattended install (default: madhyama)
set -euo pipefail

VM_NAME="${VM_NAME:-madhyama-ubuntu}"
VM_DIR="${VM_DIR:-$HOME/VirtualBox VMs/madhyamas}"
VM_RAM_MB="${VM_RAM_MB:-2048}"
VM_CPUS="${VM_CPUS:-2}"
VM_DISK_GB="${VM_DISK_GB:-25}"
GUEST_USER="${GUEST_USER:-madhyama}"
GUEST_PASSWORD="${GUEST_PASSWORD:-madhyama}"
OS_TYPE="Ubuntu_64"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
. "${SCRIPT_DIR}/common.sh"
vbox_preflight

if vm_exists "${VM_NAME}"; then
  echo "error: VM '${VM_NAME}' already exists. Delete it first:" >&2
  echo "  VBoxManage unregistervm '${VM_NAME}' --delete" >&2
  exit 1
fi

mkdir -p "${VM_DIR}"

echo "Creating ${VM_NAME} in ${VM_DIR}..."
VBoxManage createvm \
  --name "${VM_NAME}" \
  --ostype "${OS_TYPE}" \
  --basefolder "${VM_DIR}" \
  --register

VBoxManage createmedium \
  --filename "${VM_DIR}/${VM_NAME}/${VM_NAME}.vdi" \
  --size $((VM_DISK_GB * 1024)) \
  --variant Standard

# Hardware
VBoxManage modifyvm "${VM_NAME}" \
  --memory "${VM_RAM_MB}" \
  --cpus "${VM_CPUS}" \
  --ioapic on \
  --rtc-use-utc on \
  --graphicscontroller vmsvga \
  --vram 64 \
  --nic1 nat \
  --nictype1 virtio \
  --nic2 hostonly \
  --hostonlyadapter2 vboxnet0 \
  --nictype2 virtio \
  --natdnshostresolver1 on

# Storage: SATA controller + DVD
VBoxManage storagectl "${VM_NAME}" --name SATA --add sata --portcount 4 --bootable on
VBoxManage storageattach "${VM_NAME}" \
  --storagectl SATA --port 0 --device 0 \
  --type hdd --medium "${VM_DIR}/${VM_NAME}/${VM_NAME}.vdi"

# Boot order
VBoxManage modifyvm "${VM_NAME}" --boot1 dvd --boot2 disk --boot3 none --boot4 none

# Optional unattended install. If --install is omitted but UBUNTU_ISO is set,
# we still attach the ISO so a manual install works on first boot.
if [[ "${1:-}" == "--install" ]]; then
  if [[ -z "${UBUNTU_ISO:-}" || ! -f "${UBUNTU_ISO}" ]]; then
    echo "error: --install requires UBUNTU_ISO to point at the Ubuntu 24.04 ISO." >&2
    exit 1
  fi
  echo "Starting unattended install (this also attaches the ISO)..."
  VBoxManage unattended install "${VM_NAME}" \
    --iso="${UBUNTU_ISO}" \
    --user="${GUEST_USER}" \
    --user-password="${GUEST_PASSWORD}" \
    --full-user-name="Madhyama Tester" \
    --install-additions \
    --locale="en_US" \
    --time-zone="UTC"
elif [[ -n "${UBUNTU_ISO:-}" && -f "${UBUNTU_ISO}" ]]; then
  echo "Attaching ISO ${UBUNTU_ISO} (manual install)..."
  VBoxManage storageattach "${VM_NAME}" \
    --storagectl SATA --port 1 --device 0 \
    --type dvddrive --medium "${UBUNTU_ISO}"
else
  echo "UBUNTU_ISO not set; no ISO attached."
  echo "Attach one before first boot:"
  echo "  VBoxManage storageattach '${VM_NAME}' --storagectl SATA --port 1 --device 0 --type dvddrive --medium /path/to/ubuntu.iso"
fi

echo
echo "VM '${VM_NAME}' created."
echo "Start with:  VBoxManage startvm '${VM_NAME}' --type gui"
echo "Proxy (set inside guest once Madhyamas is running on host):"
echo "  export http_proxy=http://192.168.56.1:8888"
echo "  export https_proxy=http://192.168.56.1:8888"

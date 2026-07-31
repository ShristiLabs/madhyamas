#!/usr/bin/env bash
# Creates a Windows 11 VM preconfigured for Madhyamas proxy testing.
#
# Usage:
#   WIN_ISO=/path/to/Win11_English_x64.iso ./create-windows.sh [--install]
#
#   --install    Run VBoxManage unattended install after creating the VM.
#                Requires a product key in WIN_KEY (use "0000-0000-..." placeholder
#                if you want the unattended install to skip activation).
#
# Env vars:
#   WIN_ISO         Path to the Windows 11 ISO. Required for --install.
#   WIN_KEY         Product key (optional; default skips activation).
#   VM_DIR          Where to place the VM (default: ~/VirtualBox VMs/madhyamas)
#   VM_NAME         VM name (default: madhyama-win11)
#   VM_RAM_MB       RAM in MB (default: 4096 — Win11 minimum)
#   VM_CPUS         vCPU count (default: 2 — Win11 minimum)
#   VM_DISK_GB      Disk size in GB (default: 40)
#   GUEST_USER      Username for unattended install (default: madhyama)
#   GUEST_PASSWORD  Password for unattended install (default: madhyama-Pa55w0rd)
set -euo pipefail

VM_NAME="${VM_NAME:-madhyama-win11}"
VM_DIR="${VM_DIR:-$HOME/VirtualBox VMs/madhyamas}"
VM_RAM_MB="${VM_RAM_MB:-4096}"
VM_CPUS="${VM_CPUS:-2}"
VM_DISK_GB="${VM_DISK_GB:-40}"
GUEST_USER="${GUEST_USER:-madhyama}"
GUEST_PASSWORD="${GUEST_PASSWORD:-madhyama-Pa55w0rd}"
OS_TYPE="Windows11_64"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
. "${SCRIPT_DIR}/common.sh"
vbox_preflight

if vm_exists "${VM_NAME}"; then
  echo "error: VM '${VM_NAME}' already exists." >&2
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

# Windows 11 requirements: UEFI firmware + TPM 2.0. Secure Boot is not enabled
# here because it requires separate key enrollment via `VBoxManage modifynvram`;
# the unattended installer applies the SecureBootCheck bypass anyway, so it
# isn't needed for install to proceed.
#
# NIC type: 82540EM (Intel e1000) instead of virtio. Windows has no built-in
# virtio driver, so virtio would leave the guest without networking until
# drivers are installed manually. e1000 works out of the box.
VBoxManage modifyvm "${VM_NAME}" \
  --memory "${VM_RAM_MB}" \
  --cpus "${VM_CPUS}" \
  --ioapic on \
  --rtc-use-utc on \
  --graphicscontroller vmsvga \
  --vram 128 \
  --firmware efi \
  --tpm-type 2.0 \
  --nic1 nat \
  --nictype1 82540EM \
  --nic2 hostonly \
  --hostonlyadapter2 vboxnet0 \
  --nictype2 82540EM \
  --natdnshostresolver1 on

VBoxManage storagectl "${VM_NAME}" --name SATA --add sata --portcount 4 --bootable on
VBoxManage storageattach "${VM_NAME}" \
  --storagectl SATA --port 0 --device 0 \
  --type hdd --medium "${VM_DIR}/${VM_NAME}/${VM_NAME}.vdi"

# Point at the VirtualBox Guest Additions ISO if present (not required for --install).
GA_ISO="${GA_ISO:-}"
if [[ -n "${GA_ISO}" && -f "${GA_ISO}" ]]; then
  VBoxManage storageattach "${VM_NAME}" \
    --storagectl SATA --port 2 --device 0 \
    --type dvddrive --medium "${GA_ISO}"
fi

VBoxManage modifyvm "${VM_NAME}" --boot1 dvd --boot2 disk --boot3 none --boot4 none

if [[ "${1:-}" == "--install" ]]; then
  if [[ -z "${WIN_ISO:-}" || ! -f "${WIN_ISO}" ]]; then
    echo "error: --install requires WIN_ISO to point at the Windows 11 ISO." >&2
    exit 1
  fi
  # VirtualBox unattended install for Win11 applies the official hardware-check
  # bypass (BypassTPMCheck / BypassRAMCheck / BypassSecureBootCheck) so the
  # installer proceeds without complaining about the VM spec.
  UNATTENDED_ARGS=(
    --iso="${WIN_ISO}"
    --user="${GUEST_USER}"
    --user-password="${GUEST_PASSWORD}"
    --full-user-name="Madhyama Tester"
    --install-additions
    --locale="en-US"
    --time-zone="UTC"
  )
  if [[ -n "${WIN_KEY:-}" ]]; then
    UNATTENDED_ARGS+=("--key=${WIN_KEY}")
  fi
  echo "Starting unattended install (this also attaches the ISO)..."
  VBoxManage unattended install "${VM_NAME}" "${UNATTENDED_ARGS[@]}"
elif [[ -n "${WIN_ISO:-}" && -f "${WIN_ISO}" ]]; then
  echo "Attaching ISO ${WIN_ISO} (manual install)..."
  VBoxManage storageattach "${VM_NAME}" \
    --storagectl SATA --port 1 --device 0 \
    --type dvddrive --medium "${WIN_ISO}"
else
  echo "WIN_ISO not set; no ISO attached."
  echo "Attach one before first boot:"
  echo "  VBoxManage storageattach '${VM_NAME}' --storagectl SATA --port 1 --device 0 --type dvddrive --medium /path/to/win11.iso"
fi

echo
echo "VM '${VM_NAME}' created."
echo "Start with:  VBoxManage startvm '${VM_NAME}' --type gui"
echo "After install, run provision/windows.ps1 inside the guest to configure the proxy."

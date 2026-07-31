#!/usr/bin/env bash
# Shared helpers for the virtualbox/ scripts. Sourced, not executed.
# Do NOT `set -e` here — sourcing scripts set their own options.

# vbox_preflight: detect "VBoxManage binary present but kernel modules missing".
# This is a common macOS state when VirtualBox is installed via the brew formula
# (no kext) instead of the cask, or when macOS blocked the kext after install.
# Without this check, commands like `hostonlyif create` fail with a cryptic
# "/dev/vboxnetctl: No such file or directory" message.
vbox_preflight() {
  command -v VBoxManage >/dev/null 2>&1 || {
    echo "error: VBoxManage not found in PATH. Install VirtualBox first." >&2
    exit 1
  }

  if [[ "$(uname -s)" != "Darwin" ]]; then
    return 0
  fi

  if [[ ! -c /dev/vboxnetctl && ! -c /dev/vboxdrv ]]; then
    cat >&2 <<'EOF'
error: VirtualBox kernel modules are not loaded.

The VBoxManage binary is present but the kernel extensions
(/dev/vboxnetctl, /dev/vboxdrv) are missing. On macOS this means:

  1. VirtualBox was installed via the brew *formula*, not the cask:
       brew uninstall virtualbox
       brew install --cask virtualbox

  2. Or macOS blocked the kernel extension after install. Approve it in
       System Settings > Privacy & Security >
         "Allow system software from developer Oracle"
     then reboot.

Verify the fix with:
  ls /dev/vboxnetctl /dev/vboxdrv

EOF
    exit 1
  fi
}

# vm_exists: 0 if a VM with the given name is registered, 1 otherwise.
vm_exists() {
  VBoxManage showvminfo "$1" >/dev/null 2>&1
}

# Madhyamas VirtualBox Test VMs

Preconfigured VirtualBox VMs for testing the Madhyamas HTTP/HTTPS proxy on
Windows 11, Ubuntu 24.04 LTS, and Fedora 40. Each VM is wired to a host-only
network so its traffic flows through a Madhyamas instance running on the host.

```
                    +-------------------+
                    |  Host (macOS/Win/Linux)
                    |  madhyamas serve  |   <-- proxy :8888, API :3001
                    +---------+---------+
                              |  vboxnet0  192.168.56.0/24
                              |  host = 192.168.56.1
              +---------------+---------------+
              |               |               |
        +-----+-----+   +-----+-----+   +-----+-----+
        | Ubuntu    |   | Fedora    |   | Windows   |
        | 24.04 LTS |   | 40        |   | 11        |
        +-----------+   +-----------+   +-----------+
              http(s)_proxy = http://192.168.56.1:8888
              CA cert installed in trust store
```

## Prerequisites

1. **VirtualBox 7.0+** — <https://www.virtualbox.org/wiki/Downloads>
   - Also install the **VirtualBox Extension Pack** (for TPM 2.0 / Win11).
2. **~10 GB free disk** per VM.
3. **Madhyamas** running on the host (built from this repo, or installed via the
   instructions in [`packaging/README.md`](../packaging/README.md)).
4. **ISOs** for the guests you want to test (see below). Download paths are
   passed to the create scripts via env vars.

### ISO download links

| Guest | ISO | Source |
|-------|-----|--------|
| Ubuntu 24.04 LTS | `ubuntu-24.04.x-desktop-amd64.iso` | <https://releases.ubuntu.com/24.04/> |
| Fedora 40 | `Fedora-Workstation-Live-x86_64-40.iso` | <https://fedoraproject.org/workstation/download/> |
| Windows 11 | `Win11_English_x64.iso` (or local equivalent) | <https://www.microsoft.com/software-download/windows11> |

VirtualBox will also want the **Guest Additions ISO** (`VBoxGuestAdditions.iso`)
for the `--install` (unattended) path. It usually lives in
`/Applications/VirtualBox.app/Contents/MacOS/VBoxGuestAdditions.iso` on macOS.

## Quick start

From this directory:

```bash
# 1. Create the host-only network (run once)
./network.sh

# 2. Start Madhyamas on the host (must listen on the host-only IP)
#    In another terminal, from the repo root:
#      madhyamas serve --host 0.0.0.0 --public-ip 192.168.56.1

# 3. Create a VM (pick one)
UBUNTU_ISO=/path/to/ubuntu-24.04.iso ./create-ubuntu.sh --install
FEDORA_ISO=/path/to/Fedora-Workstation-Live-x86_64-40.iso ./create-fedora.sh --install
WIN_ISO=/path/to/Win11_English_x64.iso GA_ISO=/path/to/VBoxGuestAdditions.iso ./create-windows.sh --install

# 4. Boot it
VBoxManage startvm madhyama-ubuntu --type gui   # or -fedora / -win11
```

Once the guest has booted and you've logged in, run the matching provision
script inside the guest to install the Madhyamas CA cert and configure the
system-wide proxy.

## Provisioning a guest

The provision scripts are designed to run **inside the guest**. Easiest path:
share this directory with the guest via a VirtualBox shared folder, or `curl`
the file from the host once Madhyamas is proxying traffic.

```bash
# Ubuntu (inside the guest, with sudo)
sudo bash provision/ubuntu.sh

# Fedora (inside the guest, with sudo)
sudo bash provision/fedora.sh
```

```powershell
# Windows 11 (in an elevated PowerShell inside the guest)
Set-ExecutionPolicy -Scope Process Bypass -Force
.\provision\windows.ps1
```

Each script will:

1. Ping the host at `192.168.56.1` to confirm reachability.
2. Fetch the Madhyamas CA cert from `http://192.168.56.1:3001/api/cert/ca`.
3. Install it into the guest's trust store.
4. Set `http_proxy` / `https_proxy` system-wide so all subsequent traffic flows
   through Madhyamas.

After provisioning, open a **new** shell in the guest and run any HTTP/HTTPS
request — it should show up in the Madhyamas web UI:

```bash
curl -v https://example.com
```

## File map

```
virtualbox/
├── README.md              this file
├── network.sh             creates the 192.168.56.0/24 host-only network
├── create-ubuntu.sh       Ubuntu 24.04 LTS VM + optional unattended install
├── create-fedora.sh       Fedora 40 VM + optional unattended install
├── create-windows.sh      Windows 11 VM (UEFI + TPM 2.0) + optional unattended
└── provision/
    ├── ubuntu.sh          guest: fetch CA cert + set system proxy
    ├── fedora.sh          guest: fetch CA cert + set system proxy
    └── windows.ps1        guest: fetch CA cert + set system proxy
```

## Customisation

All `create-*.sh` scripts accept the same env vars. Defaults in parentheses:

| Var | Default | Notes |
|-----|---------|-------|
| `VM_DIR` | `~/VirtualBox VMs/madhyamas` | Where the .vbox + .vdi live |
| `VM_NAME` | `madhyama-{ubuntu,fedora,win11}` | VirtualBox VM name |
| `VM_RAM_MB` | `2048` (Linux), `4096` (Win11) | Memory in MB |
| `VM_CPUS` | `2` | vCPU count |
| `VM_DISK_GB` | `25` (Linux), `40` (Win11) | Disk size in GB |
| `GUEST_USER` | `madhyama` | Unattended-install username |
| `GUEST_PASSWORD` | `madhyama` (Linux), `madhyama-Pa55w0rd` (Win11) | Unattended password |
| `WIN_KEY` | unset | Windows product key (skip to defer activation) |

Provision scripts accept `HOST_IP` / `API_PORT` / `PROXY_PORT` if your
network differs from the defaults.

## Running Madhyamas on the host

The proxy must be reachable from `vboxnet0`. Two ways:

```bash
# A. Bind to all interfaces (simplest)
madhyamas serve --host 0.0.0.0 --public-ip 192.168.56.1

# B. Bind only to the host-only IP
madhyamas serve --host 192.168.56.1 --public-ip 192.168.56.1
```

The `--public-ip` flag is just for display in the UI; the bind address is what
actually controls reachability.

## Teardown

```bash
# Stop and remove one VM
VBoxManage controlvm madhyama-ubuntu poweroff
VBoxManage unregistervm madhyama-ubuntu --delete

# Remove the host-only network (do this last)
VBoxManage dhcpserver remove --ifname vboxnet0
VBoxManage hostonlyif remove vboxnet0
```

## Troubleshooting

**`hostonlyif create` fails with "failed to open /dev/vboxnetctl"**

The VBoxManage binary is present but the kernel modules aren't loaded. On macOS
this means VirtualBox was installed via the brew **formula** (which doesn't ship
the kext) instead of the **cask**, or macOS blocked the kext on first launch.

```bash
# Check
ls /dev/vboxnetctl /dev/vboxdrv     # both must exist as char devices

# Fix option A: reinstall via cask
brew uninstall virtualbox
brew install --cask virtualbox

# Fix option B: approve the blocked kext
#   System Settings > Privacy & Security >
#     "Allow system software from developer Oracle"
# then reboot.
```

The preflight check in `common.sh` catches this before any VM is touched.

**Guest cannot reach `192.168.56.1`**
- Run `./network.sh` and confirm `vboxnet0` is in `VBoxManage list hostonlyifs`.
- Check the VM has a NIC2 set to host-only: `VBoxManage showvminfo <name> | grep -A2 "NIC 2"`.
- macOS hosts: allow VirtualBox through the firewall in *System Settings → Network → Firewall*.

**CA cert install reports "no such file"**
- Madhyamas generates its CA on first run. Make sure it actually ran at least once
  on the host before provisioning the guest. The CA lives in `~/.madhyamas/certs/`.

**Win11 installer complains about TPM / RAM**
- The `--install` flag applies the standard registry bypasses, but if you boot
  the ISO manually, you must press **Shift+F10** at the region screen and run:
  ```
  reg add "HKLM\System\Setup\LabConfig" /v BypassTPMCheck /t REG_DWORD /d 1 /f
  reg add "HKLM\System\Setup\LabConfig" /v BypassRAMCheck /t REG_DWORD /d 1 /f
  reg add "HKLM\System\Setup\LabConfig" /v BypassSecureBootCheck /t REG_DWORD /d 1 /f
  ```
- Make sure the Extension Pack is installed (provides TPM 2.0).

**HTTPS still warns about cert in browser**
- Browsers (Chrome, Firefox) often maintain their own trust stores. Firefox:
  *Settings → Privacy & Security → Certificates → View Certificates → Authorities → Import*.
  Chrome on Linux/Windows inherits the OS store, so a restart is usually enough.

**Proxy works for curl but not for the browser**
- Some apps ignore `http_proxy` env vars. Use the OS proxy settings UI
  (Windows: *Settings → Network → Proxy*; GNOME: *Settings → Network → Proxy*).

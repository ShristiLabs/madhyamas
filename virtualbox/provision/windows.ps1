# Windows 11 guest provisioning for Madhyamas testing.
#
# Run AFTER Madhyamas is started on the host (default host IP: 192.168.56.1).
#
# This script:
#   1. Fetches the Madhyamas CA cert from the host's API
#   2. Installs it into the LocalMachine Root store
#   3. Sets WinHTTP proxy (used by Windows Update, etc.) system-wide
#   4. Sets per-user proxy env vars (http_proxy / https_proxy)
#
# Usage (in an elevated PowerShell inside the guest):
#   Set-ExecutionPolicy -Scope Process Bypass -Force
#   .\provision\windows.ps1
#
# Override defaults:
#   .\provision\windows.ps1 -HostIp 192.168.56.1 -ApiPort 3001 -ProxyPort 8888

[CmdletBinding()]
param(
  [string]$HostIp    = $env:MADHYAMAS_HOST_IP,
  [string]$ApiPort   = "3001",
  [string]$ProxyPort = "8888"
)

if (-not $HostIp) { $HostIp = "192.168.56.1" }

$ErrorActionPreference = "Stop"

# --- Require admin -----------------------------------------------------------
$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
  Write-Error "Run PowerShell as Administrator."
  exit 1
}

$apiUrl   = "http://${HostIp}:${ApiPort}"
$proxyUrl = "http://${HostIp}:${ProxyPort}"

# --- Reachability ------------------------------------------------------------
Write-Host "==> Checking host reachability at ${HostIp}..."
if (-not (Test-Connection -ComputerName $HostIp -Count 1 -Quiet)) {
  Write-Error "Cannot reach host at ${HostIp}. Start Madhyamas on the host first."
}

# --- Fetch CA cert -----------------------------------------------------------
# Use curl.exe (bundled with Windows 10 1803+) instead of Invoke-WebRequest.
# IWR's -Proxy $null does NOT bypass the system proxy in Windows PowerShell 5.1,
# and -NoProxy is PowerShell 7+ only. curl's --noproxy '*' reliably bypasses.
Write-Host "==> Fetching Madhyamas CA cert from ${apiUrl}/api/cert/ca..."
$tmpCert = Join-Path $env:TEMP "madhyamas-ca.cer"
$curlArgs = @("-fsSL", "--noproxy", "*", "-o", $tmpCert, "${apiUrl}/api/cert/ca")
& curl.exe @curlArgs
if ($LASTEXITCODE -ne 0) {
  Write-Error "CA cert download failed (curl exit $LASTEXITCODE). Is Madhyamas running on the host?"
}

# --- Install into Root store -------------------------------------------------
Write-Host "==> Installing CA cert into LocalMachine\Root..."
$import = Import-Certificate -FilePath $tmpCert -CertStoreLocation Cert:\LocalMachine\Root
Write-Host "    Thumbprint: $($import.Thumbprint)"

# --- WinHTTP proxy (system services, Windows Update) -------------------------
Write-Host "==> Setting WinHTTP proxy to ${proxyUrl}..."
netsh winhttp set proxy proxy-server="${proxyUrl}" bypass-list="localhost;127.0.0.1;${HostIp}"

# --- Per-user proxy env vars (machine-wide) ----------------------------------
Write-Host "==> Setting machine-wide http_proxy / https_proxy env vars..."
[Environment]::SetEnvironmentVariable("http_proxy",  $proxyUrl, "Machine")
[Environment]::SetEnvironmentVariable("https_proxy", $proxyUrl, "Machine")
[Environment]::SetEnvironmentVariable("HTTP_PROXY",  $proxyUrl, "Machine")
[Environment]::SetEnvironmentVariable("HTTPS_PROXY", $proxyUrl, "Machine")
$noProxy = "localhost;127.0.0.1;::1;${HostIp}"
[Environment]::SetEnvironmentVariable("no_proxy", $noProxy, "Machine")
[Environment]::SetEnvironmentVariable("NO_PROXY", $noProxy, "Machine")

Remove-Item $tmpCert -Force

Write-Host ""
Write-Host "Provisioning complete."
Write-Host "  CA cert:     LocalMachine\Root (thumbprint $($import.Thumbprint))"
Write-Host "  WinHTTP:     ${proxyUrl}"
Write-Host "  No-proxy:    ${noProxy}"
Write-Host ""
Write-Host "Open a NEW shell (so env vars refresh), then verify with:"
Write-Host "  curl.exe -v https://example.com"
Write-Host "  -> request should appear in Madhyamas traffic list"

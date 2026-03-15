$ErrorActionPreference = 'Stop'

$packageName = 'madhyamas'
$toolsDir = "$(Split-Path -Parent $MyInvocation.MyCommand.Definition)"

# Remove shim
Uninstall-BinFile -Name "madhyamas"

# Optionally remove web assets (keep user data)
$webDest = Join-Path $env:ProgramData "Madhyamas\web"
if (Test-Path $webDest) {
    Remove-Item -Path $webDest -Recurse -Force
}

Write-Host "Madhyamas has been uninstalled." -ForegroundColor Yellow
Write-Host "User data in %USERPROFILE%\.madhyamas has been preserved."

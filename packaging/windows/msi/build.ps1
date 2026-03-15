# Madhyamas MSI Build Script
# Requires: WiX Toolset v4+ (https://wixtoolset.org/)
#
# Usage: .\build.ps1 -Version "0.1.0" -SourceDir "path\to\binaries"

param(
    [Parameter(Mandatory=$true)]
    [string]$Version,
    
    [Parameter(Mandatory=$true)]
    [string]$SourceDir,
    
    [string]$OutputDir = ".\output"
)

$ErrorActionPreference = "Stop"

Write-Host "Building Madhyamas MSI Installer v$Version" -ForegroundColor Cyan

# Ensure output directory exists
if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir | Out-Null
}

# Update version in WXS file
$wxsContent = Get-Content "madhyamas.wxs" -Raw
$wxsContent = $wxsContent -replace 'Version="[^"]*"', "Version=`"$Version`""
$wxsContent | Set-Content "madhyamas.wxs"

# Build MSI
Write-Host "Compiling WiX source..." -ForegroundColor Yellow
wix build madhyamas.wxs `
    -d SourceDir="$SourceDir" `
    -d Version="$Version" `
    -o "$OutputDir\madhyamas-$Version-x64.msi"

if ($LASTEXITCODE -eq 0) {
    Write-Host "MSI built successfully: $OutputDir\madhyamas-$Version-x64.msi" -ForegroundColor Green
} else {
    Write-Host "MSI build failed!" -ForegroundColor Red
    exit 1
}

# Generate checksum
$hash = Get-FileHash "$OutputDir\madhyamas-$Version-x64.msi" -Algorithm SHA256
$hash.Hash | Out-File "$OutputDir\madhyamas-$Version-x64.msi.sha256"
Write-Host "SHA256: $($hash.Hash)" -ForegroundColor Gray

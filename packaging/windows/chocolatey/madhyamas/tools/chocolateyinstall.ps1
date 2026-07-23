$ErrorActionPreference = 'Stop'

$packageName = 'madhyamas'
$toolsDir = "$(Split-Path -Parent $MyInvocation.MyCommand.Definition)"

$packageArgs = @{
    packageName    = $packageName
    unzipLocation  = $toolsDir
    url64bit       = 'https://github.com/ShristiLabs/madhyamas/releases/download/v__VERSION__/madhyamas-v__VERSION__-x86_64-pc-windows-msvc.zip'
    checksum64     = '__CHECKSUM__'
    checksumType64 = 'sha256'
}

Install-ChocolateyZipPackage @packageArgs

# Create shim for the executable
$exePath = Join-Path $toolsDir "madhyamas.exe"
Install-BinFile -Name "madhyamas" -Path $exePath

# Install web assets to program data
$webSource = Join-Path $toolsDir "web"
$webDest = Join-Path $env:ProgramData "Madhyamas\web"
if (Test-Path $webSource) {
    if (-not (Test-Path $webDest)) {
        New-Item -ItemType Directory -Path $webDest -Force | Out-Null
    }
    Copy-Item -Path "$webSource\*" -Destination $webDest -Recurse -Force
}

Write-Host ""
Write-Host "Madhyamas has been installed!" -ForegroundColor Green
Write-Host ""
Write-Host "To start the proxy server:"
Write-Host "  madhyamas start"
Write-Host ""
Write-Host "Web UI: http://localhost:3000"
Write-Host "Proxy:  localhost:8888"
Write-Host ""

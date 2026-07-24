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

# The zip extracts to a subdirectory (madhyamas-v<version>-x86_64-pc-windows-msvc/)
# Find the actual exe path and create the shim
$exePath = Get-ChildItem -Path $toolsDir -Recurse -Filter "madhyamas.exe" | Select-Object -First 1 -ExpandProperty FullName
if (-not $exePath) {
    throw "madhyamas.exe not found after extraction in $toolsDir"
}
Install-BinFile -Name "madhyamas" -Path $exePath

Write-Host ""
Write-Host "Madhyamas has been installed!" -ForegroundColor Green
Write-Host ""
Write-Host "To start the proxy server:"
Write-Host "  madhyamas"
Write-Host ""
Write-Host "Web UI: http://localhost:3001"
Write-Host "Proxy:  localhost:8888"
Write-Host ""

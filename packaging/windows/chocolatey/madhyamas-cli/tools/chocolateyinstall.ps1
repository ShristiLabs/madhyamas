$ErrorActionPreference = 'Stop'

$packageName = 'madhyamas-cli'
$toolsDir = "$(Split-Path -Parent $MyInvocation.MyCommand.Definition)"

$packageArgs = @{
    packageName    = $packageName
    unzipLocation  = $toolsDir
    url64bit       = 'https://github.com/madhyamas/madhyamas/releases/download/v__VERSION__/madhyamas-cli-v__VERSION__-x86_64-pc-windows-msvc.zip'
    checksum64     = '__CHECKSUM__'
    checksumType64 = 'sha256'
}

Install-ChocolateyZipPackage @packageArgs

# Create shim for the executable
$exePath = Join-Path $toolsDir "madhyamas-cli.exe"
Install-BinFile -Name "madhyamas-cli" -Path $exePath
Install-BinFile -Name "pf" -Path $exePath  # Shorthand alias

Write-Host ""
Write-Host "Madhyamas CLI has been installed!" -ForegroundColor Green
Write-Host ""
Write-Host "Usage:"
Write-Host "  madhyamas-cli <command> [options]"
Write-Host "  pf <command> [options]  # shorthand"
Write-Host ""

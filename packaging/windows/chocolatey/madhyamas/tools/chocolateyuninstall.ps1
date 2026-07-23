$ErrorActionPreference = 'Stop'

$packageName = 'madhyamas'
$toolsDir = "$(Split-Path -Parent $MyInvocation.MyCommand.Definition)"

# Remove shim
Uninstall-BinFile -Name "madhyamas"

Write-Host "Madhyamas has been uninstalled." -ForegroundColor Yellow
Write-Host "User data in %USERPROFILE%\.madhyamas has been preserved."

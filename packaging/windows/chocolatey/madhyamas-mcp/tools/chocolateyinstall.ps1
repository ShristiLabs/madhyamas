$ErrorActionPreference = 'Stop'

$packageName = 'madhyamas-mcp'
$toolsDir = "$(Split-Path -Parent $MyInvocation.MyCommand.Definition)"

$packageArgs = @{
    packageName    = $packageName
    unzipLocation  = $toolsDir
    url64bit       = 'https://github.com/madhyamas/madhyamas/releases/download/v__VERSION__/madhyamas-mcp-v__VERSION__-x86_64-pc-windows-msvc.zip'
    checksum64     = '__CHECKSUM__'
    checksumType64 = 'sha256'
}

Install-ChocolateyZipPackage @packageArgs

# Create shim for the executable
$exePath = Join-Path $toolsDir "madhyamas-mcp.exe"
Install-BinFile -Name "madhyamas-mcp" -Path $exePath

Write-Host ""
Write-Host "Madhyamas MCP Server has been installed!" -ForegroundColor Green
Write-Host ""
Write-Host "Configure in Claude Desktop config:"
Write-Host '  "mcpServers": {'
Write-Host '    "madhyamas": {'
Write-Host "      `"command`": `"$exePath`""
Write-Host '    }'
Write-Host '  }'
Write-Host ""

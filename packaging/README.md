# Madhyamas Packaging

This directory contains packaging configurations for distributing Madhyamas across different platforms and package managers.

## Package Structure

Madhyamas is distributed as three separate packages:

| Package | Description | Contains |
|---------|-------------|----------|
| **madhyamas** | Main proxy server with web UI | madhyamas-core + madhyamas-api + web UI |
| **madhyamas-cli** | Command-line interface | CLI tool for interacting with Madhyamas |
| **madhyamas-mcp** | MCP Server | Model Context Protocol server for AI agents |

## Installation Methods

### macOS (Homebrew)

```bash
# Add the Madhyamas tap
brew tap madhyamas/tap

# Install packages
brew install madhyamas           # Main server
brew install madhyamas-cli       # CLI tool
brew install madhyamas-mcp       # MCP server
```

### Windows

#### MSI Installer
Download the `.msi` installer from the [releases page](https://github.com/madhyamas/madhyamas/releases).

#### Chocolatey
```powershell
# Install packages
choco install madhyamas
choco install madhyamas-cli
choco install madhyamas-mcp
```

### Linux

#### Snap (Ubuntu, Debian, etc.)
```bash
sudo snap install madhyamas
sudo snap install madhyamas-cli
sudo snap install madhyamas-mcp
```

#### DNF/YUM (Fedora, RHEL, CentOS)
```bash
# Add the repository
sudo dnf config-manager --add-repo https://rpm.madhyamas.io/madhyamas.repo

# Install packages
sudo dnf install madhyamas
sudo dnf install madhyamas-cli
sudo dnf install madhyamas-mcp
```

#### AUR (Arch Linux)
```bash
# Using yay
yay -S madhyamas
yay -S madhyamas-cli
yay -S madhyamas-mcp

# Or using paru
paru -S madhyamas
```

## Directory Structure

```
packaging/
├── homebrew/           # Homebrew formulas for macOS/Linux
│   ├── madhyamas.rb
│   ├── madhyamas-cli.rb
│   └── madhyamas-mcp.rb
├── windows/            # Windows packaging
│   ├── msi/            # WiX MSI installer
│   └── chocolatey/     # Chocolatey packages
├── linux/
│   ├── snap/           # Snap packages
│   ├── rpm/            # RPM specs for DNF/YUM
│   └── aur/            # Arch User Repository
└── scripts/            # Build and release scripts
```

## Building Packages

See the GitHub Actions workflow in `.github/workflows/release.yml` for automated package building.

### Manual Build

```bash
# Build all packages
./packaging/scripts/build-all.sh

# Build specific platform
./packaging/scripts/build-macos.sh
./packaging/scripts/build-windows.sh
./packaging/scripts/build-linux.sh
```

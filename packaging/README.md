# Madhyamas Packaging

This directory contains packaging configurations for distributing Madhyamas across different platforms and package managers.

## Package Structure

Madhyamas is distributed as a **single unified binary** that includes the proxy server, web UI (embedded), MCP server, and CLI:

| Package | Description | Contains |
|---------|-------------|----------|
| **madhyamas** | Unified binary | Proxy server + web UI + MCP server + CLI |

### Subcommands

```bash
madhyamas              # Start proxy server with web UI (default)
madhyamas serve        # Same as above
madhyamas mcp          # Run as MCP server (stdio)
madhyamas traffic list # CLI command
madhyamas --help       # See all commands
```

## Installation Methods

### macOS (Homebrew)

```bash
# Add the Madhyamas tap
brew tap madhyamas/tap

# Install
brew install madhyamas
```

### Windows

#### MSI Installer
Download the `.msi` installer from the [releases page](https://github.com/ShristiLabs/madhyamas/releases).

#### Chocolatey
```powershell
choco install madhyamas
```

### Linux

#### Snap (Ubuntu, Debian, etc.)
```bash
sudo snap install madhyamas
```

#### DNF/YUM (Fedora, RHEL, CentOS)
```bash
# Add the repository
sudo dnf config-manager --add-repo https://rpm.madhyamas.io/madhyamas.repo

# Install
sudo dnf install madhyamas
```

#### AUR (Arch Linux)
```bash
# Using yay
yay -S madhyamas

# Or using paru
paru -S madhyamas
```

## Directory Structure

```
packaging/
├── homebrew/           # Homebrew formulas for macOS/Linux
│   └── madhyamas.rb
├── windows/            # Windows packaging
│   ├── msi/            # WiX MSI installer
│   └── chocolatey/     # Chocolatey package
├── linux/
│   ├── snap/           # Snap package
│   ├── rpm/            # RPM spec for DNF/YUM
│   └── aur/            # Arch User Repository
└── scripts/            # Build and release scripts
```

## Building Packages

See the GitHub Actions workflow in `.github/workflows/release.yml` for automated package building.

### Manual Build

```bash
# Build the unified binary
cargo build --release -p madhyamas

# Build for specific targets
./packaging/scripts/build-all.sh
```

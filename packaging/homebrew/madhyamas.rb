# Homebrew Formula for Madhyamas
# Single unified binary: proxy server + web UI (embedded) + MCP + CLI
#
# Install: brew install madhyamas/tap/madhyamas

class Madhyamas < Formula
  desc "Open-source HTTP/HTTPS debugging proxy with web-based UI"
  homepage "https://github.com/madhyamas/madhyamas"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_intel do
      url "https://github.com/madhyamas/madhyamas/releases/download/v#{version}/madhyamas-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_MACOS_INTEL"
    end
    on_arm do
      url "https://github.com/madhyamas/madhyamas/releases/download/v#{version}/madhyamas-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_MACOS_ARM"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/madhyamas/madhyamas/releases/download/v#{version}/madhyamas-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_SHA256_LINUX_INTEL"
    end
    on_arm do
      url "https://github.com/madhyamas/madhyamas/releases/download/v#{version}/madhyamas-v#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_SHA256_LINUX_ARM"
    end
  end

  depends_on "openssl@3"

  def install
    # Single binary — web UI is embedded, no external assets needed
    bin.install "madhyamas"
  end

  def caveats
    <<~EOS
      Madhyamas has been installed!

      To start the proxy server:
        madhyamas
      # or: madhyamas serve

      To start as a background service:
        brew services start madhyamas

      Web UI: http://localhost:3001
      Proxy:  localhost:8888

      Other modes:
        madhyamas mcp              # Run as MCP server
        madhyamas traffic list     # CLI commands
        madhyamas --help           # See all options

      To trust the Madhyamas CA certificate (required for HTTPS):
        Install the cert from ~/.madhyamas/certs/madhyamas-ca.pem

      For more information:
        https://github.com/madhyamas/madhyamas#readme
    EOS
  end

  service do
    run [opt_bin/"madhyamas"]
    keep_alive true
    log_path var/"log/madhyamas.log"
    error_log_path var/"log/madhyamas.error.log"
    working_dir var/"lib/madhyamas"
  end

  test do
    assert_match "madhyamas #{version}", shell_output("#{bin}/madhyamas --version")
  end
end

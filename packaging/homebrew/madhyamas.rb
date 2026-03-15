# Homebrew Formula for Madhyamas (Main Server)
# This is the main proxy server with embedded web UI
#
# Install: brew install madhyamas/tap/madhyamas

class Proxyforge < Formula
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
    bin.install "madhyamas"

    # Install web assets alongside the binary
    (share/"madhyamas/web").install Dir["web/*"] if Dir.exist?("web")

    # Generate shell completions
    generate_completions_from_executable(bin/"madhyamas", "completion", "--shell", shells: [:bash, :zsh, :fish])
  end

  def caveats
    <<~EOS
      Madhyamas has been installed!

      To start the proxy server:
        madhyamas start

      To start as a background service:
        brew services start madhyamas

      Web UI will be available at:
        http://localhost:3000

      Proxy server listens on:
        localhost:8888

      To trust the Madhyamas CA certificate (required for HTTPS):
        madhyamas cert trust

      For more information:
        https://github.com/madhyamas/madhyamas#readme
    EOS
  end

  service do
    run [opt_bin/"madhyamas", "start"]
    keep_alive true
    log_path var/"log/madhyamas.log"
    error_log_path var/"log/madhyamas.error.log"
    working_dir var/"lib/madhyamas"
  end

  test do
    assert_match "madhyamas #{version}", shell_output("#{bin}/madhyamas --version")
  end
end

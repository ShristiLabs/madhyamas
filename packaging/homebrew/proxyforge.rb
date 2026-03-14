# Homebrew Formula for ProxyForge
# Install with: brew install --formula ./proxyforge.rb

class Proxyforge < Formula
  desc "Open-source HTTP/HTTPS debugging proxy with web-based UI"
  homepage "https://github.com/proxyforge/proxyforge"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_intel do
      url "https://github.com/proxyforge/proxyforge/releases/download/v#{version}/proxyforge-#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256"
    end
    on_arm do
      url "https://github.com/proxyforge/proxyforge/releases/download/v#{version}/proxyforge-#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/proxyforge/proxyforge/releases/download/v#{version}/proxyforge-#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_SHA256"
    end
    on_arm do
      url "https://github.com/proxyforge/proxyforge/releases/download/v#{version}/proxyforge-#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_SHA256"
    end
  end

  depends_on "openssl@3"

  def install
    bin.install "proxyforge"

    # Install web assets
    libexec.install "web"

    # Create data directory
    (var/"lib/proxyforge").mkpath

    # Generate shell completions
    generate_completions_from_executable(bin/"proxyforge", "completion", "--shell", shells: [:bash, :zsh, :fish])
  end

  def caveats
    <<~EOS
      ProxyForge has been installed!

      To start the proxy server:
        proxyforge start

      To configure your browser to use the proxy:
        HTTP Proxy:  localhost:8888
        HTTPS Proxy: localhost:8888

      To trust the ProxyForge CA certificate:
        macOS: open ~/Library/Application\ Support/proxyforge/certs/ca.crt
        Then add it to Keychain and set to "Always Trust"

      For more information:
        https://github.com/proxyforge/proxyforge#readme
    EOS
  end

  service do
    run [opt_bin/"proxyforge", "start"]
    keep_alive true
    log_path var/"log/proxyforge.log"
    error_log_path var/"log/proxyforge.error.log"
    working_dir var/"lib/proxyforge"
  end

  test do
    assert_match "ProxyForge #{version}", shell_output("#{bin}/proxyforge --version")
  end
end

# Homebrew Formula for Madhyamas CLI
# Command-line interface for interacting with Madhyamas
#
# Install: brew install madhyamas/tap/madhyamas-cli

class ProxyforgeCli < Formula
  desc "Command-line interface for Madhyamas debugging proxy"
  homepage "https://github.com/madhyamas/madhyamas"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_intel do
      url "https://github.com/madhyamas/madhyamas/releases/download/v#{version}/madhyamas-cli-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_MACOS_INTEL"
    end
    on_arm do
      url "https://github.com/madhyamas/madhyamas/releases/download/v#{version}/madhyamas-cli-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_MACOS_ARM"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/madhyamas/madhyamas/releases/download/v#{version}/madhyamas-cli-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_SHA256_LINUX_INTEL"
    end
    on_arm do
      url "https://github.com/madhyamas/madhyamas/releases/download/v#{version}/madhyamas-cli-v#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_SHA256_LINUX_ARM"
    end
  end

  def install
    bin.install "madhyamas-cli"

    # Create symlink for convenience
    bin.install_symlink "madhyamas-cli" => "pf"

    # Generate shell completions
    generate_completions_from_executable(bin/"madhyamas-cli", "completion", "--shell", shells: [:bash, :zsh, :fish])
  end

  def caveats
    <<~EOS
      Madhyamas CLI has been installed!

      Usage:
        madhyamas-cli <command> [options]
        pf <command> [options]  # shorthand alias

      Common commands:
        madhyamas-cli traffic list     # List captured traffic
        madhyamas-cli session list     # List debug sessions
        madhyamas-cli config get       # Show configuration

      Ensure Madhyamas server is running:
        brew services start madhyamas

      For more information:
        madhyamas-cli --help
    EOS
  end

  test do
    assert_match "madhyamas-cli #{version}", shell_output("#{bin}/madhyamas-cli --version")
  end
end

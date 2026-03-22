# Homebrew Formula for Madhyamas MCP Server
# Model Context Protocol server for AI agent integration
#
# Install: brew install madhyamas/tap/madhyamas-mcp

class MadhyamasMcp < Formula
  desc "MCP server for Madhyamas - enables AI agents to interact with the proxy"
  homepage "https://github.com/madhyamas/madhyamas"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_intel do
      url "https://github.com/madhyamas/madhyamas/releases/download/v#{version}/madhyamas-mcp-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_MACOS_INTEL"
    end
    on_arm do
      url "https://github.com/madhyamas/madhyamas/releases/download/v#{version}/madhyamas-mcp-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER_SHA256_MACOS_ARM"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/madhyamas/madhyamas/releases/download/v#{version}/madhyamas-mcp-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_SHA256_LINUX_INTEL"
    end
    on_arm do
      url "https://github.com/madhyamas/madhyamas/releases/download/v#{version}/madhyamas-mcp-v#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_SHA256_LINUX_ARM"
    end
  end

  def install
    bin.install "madhyamas-mcp"
  end

  def caveats
    <<~EOS
      Madhyamas MCP Server has been installed!

      This is a Model Context Protocol (MCP) server that allows AI agents
      (like Claude, Cursor, Windsurf) to interact with Madhyamas.

      Configuration for Claude Desktop (~/.config/claude/claude_desktop_config.json):
        {
          "mcpServers": {
            "madhyamas": {
              "command": "#{opt_bin}/madhyamas-mcp",
              "env": {
                "MADHYAMAS_API_URL": "http://127.0.0.1:3001"
              }
            }
          }
        }

      Configuration for Windsurf/Cursor:
        Add to your MCP configuration file with the path:
        #{opt_bin}/madhyamas-mcp

      Ensure Madhyamas server is running:
        brew services start madhyamas

      For more information:
        https://github.com/madhyamas/madhyamas#mcp-integration
    EOS
  end

  test do
    # MCP server uses stdio, so we just verify it exists and is executable
    assert_predicate bin/"madhyamas-mcp", :executable?
  end
end

# Homebrew formula for ProxyForge

class Proxyforge < Formula
  desc "Open source HTTP/HTTPS debugging proxy"
  homepage "https://github.com/proxyforge/proxyforge"
  license "MIT"
  head_branch "main"

  depends_on "rust"

  on_mac:sonoma
  on_linux:sonoma
  on Intel: sonoma

  def install
    prefix.install_opt_lib "/opt/homebrew-cask"
    system "install"
  end

  test do!
    system "proxyforge --version"
  end
end

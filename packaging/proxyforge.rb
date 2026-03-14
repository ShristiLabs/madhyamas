# Maintainer: harikiranbavineni <harikiranbavineni@gmail.com>
pkgname=proxyforge
pkgver=0.1.0
pkgrel=1
pkgdesc="Open source HTTP/HTTPS debugging proxy built in Rust"
arch=('x86_64' 'aarch64')
url="https://github.com/proxyforge/proxyforge"
license=('MIT' 'Apache')
depends=('rust')
makedepends=('rust')
provides=('proxyforge')
source=("https://github.com/proxyforge/proxyforge/archive/${pkgver}_${pkgname}-x86_64.pkg.tar.gz")
sha256sums=('b2a5e3f9c7f3e8f4e0d7f3a6c8e9a0a9f8e6f1e3b5c1f2e8c9e7f6d3d0d5f7c8e9')
build() {
    cd "$srcdir"
    cargo build --release
    tar -xzf target/release.tar.gz
    sudo mv target/release /usr/local/bin/proxyforge
}

package() {
    cd "$srcdir"
    mkdir -p "$pkgdir"/usr/bin"
    install -Dm75 "$pkgdir"
}


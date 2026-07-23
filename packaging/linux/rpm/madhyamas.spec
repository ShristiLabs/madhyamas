# RPM Spec for Madhyamas
# For Fedora, RHEL, CentOS, Rocky Linux, AlmaLinux
# Build: rpmbuild -ba madhyamas.spec

Name:           madhyamas
Version:        0.1.0
Release:        1%{?dist}
Summary:        Open-source HTTP/HTTPS debugging proxy with web-based UI

License:        MIT
URL:            https://github.com/ShristiLabs/madhyamas
Source0:        https://github.com/ShristiLabs/madhyamas/archive/refs/tags/v%{version}.tar.gz

BuildRequires:  rust >= 1.88
BuildRequires:  cargo
BuildRequires:  openssl-devel
BuildRequires:  nodejs >= 18
BuildRequires:  npm
BuildRequires:  systemd-rpm-macros

Requires:       openssl
Requires:       glibc

%description
Madhyamas is a high-performance, cross-platform HTTP/HTTPS debugging proxy
built in Rust with a modern web-based UI. It's the free, open-source
alternative to tools like Charles Proxy and Fiddler.

Single unified binary includes:
- Proxy server with HTTPS interception
- Web UI (embedded in binary, no external files)
- MCP server for AI agent integration
- CLI commands for scripting and automation

%prep
%autosetup -n %{name}-%{version}

%build
# Build frontend (embedded into binary via rust-embed)
cd web
npm ci
npm run build
cd ..

# Build the unified binary
cargo build --release -p madhyamas

%install
# Install binary (web UI is embedded — no external assets needed)
install -Dm755 target/release/madhyamas %{buildroot}%{_bindir}/madhyamas

# Install systemd service
install -Dm644 packaging/linux/rpm/madhyamas.service %{buildroot}%{_unitdir}/madhyamas.service

# Install default config
install -Dm644 config/default.toml %{buildroot}%{_sysconfdir}/madhyamas/config.toml

%post
%systemd_post madhyamas.service

%preun
%systemd_preun madhyamas.service

%postun
%systemd_postun_with_restart madhyamas.service

%files
%license LICENSE-MIT LICENSE-APACHE
%doc README.md
%{_bindir}/madhyamas
%{_unitdir}/madhyamas.service
%config(noreplace) %{_sysconfdir}/madhyamas/config.toml

%changelog
* %(date "+%a %b %d %Y") Madhyamas Team <team@madhyamas.io> - %{version}-%{release}
- Initial package release

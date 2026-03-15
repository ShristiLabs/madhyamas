# RPM Spec for Madhyamas
# For Fedora, RHEL, CentOS, Rocky Linux, AlmaLinux
# Build: rpmbuild -ba madhyamas.spec

Name:           madhyamas
Version:        0.1.0
Release:        1%{?dist}
Summary:        Open-source HTTP/HTTPS debugging proxy with web-based UI

License:        MIT
URL:            https://github.com/madhyamas/madhyamas
Source0:        https://github.com/madhyamas/madhyamas/archive/refs/tags/v%{version}.tar.gz

BuildRequires:  rust >= 1.75
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

Features:
- HTTP/HTTPS traffic interception and inspection
- Automatic TLS certificate generation
- Modern React-based web UI
- Request/response filtering and search
- Session management for organized debugging

%prep
%autosetup -n %{name}-%{version}

%build
# Build backend
cargo build --release -p madhyamas-cli

# Build frontend
cd web
npm ci
npm run build

%install
# Install binary
install -Dm755 target/release/madhyamas %{buildroot}%{_bindir}/madhyamas

# Install web assets
install -dm755 %{buildroot}%{_datadir}/madhyamas/web
cp -r web/dist/* %{buildroot}%{_datadir}/madhyamas/web/

# Install systemd service
install -Dm644 packaging/linux/rpm/madhyamas.service %{buildroot}%{_unitdir}/madhyamas.service

# Install default config
install -Dm644 config/default.toml %{buildroot}%{_sysconfdir}/madhyamas/config.toml

# Install shell completions
install -dm755 %{buildroot}%{_datadir}/bash-completion/completions
install -dm755 %{buildroot}%{_datadir}/zsh/site-functions
install -dm755 %{buildroot}%{_datadir}/fish/vendor_completions.d

%{buildroot}%{_bindir}/madhyamas completion --shell bash > %{buildroot}%{_datadir}/bash-completion/completions/madhyamas
%{buildroot}%{_bindir}/madhyamas completion --shell zsh > %{buildroot}%{_datadir}/zsh/site-functions/_madhyamas
%{buildroot}%{_bindir}/madhyamas completion --shell fish > %{buildroot}%{_datadir}/fish/vendor_completions.d/madhyamas.fish

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
%{_datadir}/madhyamas/
%{_unitdir}/madhyamas.service
%config(noreplace) %{_sysconfdir}/madhyamas/config.toml
%{_datadir}/bash-completion/completions/madhyamas
%{_datadir}/zsh/site-functions/_madhyamas
%{_datadir}/fish/vendor_completions.d/madhyamas.fish

%changelog
* %(date "+%a %b %d %Y") Madhyamas Team <team@madhyamas.io> - %{version}-%{release}
- Initial package release

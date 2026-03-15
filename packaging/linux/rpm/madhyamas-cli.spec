# RPM Spec for Madhyamas CLI
# For Fedora, RHEL, CentOS, Rocky Linux, AlmaLinux

Name:           madhyamas-cli
Version:        0.1.0
Release:        1%{?dist}
Summary:        Command-line interface for Madhyamas debugging proxy

License:        MIT
URL:            https://github.com/madhyamas/madhyamas
Source0:        https://github.com/madhyamas/madhyamas/archive/refs/tags/v%{version}.tar.gz

BuildRequires:  rust >= 1.75
BuildRequires:  cargo
BuildRequires:  openssl-devel

Requires:       openssl
Requires:       glibc

%description
Madhyamas CLI is the command-line interface for interacting with
the Madhyamas HTTP/HTTPS debugging proxy.

Features:
- View and filter captured traffic
- Manage debugging sessions
- Configure proxy settings
- Export traffic data

%prep
%autosetup -n madhyamas-%{version}

%build
cargo build --release -p madhyamas-cli

%install
install -Dm755 target/release/madhyamas-cli %{buildroot}%{_bindir}/madhyamas-cli

# Create symlink alias
ln -s madhyamas-cli %{buildroot}%{_bindir}/pf

# Install shell completions
install -dm755 %{buildroot}%{_datadir}/bash-completion/completions
install -dm755 %{buildroot}%{_datadir}/zsh/site-functions
install -dm755 %{buildroot}%{_datadir}/fish/vendor_completions.d

%{buildroot}%{_bindir}/madhyamas-cli completion --shell bash > %{buildroot}%{_datadir}/bash-completion/completions/madhyamas-cli
%{buildroot}%{_bindir}/madhyamas-cli completion --shell zsh > %{buildroot}%{_datadir}/zsh/site-functions/_madhyamas-cli
%{buildroot}%{_bindir}/madhyamas-cli completion --shell fish > %{buildroot}%{_datadir}/fish/vendor_completions.d/madhyamas-cli.fish

%files
%license LICENSE-MIT LICENSE-APACHE
%doc README.md
%{_bindir}/madhyamas-cli
%{_bindir}/pf
%{_datadir}/bash-completion/completions/madhyamas-cli
%{_datadir}/zsh/site-functions/_madhyamas-cli
%{_datadir}/fish/vendor_completions.d/madhyamas-cli.fish

%changelog
* %(date "+%a %b %d %Y") Madhyamas Team <team@madhyamas.io> - %{version}-%{release}
- Initial package release

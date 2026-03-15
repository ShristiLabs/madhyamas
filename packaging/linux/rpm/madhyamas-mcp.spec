# RPM Spec for Madhyamas MCP Server
# For Fedora, RHEL, CentOS, Rocky Linux, AlmaLinux

Name:           madhyamas-mcp
Version:        0.1.0
Release:        1%{?dist}
Summary:        MCP server for Madhyamas - AI agent integration

License:        MIT
URL:            https://github.com/madhyamas/madhyamas
Source0:        https://github.com/madhyamas/madhyamas/archive/refs/tags/v%{version}.tar.gz

BuildRequires:  rust >= 1.75
BuildRequires:  cargo
BuildRequires:  openssl-devel

Requires:       openssl
Requires:       glibc

%description
Madhyamas MCP Server implements the Model Context Protocol (MCP)
to enable AI agents like Claude, Cursor, and Windsurf to interact
with Madhyamas for automated debugging and traffic analysis.

%prep
%autosetup -n madhyamas-%{version}

%build
cargo build --release -p madhyamas-mcp

%install
install -Dm755 target/release/madhyamas-mcp %{buildroot}%{_bindir}/madhyamas-mcp

%files
%license LICENSE-MIT LICENSE-APACHE
%doc README.md
%{_bindir}/madhyamas-mcp

%changelog
* %(date "+%a %b %d %Y") Madhyamas Team <team@madhyamas.io> - %{version}-%{release}
- Initial package release

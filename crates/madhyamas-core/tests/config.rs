//! Integration tests for the public config API: defaults, serde round-trips,
//! debug-log config filtering, secrets config, and upstream proxy rules.

use madhyamas_core::config::{
    DebugLogConfig, DebugLogLevel, ProxyConfig, SecretsConfig, UpstreamProxyConfig,
};

fn cfg(enabled: bool, protocol: &str, host: &str, port: u16) -> UpstreamProxyConfig {
    UpstreamProxyConfig {
        enabled,
        protocol: protocol.to_string(),
        host: host.to_string(),
        port,
        auth_username: None,
        auth_password: None,
        no_proxy_hosts: Vec::new(),
    }
}

// ── DebugLogConfig / DebugLogLevel ───────────────────────────────────────

#[test]
fn secrets_config_defaults() {
    let c = SecretsConfig::default();
    assert!(c.redact_enabled);
    assert!(c
        .redact_headers
        .iter()
        .any(|h| h.eq_ignore_ascii_case("authorization")));
    assert!(c
        .redact_headers
        .iter()
        .any(|h| h.eq_ignore_ascii_case("cookie")));
}

#[test]
fn secrets_config_deserializes_minimal_and_custom() {
    // Absent fields fall back to defaults (serde default).
    let c: SecretsConfig = serde_json::from_str("{}").unwrap();
    assert!(c.redact_enabled);
    assert!(!c.redact_headers.is_empty());
    // Custom headers + disabled.
    let c: SecretsConfig =
        serde_json::from_str(r#"{"redact_enabled": false, "redact_headers": ["X-Internal"]}"#)
            .unwrap();
    assert!(!c.redact_enabled);
    assert_eq!(c.redact_headers, vec!["X-Internal".to_string()]);
    // ProxyConfig round-trips with the secrets section.
    let pc = ProxyConfig::default();
    assert!(pc.secrets.redact_enabled);
}

#[test]
fn debug_log_config_defaults() {
    let c = DebugLogConfig::default();
    assert!(!c.enabled);
    assert_eq!(c.level, DebugLogLevel::Summary);
    assert_eq!(c.host_filter, None);
    assert_eq!(
        c.redact_headers,
        vec![
            "Authorization".to_string(),
            "Cookie".to_string(),
            "Set-Cookie".to_string()
        ]
    );
    assert!(!c.redact_bodies);
}

#[test]
fn debug_log_level_parse_valid_values() {
    assert_eq!(
        DebugLogLevel::parse("summary"),
        Some(DebugLogLevel::Summary)
    );
    assert_eq!(
        DebugLogLevel::parse("headers"),
        Some(DebugLogLevel::Headers)
    );
    assert_eq!(DebugLogLevel::parse("full"), Some(DebugLogLevel::Full));
    assert_eq!(DebugLogLevel::parse("  FULL "), Some(DebugLogLevel::Full));
    assert_eq!(DebugLogLevel::parse("verbose"), None);
    assert_eq!(DebugLogLevel::parse(""), None);
}

#[test]
fn debug_log_level_str_roundtrip() {
    for level in [
        DebugLogLevel::Summary,
        DebugLogLevel::Headers,
        DebugLogLevel::Full,
    ] {
        assert_eq!(DebugLogLevel::parse(level.as_str()), Some(level));
    }
    assert_eq!(DebugLogLevel::Summary.as_str(), "summary");
    assert_eq!(DebugLogLevel::Headers.as_str(), "headers");
    assert_eq!(DebugLogLevel::Full.as_str(), "full");
}

#[test]
fn debug_log_config_serde_roundtrip() {
    let c = DebugLogConfig {
        enabled: true,
        level: DebugLogLevel::Full,
        host_filter: Some(vec!["*.example.com".to_string()]),
        redact_headers: vec!["X-Secret".to_string()],
        redact_bodies: true,
    };
    let json = serde_json::to_string(&c).unwrap();
    let back: DebugLogConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(back, c);
    // Level serializes lowercase.
    assert!(json.contains("\"level\":\"full\""));
}

#[test]
fn debug_log_config_deserializes_defaults_when_empty_object() {
    let c: DebugLogConfig = serde_json::from_str("{}").unwrap();
    assert_eq!(c, DebugLogConfig::default());
}

#[test]
fn debug_log_config_should_log_disabled() {
    let c = DebugLogConfig {
        host_filter: None,
        ..DebugLogConfig::default()
    };
    assert!(!c.should_log("example.com"));
}

#[test]
fn debug_log_config_should_log_no_filter_matches_all() {
    let c = DebugLogConfig {
        enabled: true,
        ..DebugLogConfig::default()
    };
    assert!(c.should_log("anything.example.com"));
    let c = DebugLogConfig {
        enabled: true,
        host_filter: Some(Vec::new()),
        ..DebugLogConfig::default()
    };
    assert!(c.should_log("anything.example.com"));
}

#[test]
fn debug_log_config_should_log_filter_matching() {
    let c = DebugLogConfig {
        enabled: true,
        host_filter: Some(vec![
            "api.example.com".to_string(), // exact
            "*.internal.corp".to_string(), // wildcard subdomain
            "service*".to_string(),        // glob
        ]),
        ..DebugLogConfig::default()
    };
    assert!(c.should_log("api.example.com"));
    assert!(c.should_log("host.internal.corp"));
    assert!(c.should_log("service-1.prod"));
    // Suffix match on a non-wildcard entry.
    assert!(c.should_log("v2.api.example.com"));
    assert!(!c.should_log("other.example.com"));
    assert!(!c.should_log("external.com"));
}

#[test]
fn debug_log_config_should_log_case_and_trailing_dot_insensitive() {
    let c = DebugLogConfig {
        enabled: true,
        host_filter: Some(vec!["API.Example.COM.".to_string()]),
        ..DebugLogConfig::default()
    };
    assert!(c.should_log("api.example.com"));
}

#[test]
fn proxy_config_old_json_without_debug_logging_uses_defaults() {
    // Serialize the default config, strip the debug_logging section
    // (simulating an old config file), and confirm deserialization
    // falls back to defaults.
    let mut v: serde_json::Value = serde_json::to_value(ProxyConfig::default()).unwrap();
    let obj = v.as_object_mut().unwrap();
    obj.remove("debug_logging");
    let cfg: ProxyConfig = serde_json::from_value(v).unwrap();
    assert_eq!(cfg.debug_logging, DebugLogConfig::default());
}

#[test]
fn proxy_config_debug_logging_roundtrip() {
    let cfg = ProxyConfig {
        debug_logging: DebugLogConfig {
            enabled: true,
            level: DebugLogLevel::Headers,
            host_filter: Some(vec!["example.com".to_string()]),
            redact_headers: vec!["Authorization".to_string()],
            redact_bodies: true,
        },
        ..ProxyConfig::default()
    };
    let json = serde_json::to_string(&cfg).unwrap();
    let back: ProxyConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(back.debug_logging, cfg.debug_logging);
}

#[test]
fn proxy_url_returns_none_when_disabled() {
    let c = cfg(false, "http", "proxy.example.com", 8080);
    assert_eq!(c.proxy_url().unwrap(), None);
}

#[test]
fn proxy_url_returns_none_when_host_empty() {
    let c = cfg(true, "http", "", 8080);
    assert_eq!(c.proxy_url().unwrap(), None);
}

#[test]
fn proxy_url_builds_http_url() {
    let c = cfg(true, "http", "corp-proxy.example.com", 8080);
    assert_eq!(
        c.proxy_url().unwrap().as_deref(),
        Some("http://corp-proxy.example.com:8080")
    );
}

#[test]
fn proxy_url_builds_https_url() {
    let c = cfg(true, "https", "secure-proxy.example.com", 443);
    assert_eq!(
        c.proxy_url().unwrap().as_deref(),
        Some("https://secure-proxy.example.com:443")
    );
}

#[test]
fn proxy_url_builds_socks5_url() {
    let c = cfg(true, "socks5", "127.0.0.1", 1080);
    assert_eq!(
        c.proxy_url().unwrap().as_deref(),
        Some("socks5://127.0.0.1:1080")
    );
}

#[test]
fn proxy_url_rejects_invalid_protocol() {
    let c = cfg(true, "ftp", "proxy.example.com", 8080);
    assert!(c.proxy_url().is_err());
}

#[test]
fn proxy_url_normalizes_protocol_case() {
    let c = cfg(true, "SOCKS5", "127.0.0.1", 1080);
    assert_eq!(
        c.proxy_url().unwrap().as_deref(),
        Some("socks5://127.0.0.1:1080")
    );
}

#[test]
fn auth_enabled_requires_both_username_and_password() {
    let mut c = cfg(true, "http", "proxy", 8080);
    assert!(!c.auth_enabled());
    c.auth_username = Some("user".to_string());
    assert!(!c.auth_enabled());
    c.auth_password = Some("pass".to_string());
    assert!(c.auth_enabled());
}

#[test]
fn should_bypass_empty_list_never_bypasses() {
    let c = cfg(true, "http", "proxy", 8080);
    assert!(!c.should_bypass("localhost"));
    assert!(!c.should_bypass("example.com"));
}

#[test]
fn should_bypass_exact_hostname_match() {
    let c = UpstreamProxyConfig {
        no_proxy_hosts: vec!["localhost".to_string()],
        ..cfg(true, "http", "proxy", 8080)
    };
    assert!(c.should_bypass("localhost"));
    // Suffix matching: "localhost" matches "api.localhost" (consistent
    // with the existing should_passthrough logic).
    assert!(c.should_bypass("api.localhost"));
    // But not a completely different host.
    assert!(!c.should_bypass("example.com"));
}

#[test]
fn should_bypass_suffix_match() {
    let c = UpstreamProxyConfig {
        no_proxy_hosts: vec!["example.com".to_string()],
        ..cfg(true, "http", "proxy", 8080)
    };
    assert!(c.should_bypass("example.com"));
    assert!(c.should_bypass("api.example.com"));
    assert!(c.should_bypass("www.example.com"));
    assert!(!c.should_bypass("notexample.com"));
}

#[test]
fn should_bypass_wildcard_suffix() {
    let c = UpstreamProxyConfig {
        no_proxy_hosts: vec!["*.internal.corp".to_string()],
        ..cfg(true, "http", "proxy", 8080)
    };
    assert!(c.should_bypass("api.internal.corp"));
    assert!(c.should_bypass("internal.corp"));
    assert!(!c.should_bypass("external.corp"));
}

#[test]
fn should_bypass_bare_ipv4() {
    let c = UpstreamProxyConfig {
        no_proxy_hosts: vec!["127.0.0.1".to_string()],
        ..cfg(true, "http", "proxy", 8080)
    };
    assert!(c.should_bypass("127.0.0.1"));
    assert!(!c.should_bypass("127.0.0.2"));
}

#[test]
fn should_bypass_ipv4_cidr() {
    let c = UpstreamProxyConfig {
        no_proxy_hosts: vec!["192.168.0.0/16".to_string()],
        ..cfg(true, "http", "proxy", 8080)
    };
    assert!(c.should_bypass("192.168.1.100"));
    assert!(c.should_bypass("192.168.0.0"));
    assert!(c.should_bypass("192.168.255.255"));
    assert!(!c.should_bypass("192.169.0.1"));
    assert!(!c.should_bypass("10.0.0.1"));
}

#[test]
fn should_bypass_ipv4_cidr_24() {
    let c = UpstreamProxyConfig {
        no_proxy_hosts: vec!["10.0.0.0/24".to_string()],
        ..cfg(true, "http", "proxy", 8080)
    };
    assert!(c.should_bypass("10.0.0.0"));
    assert!(c.should_bypass("10.0.0.255"));
    assert!(!c.should_bypass("10.0.1.0"));
}

#[test]
fn should_bypass_ipv6_cidr() {
    let c = UpstreamProxyConfig {
        no_proxy_hosts: vec!["fd00::/8".to_string()],
        ..cfg(true, "http", "proxy", 8080)
    };
    assert!(c.should_bypass("fd00::1"));
    assert!(c.should_bypass("fd12:3456::abcd"));
    assert!(!c.should_bypass("fe00::1"));
}

#[test]
fn should_bypass_case_insensitive() {
    let c = UpstreamProxyConfig {
        no_proxy_hosts: vec!["Example.COM".to_string()],
        ..cfg(true, "http", "proxy", 8080)
    };
    assert!(c.should_bypass("API.example.com"));
    assert!(c.should_bypass("EXAMPLE.com"));
}

#[test]
fn should_bypass_multiple_entries() {
    let c = UpstreamProxyConfig {
        no_proxy_hosts: vec![
            "localhost".to_string(),
            "127.0.0.0/8".to_string(),
            "*.internal.corp".to_string(),
        ],
        ..cfg(true, "http", "proxy", 8080)
    };
    assert!(c.should_bypass("localhost"));
    assert!(c.should_bypass("127.0.0.1"));
    assert!(c.should_bypass("127.255.255.255"));
    assert!(c.should_bypass("api.internal.corp"));
    assert!(!c.should_bypass("example.com"));
}

#[test]
fn should_bypass_trims_entries() {
    let c = UpstreamProxyConfig {
        no_proxy_hosts: vec!["  localhost  ".to_string()],
        ..cfg(true, "http", "proxy", 8080)
    };
    assert!(c.should_bypass("localhost"));
}

#[test]
fn default_upstream_proxy_is_disabled() {
    let c = UpstreamProxyConfig::default();
    assert!(!c.enabled);
    assert_eq!(c.protocol, "http");
    assert!(c.host.is_empty());
    assert_eq!(c.port, 0);
    assert!(!c.auth_enabled());
    assert!(c.no_proxy_hosts.is_empty());
}

#[test]
fn proxy_config_default_has_disabled_upstream() {
    let c = ProxyConfig::default();
    assert!(!c.upstream_proxy_active());
    assert!(!c.should_bypass_upstream("anything.com"));
}

#[test]
fn proxy_config_upstream_proxy_active_when_enabled_with_host() {
    let c = ProxyConfig {
        upstream_proxy: cfg(true, "http", "corp-proxy", 8080),
        ..Default::default()
    };
    assert!(c.upstream_proxy_active());
}

#[test]
fn proxy_config_upstream_proxy_inactive_when_enabled_but_no_host() {
    let c = ProxyConfig {
        upstream_proxy: cfg(true, "http", "", 8080),
        ..Default::default()
    };
    assert!(!c.upstream_proxy_active());
}

#[test]
fn proxy_config_should_bypass_upstream_respects_disabled_state() {
    let c = ProxyConfig {
        upstream_proxy: UpstreamProxyConfig {
            enabled: false,
            no_proxy_hosts: vec!["localhost".to_string()],
            ..cfg(false, "http", "proxy", 8080)
        },
        ..Default::default()
    };
    // Even though "localhost" is in the bypass list, the proxy is
    // disabled so should_bypass_upstream must return false.
    assert!(!c.should_bypass_upstream("localhost"));
}

#[test]
fn upstream_config_serializes_and_deserializes() {
    let c = UpstreamProxyConfig {
        enabled: true,
        protocol: "socks5".to_string(),
        host: "proxy.example.com".to_string(),
        port: 1080,
        auth_username: Some("user".to_string()),
        auth_password: Some("pass".to_string()),
        no_proxy_hosts: vec!["localhost".to_string(), "10.0.0.0/8".to_string()],
    };
    let json = serde_json::to_string(&c).unwrap();
    let back: UpstreamProxyConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(c, back);
}

#[test]
fn upstream_config_deserializes_with_defaults_for_missing_fields() {
    // Simulates an old config file that predates the upstream_proxy field.
    let json = r#"{"enabled": true, "host": "proxy", "port": 8080}"#;
    let c: UpstreamProxyConfig = serde_json::from_str(json).unwrap();
    assert!(c.enabled);
    assert_eq!(c.protocol, "http"); // default applied
    assert_eq!(c.host, "proxy");
    assert_eq!(c.port, 8080);
    assert!(c.auth_username.is_none());
    assert!(c.no_proxy_hosts.is_empty());
}

#[test]
fn proxy_config_with_upstream_serializes_roundtrip() {
    let c = ProxyConfig {
        upstream_proxy: UpstreamProxyConfig {
            enabled: true,
            protocol: "https".to_string(),
            host: "secure-proxy.example.com".to_string(),
            port: 443,
            auth_username: Some("alice".to_string()),
            auth_password: Some("secret".to_string()),
            no_proxy_hosts: vec!["localhost".to_string()],
        },
        ..Default::default()
    };
    let json = serde_json::to_string(&c).unwrap();
    let back: ProxyConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(c.upstream_proxy, back.upstream_proxy);
}

#[test]
fn proxy_config_without_upstream_field_deserializes_with_default() {
    // A config JSON that omits the upstream_proxy field entirely (as
    // written by older versions of Madhyamas) must deserialize with the
    // default disabled upstream proxy.
    let json = r#"{
        "proxy_port": 8888,
        "api_port": 3001,
        "host": "127.0.0.1",
        "public_ip": null,
        "cert_path": "/tmp/certs",
        "db_path": "/tmp/db",
        "log_path": "/tmp/logs",
        "verbose": false,
        "max_requests": 10000,
        "intercept_https": true,
        "max_body_size": 20971520,
        "passthrough_domains": []
    }"#;
    let c: ProxyConfig = serde_json::from_str(json).unwrap();
    assert!(!c.upstream_proxy_active());
    assert_eq!(c.upstream_proxy, UpstreamProxyConfig::default());
}

//! Integration tests for the public security API: SSRF-safe callback URL
//! validation and private-IP classification.

use std::net::IpAddr;

use madhyamas_enterprise::{is_private_ip, validate_callback_url};

#[test]
fn test_validate_callback_url_valid_https() {
    assert!(validate_callback_url("https://example.com/callback").is_ok());
    assert!(validate_callback_url("https://idp.example.com:8443/cb").is_ok());
}

#[test]
fn test_validate_callback_url_rejects_http() {
    let err = validate_callback_url("http://example.com/callback").expect_err("http rejected");
    assert!(err.to_string().contains("HTTPS"));
}

#[test]
fn test_validate_callback_url_rejects_private_ipv4() {
    let err = validate_callback_url("https://10.0.0.1/callback").expect_err("private rejected");
    assert!(err.to_string().contains("private"));
}

#[test]
fn test_validate_callback_url_rejects_loopback_ipv4() {
    let err = validate_callback_url("https://127.0.0.1/callback").expect_err("loopback rejected");
    assert!(err.to_string().contains("private"));
}

#[test]
fn test_validate_callback_url_rejects_192_168() {
    let err = validate_callback_url("https://192.168.1.1/callback").expect_err("private rejected");
    assert!(err.to_string().contains("private"));
}

#[test]
fn test_validate_callback_url_rejects_172_16() {
    let err = validate_callback_url("https://172.16.0.1/callback").expect_err("private rejected");
    assert!(err.to_string().contains("private"));
}

#[test]
fn test_validate_callback_url_rejects_link_local() {
    let err =
        validate_callback_url("https://169.254.1.1/callback").expect_err("link-local rejected");
    assert!(err.to_string().contains("private"));
}

#[test]
fn test_validate_callback_url_rejects_ipv6_loopback() {
    let err = validate_callback_url("https://[::1]/callback").expect_err("ipv6 loopback rejected");
    assert!(err.to_string().contains("private"));
}

#[test]
fn test_validate_callback_url_rejects_ipv6_unique_local() {
    let err = validate_callback_url("https://[fc00::1]/callback")
        .expect_err("ipv6 unique-local rejected");
    assert!(err.to_string().contains("private"));
}

#[test]
fn test_validate_callback_url_rejects_invalid_url() {
    assert!(validate_callback_url("not-a-url").is_err());
}

#[test]
fn test_validate_callback_url_allows_domain() {
    assert!(validate_callback_url("https://my-idp.example.com/callback").is_ok());
}

#[test]
fn test_is_private_ip_ipv4() {
    assert!(is_private_ip(&IpAddr::V4("10.0.0.1".parse().unwrap())));
    assert!(is_private_ip(&IpAddr::V4("172.16.0.1".parse().unwrap())));
    assert!(is_private_ip(&IpAddr::V4("192.168.1.1".parse().unwrap())));
    assert!(is_private_ip(&IpAddr::V4("127.0.0.1".parse().unwrap())));
    assert!(is_private_ip(&IpAddr::V4("169.254.1.1".parse().unwrap())));
    assert!(is_private_ip(&IpAddr::V4("0.0.0.0".parse().unwrap())));
    assert!(!is_private_ip(&IpAddr::V4("8.8.8.8".parse().unwrap())));
    assert!(!is_private_ip(&IpAddr::V4("1.1.1.1".parse().unwrap())));
}

#[test]
fn test_is_private_ip_ipv6() {
    assert!(is_private_ip(&IpAddr::V6("::1".parse().unwrap())));
    assert!(is_private_ip(&IpAddr::V6("fc00::1".parse().unwrap())));
    assert!(is_private_ip(&IpAddr::V6("fd00::1".parse().unwrap())));
    assert!(is_private_ip(&IpAddr::V6("fe80::1".parse().unwrap())));
    assert!(is_private_ip(&IpAddr::V6("::".parse().unwrap())));
    assert!(!is_private_ip(&IpAddr::V6(
        "2606:4700:4700::1111".parse().unwrap()
    )));
}

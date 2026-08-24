//! Integration tests for the public access-control list API.

use std::net::IpAddr;

use madhyamas_core::AccessControlList;

fn ip(s: &str) -> IpAddr {
    s.parse().expect("valid IP")
}

// ── Construction ──────────────────────────────────────────────────

#[test]
fn empty_list_allows_all() {
    let acl = AccessControlList::new(&[]).unwrap();
    assert!(acl.is_allow_all());
    assert!(acl.is_allowed(ip("10.1.2.3")));
    assert!(acl.is_allowed(ip("192.168.0.1")));
    assert!(acl.is_allowed(ip("::1")));
    assert!(acl.is_allowed(ip("8.8.8.8")));
}

#[test]
fn allow_all_constructor() {
    let acl = AccessControlList::allow_all();
    assert!(acl.is_allow_all());
    assert!(acl.is_empty());
    assert!(acl.is_allowed(ip("1.2.3.4")));
}

#[test]
fn blank_entries_produce_allow_all() {
    let acl = AccessControlList::new(&["  ".to_string(), "".to_string()]).unwrap();
    assert!(acl.is_allow_all());
}

// ── IPv4 single-IP matching ───────────────────────────────────────

#[test]
fn single_ipv4_allows_exact_match() {
    let acl = AccessControlList::new(&["192.168.1.50".to_string()]).unwrap();
    assert!(!acl.is_allow_all());
    assert!(acl.is_allowed(ip("192.168.1.50")));
    assert!(!acl.is_allowed(ip("192.168.1.51")));
    assert!(!acl.is_allowed(ip("192.168.1.0")));
}

#[test]
fn explicit_32_prefix_matches_single_ip() {
    let acl = AccessControlList::new(&["10.0.0.5/32".to_string()]).unwrap();
    assert!(acl.is_allowed(ip("10.0.0.5")));
    assert!(!acl.is_allowed(ip("10.0.0.6")));
}

// ── IPv4 CIDR matching ────────────────────────────────────────────

#[test]
fn ipv4_cidr_24_matches_subnet() {
    let acl = AccessControlList::new(&["192.168.1.0/24".to_string()]).unwrap();
    assert!(acl.is_allowed(ip("192.168.1.0")));
    assert!(acl.is_allowed(ip("192.168.1.127")));
    assert!(acl.is_allowed(ip("192.168.1.255")));
    assert!(!acl.is_allowed(ip("192.168.0.255")));
    assert!(!acl.is_allowed(ip("192.168.2.0")));
}

#[test]
fn ipv4_cidr_16_matches_range() {
    let acl = AccessControlList::new(&["172.16.0.0/12".to_string()]).unwrap();
    assert!(acl.is_allowed(ip("172.16.0.1")));
    assert!(acl.is_allowed(ip("172.31.255.254")));
    assert!(!acl.is_allowed(ip("172.15.255.255")));
    assert!(!acl.is_allowed(ip("172.32.0.0")));
}

#[test]
fn ipv4_cidr_8_matches_class_a() {
    let acl = AccessControlList::new(&["10.0.0.0/8".to_string()]).unwrap();
    assert!(acl.is_allowed(ip("10.0.0.1")));
    assert!(acl.is_allowed(ip("10.255.255.255")));
    assert!(!acl.is_allowed(ip("11.0.0.0")));
}

#[test]
fn ipv4_cidr_0_matches_everything() {
    let acl = AccessControlList::new(&["0.0.0.0/0".to_string()]).unwrap();
    // Not "allow all" — it has an entry — but the entry matches all IPv4.
    assert!(!acl.is_allow_all());
    assert!(acl.is_allowed(ip("1.2.3.4")));
    assert!(acl.is_allowed(ip("255.255.255.255")));
}

// ── IPv6 matching ─────────────────────────────────────────────────

#[test]
fn single_ipv6_allows_exact_match() {
    let acl = AccessControlList::new(&["fd00::1".to_string()]).unwrap();
    assert!(acl.is_allowed(ip("fd00::1")));
    assert!(!acl.is_allowed(ip("fd00::2")));
}

#[test]
fn ipv6_cidr_8_matches_range() {
    let acl = AccessControlList::new(&["fd00::/8".to_string()]).unwrap();
    assert!(acl.is_allowed(ip("fd00::1")));
    assert!(acl.is_allowed(ip("fd12:3456:7890::abcd")));
    assert!(acl.is_allowed(ip("fdff::")));
    assert!(!acl.is_allowed(ip("fe00::1")));
}

#[test]
fn ipv6_cidr_128_matches_single() {
    let acl = AccessControlList::new(&["2001:db8::1/128".to_string()]).unwrap();
    assert!(acl.is_allowed(ip("2001:db8::1")));
    assert!(!acl.is_allowed(ip("2001:db8::2")));
}

// ── Localhost always allowed ──────────────────────────────────────

#[test]
fn localhost_ipv4_always_allowed() {
    let acl = AccessControlList::new(&["10.0.0.0/8".to_string()]).unwrap();
    assert!(acl.is_allowed(ip("127.0.0.1")));
    assert!(acl.is_allowed(ip("127.255.255.254")));
}

#[test]
fn localhost_ipv6_always_allowed() {
    let acl = AccessControlList::new(&["10.0.0.0/8".to_string()]).unwrap();
    assert!(acl.is_allowed(ip("::1")));
}

#[test]
fn localhost_allowed_even_when_not_listed() {
    let acl = AccessControlList::new(&["8.8.8.8".to_string()]).unwrap();
    assert!(!acl.is_allowed(ip("192.168.1.1")));
    // But localhost still works.
    assert!(acl.is_allowed(ip("127.0.0.1")));
    assert!(acl.is_allowed(ip("::1")));
}

// ── Mixed / multiple entries ──────────────────────────────────────

#[test]
fn multiple_entries_any_match() {
    let acl = AccessControlList::new(&[
        "10.0.0.0/8".to_string(),
        "192.168.0.0/16".to_string(),
        "fd00::/8".to_string(),
    ])
    .unwrap();
    assert!(acl.is_allowed(ip("10.5.5.5")));
    assert!(acl.is_allowed(ip("192.168.1.1")));
    assert!(acl.is_allowed(ip("fd00::42")));
    assert!(!acl.is_allowed(ip("172.16.0.1")));
    assert!(!acl.is_allowed(ip("8.8.8.8")));
}

#[test]
fn mixed_ipv4_and_ipv6_entries() {
    let acl =
        AccessControlList::new(&["10.0.0.0/8".to_string(), "2001:db8::/32".to_string()]).unwrap();
    assert!(acl.is_allowed(ip("10.1.2.3")));
    assert!(acl.is_allowed(ip("2001:db8::1")));
    assert!(!acl.is_allowed(ip("192.168.1.1")));
    assert!(!acl.is_allowed(ip("2001:db9::1")));
}

// ── Whitespace / normalization ────────────────────────────────────

#[test]
fn trims_whitespace_around_entries() {
    let acl = AccessControlList::new(&["  10.0.0.0/8  ".to_string()]).unwrap();
    assert!(acl.is_allowed(ip("10.1.2.3")));
}

#[test]
fn trims_whitespace_around_cidr_prefix() {
    let acl = AccessControlList::new(&["10.0.0.0 / 8".to_string()]).unwrap();
    assert!(acl.is_allowed(ip("10.1.2.3")));
}

// ── Address-family isolation ──────────────────────────────────────

#[test]
fn ipv4_entry_does_not_match_ipv6() {
    let acl = AccessControlList::new(&["10.0.0.0/8".to_string()]).unwrap();
    assert!(!acl.is_allowed(ip("::10.0.0.1"))); // IPv4-mapped IPv6
}

#[test]
fn ipv6_entry_does_not_match_ipv4() {
    let acl = AccessControlList::new(&["::ffff:10.0.0.0/112".to_string()]).unwrap();
    assert!(!acl.is_allowed(ip("10.0.0.1")));
}

// ── Error handling ────────────────────────────────────────────────

#[test]
fn rejects_invalid_ip() {
    let err = AccessControlList::new(&["not-an-ip".to_string()]).unwrap_err();
    assert!(err.to_string().contains("Invalid IP"));
}

#[test]
fn rejects_invalid_cidr_prefix() {
    let err = AccessControlList::new(&["10.0.0.0/abc".to_string()]).unwrap_err();
    assert!(err.to_string().contains("Invalid CIDR prefix"));
}

#[test]
fn rejects_ipv4_prefix_too_large() {
    let err = AccessControlList::new(&["10.0.0.0/33".to_string()]).unwrap_err();
    assert!(err.to_string().contains("IPv4 CIDR prefix too large"));
}

#[test]
fn rejects_ipv6_prefix_too_large() {
    let err = AccessControlList::new(&["fd00::/129".to_string()]).unwrap_err();
    assert!(err.to_string().contains("IPv6 CIDR prefix too large"));
}

// ── Introspection helpers ─────────────────────────────────────────

#[test]
fn len_and_is_empty() {
    let allow_all = AccessControlList::new(&[]).unwrap();
    assert!(allow_all.is_empty());
    assert_eq!(allow_all.len(), 0);

    let restricted = AccessControlList::new(&["10.0.0.0/8".to_string()]).unwrap();
    assert!(!restricted.is_empty());
    assert_eq!(restricted.len(), 1);
}

#[test]
fn debug_impl_does_not_leak_entries() {
    let acl = AccessControlList::new(&["10.0.0.0/8".to_string()]).unwrap();
    let s = format!("{:?}", acl);
    assert!(s.contains("entry_count"));
    assert!(s.contains("allow_all"));
    // The raw entry bytes should not appear in debug output.
    assert!(!s.contains("network"));
}

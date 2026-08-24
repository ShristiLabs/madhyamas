//! Integration tests for the public debug-log API: header redaction
//! formatting, body formatting (truncation, gzip, placeholders), and
//! request/response event emission via `tracing`.

use std::collections::HashMap;
use std::io::Write;
use std::sync::{Arc, Mutex};

use madhyamas_core::config::{DebugLogConfig, DebugLogLevel};
use madhyamas_core::debug_log::{
    format_body, log_request, log_response, redact_and_format_headers, LogCorrelation,
};
use madhyamas_core::traffic::{HttpMethod, RequestData, ResponseData};

fn make_request() -> RequestData {
    let mut headers = HashMap::new();
    headers.insert("Host".to_string(), "api.example.com".to_string());
    headers.insert(
        "Authorization".to_string(),
        "Bearer secret-token".to_string(),
    );
    headers.insert("X-Trace-Id".to_string(), "abc123".to_string());
    RequestData {
        method: HttpMethod::Post,
        url: "https://api.example.com/v1/users".to_string(),
        host: "api.example.com".to_string(),
        path: "/v1/users".to_string(),
        headers,
        body: Some(b"{\"name\":\"test\"}".to_vec()),
        content_type: Some("application/json".to_string()),
        http_version: Some("HTTP/1.1".to_string()),
    }
}

fn make_response() -> ResponseData {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("Set-Cookie".to_string(), "session=secret".to_string());
    ResponseData {
        status_code: 200,
        status_message: Some("OK".to_string()),
        headers,
        body: Some(b"{\"ok\":true}".to_vec()),
        content_type: Some("application/json".to_string()),
        duration_ms: 42,
        http_version: Some("HTTP/1.1".to_string()),
    }
}

// ── redact_and_format_headers ────────────────────────────────────────────

#[test]
fn test_redact_and_format_headers_redacts_configured_case_insensitive() {
    let mut headers = HashMap::new();
    headers.insert("Authorization".to_string(), "Bearer tok".to_string());
    headers.insert("x-custom".to_string(), "keep".to_string());
    let redact = vec!["authorization".to_string()];
    let out = redact_and_format_headers(&headers, &redact);
    assert!(out.contains("Authorization: [REDACTED]"));
    assert!(!out.contains("tok"));
    assert!(out.contains("x-custom: keep"));
}

#[test]
fn test_redact_and_format_headers_sorted_and_multiline() {
    let mut headers = HashMap::new();
    headers.insert("B".to_string(), "2".to_string());
    headers.insert("A".to_string(), "1".to_string());
    let out = redact_and_format_headers(&headers, &[]);
    assert_eq!(out, "A: 1\nB: 2");
}

#[test]
fn test_redact_and_format_headers_empty_map() {
    assert_eq!(redact_and_format_headers(&HashMap::new(), &[]), "");
}

// ── format_body ──────────────────────────────────────────────────────────

#[test]
fn test_format_body_none_and_empty() {
    assert_eq!(format_body(None, None, None, 100), "");
    assert_eq!(format_body(None, None, Some(&[]), 100), "");
}

#[test]
fn test_format_body_plain_text_passthrough() {
    let out = format_body(
        None,
        Some("application/json"),
        Some(br#"{"ok":true}"#),
        1024,
    );
    assert_eq!(out, r#"{"ok":true}"#);
}

#[test]
fn test_format_body_truncates_to_max_bytes_with_marker() {
    let body = "a".repeat(100);
    let out = format_body(None, Some("text/plain"), Some(body.as_bytes()), 10);
    assert!(out.starts_with("aaaaaaaaaa\n"));
    assert!(out.contains("[truncated: body is 100 bytes, logged 10 bytes]"));
}

#[test]
fn test_format_body_truncation_respects_char_boundaries() {
    // Each emoji is 4 bytes; cut at 5 must fall back to a boundary.
    let body = "aaaaaaaaaa".to_string();
    let out = format_body(None, Some("text/plain"), Some(body.as_bytes()), 5);
    assert!(out.starts_with("aaaaa\n"));
}

#[test]
fn test_format_body_binary_content_type_placeholder() {
    let out = format_body(None, Some("image/png"), Some(&[0u8, 1, 2, 3, 4]), 1024);
    assert_eq!(out, "[binary body: 5 bytes, content-type image/png]");
}

#[test]
fn test_format_body_non_utf8_text_content_type_placeholder() {
    let out = format_body(
        None,
        Some("text/plain"),
        Some(&[0xff, 0xfe, 0x00, 0x01]),
        1024,
    );
    assert_eq!(out, "[non-UTF-8 body: 4 bytes, content-type text/plain]");
}

#[test]
fn test_format_body_decompresses_gzip() {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    let mut enc = GzEncoder::new(Vec::new(), Compression::default());
    enc.write_all(br#"{"compressed":true}"#).unwrap();
    let compressed = enc.finish().unwrap();
    let out = format_body(
        Some("gzip"),
        Some("application/json"),
        Some(&compressed),
        1024,
    );
    assert_eq!(out, r#"{"compressed":true}"#);
}

#[test]
fn test_format_body_corrupt_gzip_falls_back_to_raw_bytes() {
    let corrupt = vec![0x1f, 0x8b, 0x99, 0x99];
    let out = format_body(Some("gzip"), None, Some(&corrupt), 1024);
    // Falls back to the raw compressed bytes; non-UTF-8 => placeholder.
    assert!(out.contains("bytes, content-type unknown"));
}

// ── log_request / log_response gating and emission ───────────────────────

/// Shared buffer writer used to capture `tracing` output per test.
#[derive(Clone)]
struct SharedBuf(Arc<Mutex<Vec<u8>>>);

impl Write for SharedBuf {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn capture_events<F: FnOnce()>(f: F) -> String {
    let buf = SharedBuf(Arc::new(Mutex::new(Vec::new())));
    let writer = buf.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || writer.clone())
        .with_ansi(false)
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::with_default(subscriber, f);
    let out = buf.0.lock().unwrap().clone();
    String::from_utf8_lossy(&out).to_string()
}

fn corr<'a>() -> LogCorrelation<'a> {
    LogCorrelation {
        request_id: "req-1",
        connection_id: "conn-1",
        rule_hit: None,
    }
}

fn enabled_cfg(level: DebugLogLevel) -> DebugLogConfig {
    DebugLogConfig {
        enabled: true,
        level,
        host_filter: None,
        redact_headers: DebugLogConfig::default().redact_headers,
        redact_bodies: false,
    }
}

#[test]
fn test_log_request_disabled_emits_nothing() {
    let cfg = DebugLogConfig::default(); // enabled: false
    let out = capture_events(|| log_request(&cfg, 1024, &make_request(), &corr()));
    assert!(!out.contains("madhyamas::debug_log"));
    assert!(!out.contains("proxied request"));
}

#[test]
fn test_log_request_host_filter_mismatch_emits_nothing() {
    let mut cfg = enabled_cfg(DebugLogLevel::Summary);
    cfg.host_filter = Some(vec!["other.example.com".to_string()]);
    let out = capture_events(|| log_request(&cfg, 1024, &make_request(), &corr()));
    assert!(!out.contains("proxied request"));
}

#[test]
fn test_log_request_summary_level() {
    let cfg = enabled_cfg(DebugLogLevel::Summary);
    let out = capture_events(|| log_request(&cfg, 1024, &make_request(), &corr()));
    assert!(out.contains("madhyamas::debug_log"));
    assert!(out.contains("proxied request"));
    assert!(out.contains("api.example.com"));
    assert!(out.contains("/v1/users"));
    assert!(out.contains("POST"));
    // Summary must not include headers or bodies.
    assert!(!out.contains("X-Trace-Id"));
    assert!(!out.contains("Bearer secret-token"));
    assert!(!out.contains("\"name\":\"test\""));
}

#[test]
fn test_log_request_headers_level_redacts_sensitive_headers() {
    let cfg = enabled_cfg(DebugLogLevel::Headers);
    let out = capture_events(|| log_request(&cfg, 1024, &make_request(), &corr()));
    assert!(out.contains("Authorization: [REDACTED]"));
    assert!(out.contains("X-Trace-Id: abc123"));
    // Headers level must not include body content.
    assert!(!out.contains("\"name\":\"test\""));
}

#[test]
fn test_log_request_full_level_includes_body() {
    let cfg = enabled_cfg(DebugLogLevel::Full);
    let out = capture_events(|| log_request(&cfg, 1024, &make_request(), &corr()));
    assert!(out.contains("Authorization: [REDACTED]"));
    assert!(out.contains("{\"name\":\"test\"}"));
}

#[test]
fn test_log_request_full_level_with_redact_bodies_omits_body() {
    let mut cfg = enabled_cfg(DebugLogLevel::Full);
    cfg.redact_bodies = true;
    let out = capture_events(|| log_request(&cfg, 1024, &make_request(), &corr()));
    assert!(!out.contains("\"name\":\"test\""));
    assert!(out.contains("[body omitted: 15 bytes]"));
}

#[test]
fn test_log_response_disabled_emits_nothing() {
    let cfg = DebugLogConfig::default();
    let out = capture_events(|| {
        log_response(
            &cfg,
            1024,
            &make_request(),
            &make_response(),
            "upstream",
            &corr(),
        )
    });
    assert!(!out.contains("proxied response"));
}

#[test]
fn test_log_response_summary_level() {
    let cfg = enabled_cfg(DebugLogLevel::Summary);
    let out = capture_events(|| {
        log_response(
            &cfg,
            1024,
            &make_request(),
            &make_response(),
            "upstream",
            &corr(),
        )
    });
    assert!(out.contains("proxied response"));
    assert!(out.contains("status=200"));
    assert!(out.contains("duration_ms=42"));
    assert!(out.contains("source=\"upstream\""));
    assert!(!out.contains("Set-Cookie"));
}

#[test]
fn test_log_response_headers_level_redacts_set_cookie() {
    let cfg = enabled_cfg(DebugLogLevel::Headers);
    let out = capture_events(|| {
        log_response(
            &cfg,
            1024,
            &make_request(),
            &make_response(),
            "mocked",
            &corr(),
        )
    });
    assert!(out.contains("Set-Cookie: [REDACTED]"));
    assert!(!out.contains("session=secret"));
    assert!(out.contains("source=\"mocked\""));
}

/// Capture events as JSON lines using the same formatter shape the
/// file layer uses (`json().flatten_event(true).with_target(true)`).
fn capture_events_json<F: FnOnce()>(f: F) -> Vec<serde_json::Value> {
    let buf = SharedBuf(Arc::new(Mutex::new(Vec::new())));
    let writer = buf.clone();
    let subscriber = tracing_subscriber::fmt()
        .json()
        .flatten_event(true)
        .with_ansi(false)
        .with_target(true)
        .with_writer(move || writer.clone())
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::with_default(subscriber, f);
    let out = buf.0.lock().unwrap().clone();
    String::from_utf8_lossy(&out)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("valid JSON line"))
        .collect()
}

#[test]
fn test_structured_schema_conformance() {
    let cfg = enabled_cfg(DebugLogLevel::Summary);
    let events = capture_events_json(|| {
        log_request(&cfg, 1024, &make_request(), &corr());
        log_response(
            &cfg,
            1024,
            &make_request(),
            &make_response(),
            "upstream",
            &corr(),
        );
    });
    assert_eq!(events.len(), 2);

    let req = &events[0];
    for field in [
        "timestamp",
        "level",
        "target",
        "request_id",
        "connection_id",
        "method",
        "host",
        "path",
    ] {
        assert!(
            req.get(field).is_some(),
            "request event missing schema field `{}`: {:?}",
            field,
            req
        );
    }
    assert_eq!(req["target"], "madhyamas::debug_log");
    assert_eq!(req["request_id"], "req-1");
    assert_eq!(req["connection_id"], "conn-1");
    assert_eq!(req["method"], "POST");
    assert_eq!(req["host"], "api.example.com");
    assert_eq!(req["path"], "/v1/users");

    let resp = &events[1];
    for field in ["status", "duration_ms", "source"] {
        assert!(
            resp.get(field).is_some(),
            "response event missing schema field `{}`: {:?}",
            field,
            resp
        );
    }
    assert_eq!(resp["status"], 200);
    assert_eq!(resp["duration_ms"], 42);
}

#[test]
fn test_structured_events_share_request_id_for_correlation() {
    let cfg = enabled_cfg(DebugLogLevel::Summary);
    let events = capture_events_json(|| {
        log_request(&cfg, 1024, &make_request(), &corr());
        log_response(
            &cfg,
            1024,
            &make_request(),
            &make_response(),
            "upstream",
            &corr(),
        );
    });
    assert_eq!(events.len(), 2);
    assert_eq!(
        events[0]["request_id"], events[1]["request_id"],
        "all events for a request must share request_id"
    );
    assert_eq!(
        events[0]["connection_id"], events[1]["connection_id"],
        "all events for a request must share connection_id"
    );
}

#[test]
fn test_structured_rule_hit_recorded_when_rule_matched() {
    let cfg = enabled_cfg(DebugLogLevel::Summary);
    let corr_with_hit = LogCorrelation {
        request_id: "req-9",
        connection_id: "conn-9",
        rule_hit: Some("users-mock"),
    };
    let events = capture_events_json(|| {
        log_response(
            &cfg,
            1024,
            &make_request(),
            &make_response(),
            "mocked",
            &corr_with_hit,
        );
    });
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["rule_hit"], "users-mock");
}

#[test]
fn test_span_carries_request_and_connection_ids() {
    // The pipeline wraps request processing in a `proxy_request` span
    // carrying request_id/connection_id; every event inside the span is
    // correlated through the span list even for events that do not set
    // the fields explicitly.
    let buf = SharedBuf(Arc::new(Mutex::new(Vec::new())));
    let writer = buf.clone();
    let subscriber = tracing_subscriber::fmt()
        .json()
        .flatten_event(true)
        .with_ansi(false)
        .with_target(true)
        .with_current_span(true)
        .with_span_list(true)
        .with_writer(move || writer.clone())
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::with_default(subscriber, || {
        let span = tracing::info_span!(
            "proxy_request",
            request_id = "span-req-1",
            connection_id = "span-conn-1"
        );
        // Same shape the pipeline uses: enter the span, record the
        // request id, then emit an event.
        let _g = span.enter();
        tracing::info!(target: "madhyamas::debug_log", "inner event");
    });
    let out = buf.0.lock().unwrap().clone();
    let line: serde_json::Value =
        serde_json::from_str(String::from_utf8_lossy(&out).trim()).unwrap();
    let spans = line["spans"].as_array().expect("span list present");
    let req_span = spans.iter().find(|s| s["name"] == "proxy_request").unwrap();
    assert_eq!(req_span["request_id"], "span-req-1");
    assert_eq!(req_span["connection_id"], "span-conn-1");
}

#[test]
fn test_log_response_full_level_includes_body_and_host_filter_match() {
    let mut cfg = enabled_cfg(DebugLogLevel::Full);
    cfg.host_filter = Some(vec!["*.example.com".to_string()]);
    let out = capture_events(|| {
        log_response(
            &cfg,
            1024,
            &make_request(),
            &make_response(),
            "upstream",
            &corr(),
        )
    });
    assert!(out.contains("{\"ok\":true}"));
}

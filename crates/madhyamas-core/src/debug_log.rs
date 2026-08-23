//! Proxied-traffic debug logging.
//!
//! Emits structured `tracing` events on the dedicated target
//! `madhyamas::debug_log` for each proxied request and response, gated at
//! runtime by [`DebugLogConfig`](crate::config::DebugLogConfig) (no restart
//! required). Events are mixed into the existing main rotated log / stdout
//! and therefore honor the `LogConfig` JSON-format / ANSI behavior.
//!
//! Verbosity levels:
//!
//! - `summary` — one line per request/response (method, host, path, status,
//!   timing)
//! - `headers` — summary plus all headers (sensitive headers redacted)
//! - `full` — headers plus bodies, size-capped at the traffic-capture
//!   `max_body_size`; compressed bodies are decompressed before logging;
//!   non-text binaries are replaced with a size/content-type placeholder

use crate::config::{DebugLogConfig, DebugLogLevel};
use crate::proxy::pipeline::Pipeline;
use crate::traffic::{RequestData, ResponseData};
use std::collections::HashMap;

/// The `tracing` target used for all proxied-traffic debug events.
pub const DEBUG_LOG_TARGET: &str = "madhyamas::debug_log";

/// Placeholder substituted for redacted header values.
const REDACTED: &str = "[REDACTED]";

/// Render a header map for logging, replacing redacted header values.
///
/// Matching against the configured redaction list is case-insensitive.
/// Headers are rendered one per line as `Name: Value` and sorted by name
/// for deterministic output.
pub fn redact_and_format_headers(headers: &HashMap<String, String>, redact: &[String]) -> String {
    let mut names: Vec<&String> = headers.keys().collect();
    names.sort();
    names
        .into_iter()
        .map(|name| {
            let redacted = redact
                .iter()
                .any(|r| r.trim().eq_ignore_ascii_case(name.trim()));
            if redacted {
                format!("{}: {}", name, REDACTED)
            } else {
                format!("{}: {}", name, headers[name])
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Whether a content type should be treated as textual (loggable as UTF-8).
fn is_text_content_type(content_type: Option<&str>) -> bool {
    match content_type {
        None => true,
        Some(ct) => {
            let ct = ct.split(';').next().unwrap_or("").trim().to_lowercase();
            ct.starts_with("text/")
                || ct.contains("json")
                || ct.contains("xml")
                || ct.contains("javascript")
                || ct.contains("urlencoded")
                || ct.contains("yaml")
                || ct.contains("csv")
                || ct.is_empty()
        }
    }
}

/// Render a body for logging at `full` verbosity.
///
/// Compressed bodies (gzip, deflate, br, zstd — per the `Content-Encoding`
/// header) are decompressed via the pipeline's shared `decompress_body`
/// helper first. The result is truncated to `max_bytes` (the traffic-capture
/// `max_body_size`). Non-text bodies (images, protobuf, ...) are replaced
/// with a placeholder carrying the size and content type.
pub fn format_body(
    content_encoding: Option<&str>,
    content_type: Option<&str>,
    body: Option<&[u8]>,
    max_bytes: usize,
) -> String {
    let Some(bytes) = body else {
        return String::new();
    };
    if bytes.is_empty() {
        return String::new();
    }

    // Decompress if needed (best effort; falls back to raw bytes). Bodies
    // already larger than the cap are never decompressed — the placeholder
    // below reports the size — which also bounds decompression-bomb
    // expansion (input is capped, so output is capped by the compression
    // ratio of a body the proxy already accepted).
    let effective: Vec<u8> = match content_encoding {
        None | Some("") => bytes.to_vec(),
        Some(_) if bytes.len() > max_bytes => bytes.to_vec(),
        Some(encoding) => {
            let mut scratch_headers = HashMap::new();
            Pipeline::decompress_body(Some(encoding), bytes.to_vec(), &mut scratch_headers)
                .unwrap_or_else(|| bytes.to_vec())
        }
    };

    if !is_text_content_type(content_type) {
        let ct = content_type.unwrap_or("unknown");
        return format!(
            "[binary body: {} bytes, content-type {}]",
            effective.len(),
            ct
        );
    }

    match String::from_utf8(effective.clone()) {
        Ok(text) => {
            if text.len() > max_bytes {
                let mut cut = max_bytes;
                while !text.is_char_boundary(cut) {
                    cut -= 1;
                }
                format!(
                    "{}\n[truncated: body is {} bytes, logged {} bytes]",
                    &text[..cut],
                    text.len(),
                    cut
                )
            } else {
                text
            }
        }
        Err(_) => {
            let ct = content_type.unwrap_or("unknown");
            format!(
                "[non-UTF-8 body: {} bytes, content-type {}]",
                effective.len(),
                ct
            )
        }
    }
}

/// Log an incoming proxied request at the configured verbosity.
pub fn log_request(cfg: &DebugLogConfig, max_body_size: usize, request: &RequestData) {
    if !cfg.should_log(&request.host) {
        return;
    }
    let method = request.method.to_string();
    match cfg.level {
        DebugLogLevel::Summary => {
            tracing::info!(
                target: DEBUG_LOG_TARGET,
                direction = "request",
                method = %method,
                host = %request.host,
                path = %request.path,
                "proxied request"
            );
        }
        DebugLogLevel::Headers | DebugLogLevel::Full => {
            let headers = redact_and_format_headers(&request.headers, &cfg.redact_headers);
            let body = if cfg.level == DebugLogLevel::Full && !cfg.redact_bodies {
                format_body(
                    request
                        .headers
                        .get("Content-Encoding")
                        .map(|s| s.as_str())
                        .or_else(|| request.headers.get("content-encoding").map(|s| s.as_str())),
                    request.content_type.as_deref(),
                    request.body.as_deref(),
                    max_body_size,
                )
            } else {
                body_placeholder(cfg, request.body.as_deref())
            };
            tracing::info!(
                target: DEBUG_LOG_TARGET,
                direction = "request",
                method = %method,
                host = %request.host,
                path = %request.path,
                headers = %headers,
                body = %body,
                "proxied request"
            );
        }
    }
}

/// Log a proxied response at the configured verbosity.
///
/// `source` describes where the response came from (`upstream`, `mocked`,
/// `blocked`, `script`, `breakpoint response`, or `error`).
pub fn log_response(
    cfg: &DebugLogConfig,
    max_body_size: usize,
    request: &RequestData,
    response: &ResponseData,
    source: &str,
) {
    if !cfg.should_log(&request.host) {
        return;
    }
    let method = request.method.to_string();
    match cfg.level {
        DebugLogLevel::Summary => {
            tracing::info!(
                target: DEBUG_LOG_TARGET,
                direction = "response",
                method = %method,
                host = %request.host,
                path = %request.path,
                status = response.status_code,
                duration_ms = response.duration_ms,
                source = source,
                "proxied response"
            );
        }
        DebugLogLevel::Headers | DebugLogLevel::Full => {
            let headers = redact_and_format_headers(&response.headers, &cfg.redact_headers);
            let body = if cfg.level == DebugLogLevel::Full && !cfg.redact_bodies {
                format_body(
                    response
                        .headers
                        .get("Content-Encoding")
                        .map(|s| s.as_str())
                        .or_else(|| response.headers.get("content-encoding").map(|s| s.as_str())),
                    response
                        .content_type
                        .as_deref()
                        .or_else(|| response.headers.get("Content-Type").map(|s| s.as_str())),
                    response.body.as_deref(),
                    max_body_size,
                )
            } else {
                body_placeholder(cfg, response.body.as_deref())
            };
            tracing::info!(
                target: DEBUG_LOG_TARGET,
                direction = "response",
                method = %method,
                host = %request.host,
                path = %request.path,
                status = response.status_code,
                duration_ms = response.duration_ms,
                source = source,
                headers = %headers,
                body = %body,
                "proxied response"
            );
        }
    }
}

/// Body rendering when bodies are not logged (redaction on, or verbosity
/// below `full`): a size-only placeholder.
fn body_placeholder(_cfg: &DebugLogConfig, body: Option<&[u8]>) -> String {
    match body {
        None | Some(&[]) => String::new(),
        Some(bytes) => format!("[body omitted: {} bytes]", bytes.len()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traffic::{HttpMethod, RequestData, ResponseData};
    use std::io::Write;
    use std::sync::{Arc, Mutex};

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

    // ── is_text_content_type ─────────────────────────────────────────────────

    #[test]
    fn test_is_text_content_type_variants() {
        assert!(is_text_content_type(None));
        assert!(is_text_content_type(Some("text/html; charset=utf-8")));
        assert!(is_text_content_type(Some("application/json")));
        assert!(is_text_content_type(Some("application/xml")));
        assert!(is_text_content_type(Some("application/javascript")));
        assert!(is_text_content_type(Some(
            "application/x-www-form-urlencoded"
        )));
        assert!(is_text_content_type(Some("text/csv")));
        assert!(is_text_content_type(Some("application/yaml")));
        assert!(!is_text_content_type(Some("image/png")));
        assert!(!is_text_content_type(Some("application/octet-stream")));
        assert!(!is_text_content_type(Some("application/grpc-proto")));
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

    // ── body_placeholder ─────────────────────────────────────────────────────

    #[test]
    fn test_body_placeholder_shapes() {
        let cfg = DebugLogConfig::default();
        assert_eq!(body_placeholder(&cfg, None), "");
        assert_eq!(body_placeholder(&cfg, Some(&[])), "");
        assert_eq!(
            body_placeholder(&cfg, Some(&[1, 2, 3])),
            "[body omitted: 3 bytes]"
        );
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
        let out = capture_events(|| log_request(&cfg, 1024, &make_request()));
        assert!(!out.contains("madhyamas::debug_log"));
        assert!(!out.contains("proxied request"));
    }

    #[test]
    fn test_log_request_host_filter_mismatch_emits_nothing() {
        let mut cfg = enabled_cfg(DebugLogLevel::Summary);
        cfg.host_filter = Some(vec!["other.example.com".to_string()]);
        let out = capture_events(|| log_request(&cfg, 1024, &make_request()));
        assert!(!out.contains("proxied request"));
    }

    #[test]
    fn test_log_request_summary_level() {
        let cfg = enabled_cfg(DebugLogLevel::Summary);
        let out = capture_events(|| log_request(&cfg, 1024, &make_request()));
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
        let out = capture_events(|| log_request(&cfg, 1024, &make_request()));
        assert!(out.contains("Authorization: [REDACTED]"));
        assert!(out.contains("X-Trace-Id: abc123"));
        // Headers level must not include body content.
        assert!(!out.contains("\"name\":\"test\""));
    }

    #[test]
    fn test_log_request_full_level_includes_body() {
        let cfg = enabled_cfg(DebugLogLevel::Full);
        let out = capture_events(|| log_request(&cfg, 1024, &make_request()));
        assert!(out.contains("Authorization: [REDACTED]"));
        assert!(out.contains("{\"name\":\"test\"}"));
    }

    #[test]
    fn test_log_request_full_level_with_redact_bodies_omits_body() {
        let mut cfg = enabled_cfg(DebugLogLevel::Full);
        cfg.redact_bodies = true;
        let out = capture_events(|| log_request(&cfg, 1024, &make_request()));
        assert!(!out.contains("\"name\":\"test\""));
        assert!(out.contains("[body omitted: 15 bytes]"));
    }

    #[test]
    fn test_log_response_disabled_emits_nothing() {
        let cfg = DebugLogConfig::default();
        let out = capture_events(|| {
            log_response(&cfg, 1024, &make_request(), &make_response(), "upstream")
        });
        assert!(!out.contains("proxied response"));
    }

    #[test]
    fn test_log_response_summary_level() {
        let cfg = enabled_cfg(DebugLogLevel::Summary);
        let out = capture_events(|| {
            log_response(&cfg, 1024, &make_request(), &make_response(), "upstream")
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
            log_response(&cfg, 1024, &make_request(), &make_response(), "mocked")
        });
        assert!(out.contains("Set-Cookie: [REDACTED]"));
        assert!(!out.contains("session=secret"));
        assert!(out.contains("source=\"mocked\""));
    }

    #[test]
    fn test_log_response_full_level_includes_body_and_host_filter_match() {
        let mut cfg = enabled_cfg(DebugLogLevel::Full);
        cfg.host_filter = Some(vec!["*.example.com".to_string()]);
        let out = capture_events(|| {
            log_response(&cfg, 1024, &make_request(), &make_response(), "upstream")
        });
        assert!(out.contains("{\"ok\":true}"));
    }
}

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

/// Per-request correlation context attached to every debug-log event.
///
/// All events emitted while a request is being processed carry the same
/// `request_id`, and every request on the same client connection carries
/// the same `connection_id` — this is the stable correlation contract of
/// the structured log schema (see `docs/LOGGING.md`). `rule_hit` names the
/// intercept rule (e.g. a mock) that matched the request, when known.
pub struct LogCorrelation<'a> {
    pub request_id: &'a str,
    pub connection_id: &'a str,
    pub rule_hit: Option<&'a str>,
}

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
pub fn log_request(
    cfg: &DebugLogConfig,
    max_body_size: usize,
    request: &RequestData,
    corr: &LogCorrelation<'_>,
) {
    if !cfg.should_log(&request.host) {
        return;
    }
    let method = request.method.to_string();
    match cfg.level {
        DebugLogLevel::Summary => {
            tracing::info!(
                target: DEBUG_LOG_TARGET,
                direction = "request",
                request_id = %corr.request_id,
                connection_id = %corr.connection_id,
                method = %method,
                host = %request.host,
                path = %request.path,
                rule_hit = corr.rule_hit,
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
                request_id = %corr.request_id,
                connection_id = %corr.connection_id,
                method = %method,
                host = %request.host,
                path = %request.path,
                rule_hit = corr.rule_hit,
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
    corr: &LogCorrelation<'_>,
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
                request_id = %corr.request_id,
                connection_id = %corr.connection_id,
                method = %method,
                host = %request.host,
                path = %request.path,
                status = response.status_code,
                duration_ms = response.duration_ms,
                source = source,
                rule_hit = corr.rule_hit,
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
                request_id = %corr.request_id,
                connection_id = %corr.connection_id,
                method = %method,
                host = %request.host,
                path = %request.path,
                status = response.status_code,
                duration_ms = response.duration_ms,
                source = source,
                rule_hit = corr.rule_hit,
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
}

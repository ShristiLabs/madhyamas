//! Shared redaction core (#87).
//!
//! One [`Redactor`] is applied at every egress point where secret material
//! could otherwise surface: traffic capture (headers/bodies), HAR export,
//! plugin logs (`GET /api/plugins/{id}/logs`), script traces
//! (`/traffic/{id}/script-traces`), and web UI responses.
//!
//! Two mechanisms, applied together:
//!
//! 1. **Header patterns** — configurable header names (Authorization,
//!    Cookie, Set-Cookie, ...) whose values are always replaced with
//!    `[REDACTED]`, matching case-insensitively.
//! 2. **Best-effort value matching** — known secret plaintexts (everything
//!    in the secret store) are replaced with `[REDACTED]` wherever they
//!    appear, including bodies and log lines. Values shorter than 4
//!    characters are skipped to avoid clobbering common substrings.

use std::collections::HashMap;

/// Placeholder substituted for redacted content.
pub const REDACTED: &str = "[REDACTED]";

/// Minimum length a secret value must have to be value-matched.
const MIN_VALUE_LEN: usize = 4;

/// Default header names that are always redacted unless configured
/// otherwise.
pub const DEFAULT_REDACT_HEADERS: &[&str] = &[
    "authorization",
    "proxy-authorization",
    "cookie",
    "set-cookie",
    "x-api-key",
];

/// Shared redaction engine: header patterns + known secret values.
#[derive(Debug, Clone, Default)]
pub struct Redactor {
    /// Lowercased header names whose values are always redacted.
    headers: Vec<String>,
    /// Known secret plaintexts (from the secret store) to value-match.
    values: Vec<String>,
}

impl Redactor {
    /// Build a redactor from configured header names (compared
    /// case-insensitively) and known secret values.
    pub fn new(
        header_patterns: &[String],
        secret_values: impl IntoIterator<Item = String>,
    ) -> Self {
        Self {
            headers: header_patterns
                .iter()
                .map(|h| h.trim().to_lowercase())
                .filter(|h| !h.is_empty())
                .collect(),
            values: secret_values
                .into_iter()
                .filter(|v| v.len() >= MIN_VALUE_LEN)
                .collect(),
        }
    }

    /// Build with the default header patterns.
    pub fn with_defaults(secret_values: impl IntoIterator<Item = String>) -> Self {
        Self::new(
            &DEFAULT_REDACT_HEADERS
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<_>>(),
            secret_values,
        )
    }

    /// Whether a header name (any case) is redacted.
    pub fn is_redacted_header(&self, name: &str) -> bool {
        let lower = name.trim().to_lowercase();
        self.headers.iter().any(|h| h == &lower)
    }

    /// Redact a header map in place (both by header pattern and by value
    /// match).
    pub fn redact_header_map(&self, headers: &mut HashMap<String, String>) {
        for (name, value) in headers.iter_mut() {
            if self.is_redacted_header(name) || self.matches_value(value) {
                *value = REDACTED.to_string();
            }
        }
    }

    /// Best-effort replace of known secret values in free text.
    pub fn redact_text(&self, text: &str) -> String {
        let mut out = text.to_string();
        for v in &self.values {
            if out.contains(v.as_str()) {
                out = out.replace(v.as_str(), REDACTED);
            }
        }
        out
    }

    fn matches_value(&self, text: &str) -> bool {
        self.values.iter().any(|v| text.contains(v.as_str()))
    }

    /// Recursively redact a JSON document: objects under a `headers` key
    /// get header-pattern redaction, and every string is value-matched.
    pub fn redact_json(&self, value: &mut serde_json::Value) {
        self.redact_json_inner(value, false);
    }

    fn redact_json_inner(&self, value: &mut serde_json::Value, in_headers: bool) {
        match value {
            serde_json::Value::String(s) => {
                // Header-name context is applied by the parent (the object
                // arm), which knows the key.
                let _ = in_headers;
                *s = self.redact_text(s);
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    self.redact_json_inner(item, false);
                }
            }
            serde_json::Value::Object(map) => {
                for (k, v) in map.iter_mut() {
                    let child_in_headers = in_headers || k.eq_ignore_ascii_case("headers");
                    self.redact_json_inner(v, child_in_headers);
                    if in_headers {
                        if let serde_json::Value::String(s) = v {
                            if self.is_redacted_header(k) {
                                *s = REDACTED.to_string();
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Redact a list of log/trace lines in place.
    pub fn redact_lines(&self, lines: &mut [String]) {
        for line in lines.iter_mut() {
            *line = self.redact_text(line);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn redactor() -> Redactor {
        Redactor::with_defaults(vec![
            "super-secret-token-123".to_string(),
            "abc".to_string(), // below MIN_VALUE_LEN -> ignored
            "pwd-9876".to_string(),
        ])
    }

    #[test]
    fn redacts_sensitive_headers_case_insensitive() {
        let r = Redactor::with_defaults(vec![]);
        let mut h = HashMap::new();
        h.insert("Authorization".to_string(), "Bearer x".to_string());
        h.insert("COOKIE".to_string(), "a=b".to_string());
        h.insert("X-Custom".to_string(), "keep".to_string());
        r.redact_header_map(&mut h);
        assert_eq!(h["Authorization"], REDACTED);
        assert_eq!(h["COOKIE"], REDACTED);
        assert_eq!(h["X-Custom"], "keep");
    }

    #[test]
    fn value_matching_replaces_known_values_in_text() {
        let r = redactor();
        assert_eq!(
            r.redact_text("Bearer super-secret-token-123 done"),
            format!("Bearer {} done", REDACTED)
        );
        // "abc" is below the length floor and must NOT be replaced.
        assert_eq!(r.redact_text("a abc string"), "a abc string");
        assert_eq!(r.redact_text("pwd-9876"), REDACTED);
    }

    #[test]
    fn value_matching_applies_to_header_values() {
        let r = redactor();
        let mut h = HashMap::new();
        h.insert("X-Token".to_string(), "super-secret-token-123".to_string());
        r.redact_header_map(&mut h);
        assert_eq!(h["X-Token"], REDACTED);
    }

    #[test]
    fn redact_json_headers_and_strings() {
        let r = redactor();
        let mut v: serde_json::Value = serde_json::json!({
            "request": {
                "method": "POST",
                "headers": { "Authorization": "Bearer abc", "Accept": "*/*" },
                "body": "token=super-secret-token-123"
            },
            "log": "pwd-9876 leaked"
        });
        r.redact_json(&mut v);
        assert_eq!(v["request"]["headers"]["Authorization"], REDACTED);
        assert_eq!(v["request"]["headers"]["Accept"], "*/*");
        assert_eq!(v["request"]["body"], format!("token={}", REDACTED));
        assert_eq!(v["log"], format!("{} leaked", REDACTED));
    }

    #[test]
    fn redact_lines() {
        let r = redactor();
        let mut lines = vec!["ok".to_string(), "v=super-secret-token-123".to_string()];
        r.redact_lines(&mut lines);
        assert_eq!(lines[0], "ok");
        assert_eq!(lines[1], format!("v={}", REDACTED));
    }

    #[test]
    fn custom_header_patterns() {
        let r = Redactor::new(&["X-Internal".to_string()], vec![]);
        assert!(r.is_redacted_header("x-internal"));
        assert!(!r.is_redacted_header("authorization"));
    }

    #[test]
    fn empty_redactor_is_noop_but_safe() {
        let r = Redactor::default();
        assert_eq!(r.redact_text("anything"), "anything");
        let mut v = serde_json::json!({"headers": {"Authorization": "x"}});
        r.redact_json(&mut v);
        assert_eq!(v["headers"]["Authorization"], "x");
    }
}

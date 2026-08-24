//! `${ENV:VAR}` / `${SECRET:name}` placeholder substitution.
//!
//! Placeholders are expanded only for names that appear in the caller's
//! [`Grants`] set. Ungranted (or unresolvable) placeholders are left
//! untouched so a plugin without grants sees no behavioral change and never
//! learns whether a name exists.

use std::collections::HashSet;

/// The set of env-var and secret names granted to a single plugin or script.
///
/// Deny by default: both sets start empty.
#[derive(Debug, Clone, Default)]
pub struct Grants {
    /// Process environment variable names the consumer may read.
    pub env: HashSet<String>,
    /// Secret-store names the consumer may receive.
    pub secrets: HashSet<String>,
}

impl Grants {
    pub fn new(env: Vec<String>, secrets: Vec<String>) -> Self {
        Self {
            env: env.into_iter().collect(),
            secrets: secrets.into_iter().collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.env.is_empty() && self.secrets.is_empty()
    }
}

/// Resolution result for one placeholder.
enum Resolved {
    /// Substitute this value.
    Value(String),
    /// Not granted (or not resolvable) — leave the placeholder untouched.
    Unresolved,
}

/// Expand all placeholders in `input`.
///
/// `env_lookup` and `secret_lookup` are only invoked for names present in
/// `grants`, so the lookups themselves cannot be used as an oracle for
/// ungranted names.
pub fn expand_str(
    input: &str,
    grants: &Grants,
    env_lookup: &dyn Fn(&str) -> Option<String>,
    secret_lookup: &dyn Fn(&str) -> Option<String>,
) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'$' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            // Find the closing brace (no nesting support — a nested
            // placeholder inside an ungranted one stays untouched).
            if let Some(close) = input[i + 2..].find('}') {
                let inner = &input[i + 2..i + 2 + close];
                let resolved = match inner.strip_prefix("ENV:") {
                    Some(name) => {
                        if grants.env.contains(name) {
                            match env_lookup(name) {
                                Some(v) => Resolved::Value(v),
                                None => Resolved::Unresolved,
                            }
                        } else {
                            Resolved::Unresolved
                        }
                    }
                    None => match inner.strip_prefix("SECRET:") {
                        Some(name) => {
                            if grants.secrets.contains(name) {
                                match secret_lookup(name) {
                                    Some(v) => Resolved::Value(v),
                                    None => Resolved::Unresolved,
                                }
                            } else {
                                Resolved::Unresolved
                            }
                        }
                        None => Resolved::Unresolved,
                    },
                };
                match resolved {
                    Resolved::Value(v) => {
                        out.push_str(&v);
                        i += 2 + close + 1;
                        continue;
                    }
                    Resolved::Unresolved => {
                        // Fall through: copy the placeholder verbatim.
                    }
                }
            }
        }
        // Advance one character (UTF-8 safe: copy the full char).
        let ch = input[i..].chars().next().unwrap_or('$');
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// Expand `${ENV:...}` placeholders granted to the given grant set, reading
/// from the process environment. Secrets are not touched by this helper.
pub fn expand_grants_str(
    input: &str,
    grants: &Grants,
    secret_lookup: &dyn Fn(&str) -> Option<String>,
) -> String {
    expand_str(
        input,
        grants,
        &|name| std::env::var(name).ok(),
        secret_lookup,
    )
}

/// Recursively expand placeholders in every string of a JSON value
/// (typically a plugin's settings map or a serialized manifest config).
pub fn expand_json(
    value: &mut serde_json::Value,
    grants: &Grants,
    env_lookup: &dyn Fn(&str) -> Option<String>,
    secret_lookup: &dyn Fn(&str) -> Option<String>,
) {
    match value {
        serde_json::Value::String(s) => {
            *s = expand_str(s, grants, env_lookup, secret_lookup);
        }
        serde_json::Value::Array(items) => {
            for item in items {
                expand_json(item, grants, env_lookup, secret_lookup);
            }
        }
        serde_json::Value::Object(map) => {
            // Keys are not expanded (placeholder-in-key is not a supported
            // pattern); values are.
            let keys: Vec<String> = map.keys().cloned().collect();
            for key in keys {
                if let Some(v) = map.get_mut(&key) {
                    expand_json(v, grants, env_lookup, secret_lookup);
                }
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grants(env: &[&str], secrets: &[&str]) -> Grants {
        Grants::new(
            env.iter().map(|s| s.to_string()).collect(),
            secrets.iter().map(|s| s.to_string()).collect(),
        )
    }

    const NONE: fn(&str) -> Option<String> = |_| None;

    #[test]
    fn expands_granted_env() {
        let g = grants(&["MY_KEY"], &[]);
        let out = expand_str(
            "key=${ENV:MY_KEY}",
            &g,
            &|n| (n == "MY_KEY").then(|| "v1".to_string()),
            &NONE,
        );
        assert_eq!(out, "key=v1");
    }

    #[test]
    fn expands_granted_secret() {
        let g = grants(&[], &["api_token"]);
        let out = expand_str("Bearer ${SECRET:api_token}", &g, &NONE, &|n| {
            (n == "api_token").then(|| "tok".to_string())
        });
        assert_eq!(out, "Bearer tok");
    }

    #[test]
    fn ungranted_names_left_untouched() {
        let g = grants(&["A"], &["s1"]);
        let out = expand_str(
            "${ENV:OTHER}${SECRET:other}",
            &g,
            &|n| (n == "OTHER").then(|| "leak".to_string()),
            &|n| (n == "other").then(|| "leak".to_string()),
        );
        assert_eq!(out, "${ENV:OTHER}${SECRET:other}");
    }

    #[test]
    fn unresolvable_granted_name_left_untouched() {
        let g = grants(&["MISSING"], &["gone"]);
        let out = expand_str("${ENV:MISSING}/${SECRET:gone}", &g, &NONE, &NONE);
        assert_eq!(out, "${ENV:MISSING}/${SECRET:gone}");
    }

    #[test]
    fn no_grants_leaves_everything_untouched() {
        let g = Grants::default();
        let input = "x=${ENV:ANY} y=${SECRET:any}";
        assert_eq!(
            expand_str(input, &g, &|_| Some("leak".into()), &|_| Some(
                "leak".into()
            )),
            input
        );
    }

    #[test]
    fn multiple_and_adjacent_placeholders() {
        let g = grants(&["A", "B"], &["s"]);
        let out = expand_str(
            "${ENV:A}-${SECRET:s}-${ENV:B}${ENV:A}",
            &g,
            &|n| {
                (n == "A")
                    .then(|| "1".into())
                    .or_else(|| (n == "B").then(|| "2".into()))
            },
            &|_| Some("x".into()),
        );
        assert_eq!(out, "1-x-21");
    }

    #[test]
    fn non_placeholder_text_and_unicode_passthrough() {
        let g = grants(&["A"], &[]);
        let out = expand_str("héllo ${ENV:A} $$ {} ${", &g, &|_| Some("v".into()), &NONE);
        assert_eq!(out, "héllo v $$ {} ${");
    }

    #[test]
    fn unclosed_placeholder_untouched() {
        let g = grants(&["A"], &[]);
        let out = expand_str("${ENV:A", &g, &|_| Some("v".into()), &NONE);
        assert_eq!(out, "${ENV:A");
    }

    #[test]
    fn expands_json_recursively() {
        let g = grants(&["A"], &["s"]);
        let mut v: serde_json::Value = serde_json::json!({
            "url": "https://${ENV:A}.example.com",
            "nested": { "token": "${SECRET:s}", "num": 5, "list": ["${SECRET:s}", true] }
        });
        expand_json(&mut v, &g, &|_| Some("app".into()), &|_| Some("tok".into()));
        assert_eq!(v["url"], "https://app.example.com");
        assert_eq!(v["nested"]["token"], "tok");
        assert_eq!(v["nested"]["num"], 5);
        assert_eq!(v["nested"]["list"][0], "tok");
        assert_eq!(v["nested"]["list"][1], true);
    }

    #[test]
    fn json_ungranted_untouched() {
        let mut v: serde_json::Value =
            serde_json::json!({ "token": "${SECRET:nope}", "env": "${ENV:nope}" });
        expand_json(
            &mut v,
            &Grants::default(),
            &|_| Some("leak".into()),
            &|_| Some("leak".into()),
        );
        assert_eq!(v["token"], "${SECRET:nope}");
        assert_eq!(v["env"], "${ENV:nope}");
    }
}

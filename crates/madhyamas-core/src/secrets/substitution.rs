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

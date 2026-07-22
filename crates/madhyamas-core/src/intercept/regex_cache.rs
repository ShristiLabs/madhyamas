//! Cached regex compilation to avoid recompiling patterns on every request match.
//!
//! Patterns are cached in a global `HashMap<String, Regex>` guarded by a `Mutex`,
//! initialized once via `OnceLock`. Repeated lookups for the same pattern return
//! the cached `Regex` (or the cached error result as `None`).

use regex::Regex;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Global cache of compiled regexes keyed by pattern string.
static REGEX_CACHE: OnceLock<Mutex<HashMap<String, Regex>>> = OnceLock::new();

fn cache() -> &'static Mutex<HashMap<String, Regex>> {
    REGEX_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Compile (or fetch cached) regex for the given pattern.
/// Returns `None` if the pattern is invalid. Results are cached so subsequent
/// calls with the same pattern are free.
pub fn cached_regex(pattern: &str) -> Option<Regex> {
    {
        let map = cache().lock().ok()?;
        if let Some(re) = map.get(pattern) {
            return Some(re.clone());
        }
    }
    let re = Regex::new(pattern).ok()?;
    if let Ok(mut map) = cache().lock() {
        map.insert(pattern.to_string(), re.clone());
    }
    Some(re)
}

/// Check whether the pattern matches the haystack. Returns `false` if the
/// pattern is invalid or does not match.
pub fn is_match(pattern: &str, haystack: &str) -> bool {
    cached_regex(pattern)
        .map(|re| re.is_match(haystack))
        .unwrap_or(false)
}

/// Apply a regex replacement. Returns the original haystack if the pattern
/// is invalid; returns the replaced string otherwise.
pub fn replace_all(pattern: &str, haystack: &str, replacement: &str) -> String {
    cached_regex(pattern)
        .map(|re| re.replace_all(haystack, replacement).to_string())
        .unwrap_or_else(|| haystack.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caches_valid_pattern() {
        let re1 = cached_regex(r"^abc$");
        let re2 = cached_regex(r"^abc$");
        assert!(re1.is_some());
        assert!(re2.is_some());
    }

    #[test]
    fn invalid_pattern_returns_none() {
        assert!(cached_regex(r"[unclosed").is_none());
        // second call still None (cached failure not stored, but safe)
        assert!(cached_regex(r"[unclosed").is_none());
    }

    #[test]
    fn is_match_works() {
        assert!(is_match(r"^hello", "hello world"));
        assert!(!is_match(r"^world", "hello world"));
        assert!(!is_match(r"[bad", "anything"));
    }

    #[test]
    fn replace_all_works() {
        assert_eq!(replace_all(r"\d+", "a1b2c3", "X"), "aXbXcX");
        // invalid pattern returns original
        assert_eq!(replace_all(r"[bad", "original", "X"), "original");
    }
}

//! Integration tests for the public secrets API: encrypted file keystore,
//! placeholder substitution, redaction, and the secret service.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use madhyamas_core::secrets::keystore::{resolve_key, FileKeystore, KEY_ENV_VAR, KEY_FILE_ENV_VAR};
use madhyamas_core::secrets::redaction::{Redactor, REDACTED};
use madhyamas_core::secrets::service::{
    SecretAuditEvent, SecretAuditSink, SecretService, SecretStore,
};
use madhyamas_core::secrets::substitution::{expand_json, expand_str, Grants};
use madhyamas_test_utils::{test_key, tmpdir};

// ============================================================================
// FileKeystore — persistence
// ============================================================================

#[test]
fn keystore_round_trip_and_persistence() {
    let dir = tmpdir("roundtrip");
    let path = dir.path().join("secrets.enc.json");
    {
        let ks = FileKeystore::with_key(path.clone(), test_key());
        assert!(ks.names().unwrap().is_empty());
        ks.set("api_token", "tok-123").unwrap();
        ks.set("db_password", "pw").unwrap();
        ks.set("api_token", "tok-456").unwrap(); // overwrite
        assert_eq!(ks.get("api_token").unwrap().as_deref(), Some("tok-456"));
        assert_eq!(
            ks.names().unwrap(),
            vec!["api_token".to_string(), "db_password".to_string()]
        );
    }
    // Reopen: entries persist and decrypt.
    let ks = FileKeystore::with_key(path, test_key());
    assert_eq!(ks.get("api_token").unwrap().as_deref(), Some("tok-456"));
    assert_eq!(ks.get("db_password").unwrap().as_deref(), Some("pw"));
    assert!(ks.get("missing").unwrap().is_none());
    let all = ks.load_all().unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all["api_token"], "tok-456");
}

#[test]
fn keystore_delete() {
    let dir = tmpdir("delete");
    let ks = FileKeystore::with_key(dir.path().join("secrets.enc.json"), test_key());
    ks.set("a", "1").unwrap();
    assert!(ks.delete("a").unwrap());
    assert!(!ks.delete("a").unwrap());
    assert!(ks.get("a").unwrap().is_none());
}

#[test]
fn keystore_file_is_encrypted_at_rest() {
    let dir = tmpdir("atrest");
    let path = dir.path().join("secrets.enc.json");
    let ks = FileKeystore::with_key(path.clone(), test_key());
    let value = "plaintext-must-not-appear";
    ks.set("s", value).unwrap();
    let raw = std::fs::read_to_string(&path).unwrap();
    assert!(!raw.contains(value), "plaintext leaked to disk");
    assert!(raw.contains("\"ciphertext\""));
}

#[test]
fn resolve_key_generates_and_reuses_key_file() {
    let dir = tmpdir("resolve");
    // No env vars set in test environment for these names in practice;
    // remove to be safe.
    std::env::remove_var(KEY_ENV_VAR);
    std::env::remove_var(KEY_FILE_ENV_VAR);
    let k1 = resolve_key(dir.path()).unwrap();
    assert_eq!(k1.len(), 32);
    let k2 = resolve_key(dir.path()).unwrap();
    assert_eq!(k1, k2, "key file must be reused across calls");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(dir.path().join("secrets.key"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}

// ============================================================================
// Substitution
// ============================================================================

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

// ============================================================================
// Redaction
// ============================================================================

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

// ============================================================================
// Secret service
// ============================================================================

#[derive(Default)]
struct AuditCapture {
    events: Mutex<Vec<SecretAuditEvent>>,
}

impl SecretAuditSink for AuditCapture {
    fn record(&self, e: SecretAuditEvent) {
        self.events.lock().unwrap().push(e);
    }
}

#[test]
fn set_get_delete_lifecycle() {
    let store = Arc::new(madhyamas_test_utils::MemStore::new());
    let svc = SecretService::new(store.clone()).unwrap();
    assert!(svc.is_empty());
    svc.set("api_token", "tok").unwrap();
    assert_eq!(svc.names(), vec!["api_token".to_string()]);
    assert_eq!(svc.get("api_token", "plugin-a").as_deref(), Some("tok"));
    assert!(svc.get("missing", "plugin-a").is_none());
    assert!(svc.delete("api_token"));
    assert!(!svc.delete("api_token"));
    assert!(svc.names().is_empty());
    // Persisted to the backing store too.
    assert!(store.load_all().unwrap().is_empty());
}

#[test]
fn invalid_names_rejected() {
    let svc = SecretService::new(Arc::new(madhyamas_test_utils::MemStore::new())).unwrap();
    assert!(svc.set("", "v").is_err());
    assert!(svc.set("bad name", "v").is_err());
    assert!(svc.set("ok_name-1", "v").is_ok());
}

#[test]
fn audit_events_emitted() {
    let audit = Arc::new(AuditCapture::default());
    let svc = SecretService::new(Arc::new(madhyamas_test_utils::MemStore::new()))
        .unwrap()
        .with_audit_sink(audit.clone());
    svc.set("s", "value-1234").unwrap();
    svc.get("s", "plugin-b").unwrap();
    svc.delete("s");
    let events = audit.events.lock().unwrap();
    let actions: Vec<&str> = events.iter().map(|e| e.action.as_str()).collect();
    assert_eq!(
        actions,
        vec!["secret_set", "secret_granted", "secret_delete"]
    );
    assert!(events
        .iter()
        .all(|e| !format!("{:?}", e).contains("value-1234")));
    assert_eq!(events[1].actor, "plugin-b");
}

#[test]
fn failed_read_of_missing_secret_no_audit() {
    let audit = Arc::new(AuditCapture::default());
    let svc = SecretService::new(Arc::new(madhyamas_test_utils::MemStore::new()))
        .unwrap()
        .with_audit_sink(audit.clone());
    svc.get("nope", "plugin");
    assert!(audit.events.lock().unwrap().is_empty());
}

#[test]
fn redactor_built_from_values() {
    let svc = SecretService::new(Arc::new(madhyamas_test_utils::MemStore::new())).unwrap();
    svc.set("t", "abcdef").unwrap();
    let r = svc.redactor(&[]);
    assert_eq!(r.redact_text("x abcdef y"), "x [REDACTED] y");
}

#[test]
fn loads_existing_from_store() {
    let store = Arc::new(madhyamas_test_utils::MemStore::new());
    store.set("pre", "existing").unwrap();
    let svc = SecretService::new(store).unwrap();
    assert_eq!(svc.get("pre", "t").as_deref(), Some("existing"));
}

//! Integration tests for the public mirror API: writing response bodies to
//! disk with path mapping, host filtering, metadata sidecars, stats, and
//! request-body saving.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use chrono::Utc;
use madhyamas_core::config::MirrorConfig;
use madhyamas_core::mirror::MirrorWriter;
use madhyamas_core::traffic::{HttpMethod, RequestData, ResponseData};
use madhyamas_test_utils::tmpdir;

fn make_response(body: &[u8], content_type: Option<&str>) -> ResponseData {
    let mut headers = HashMap::new();
    if let Some(ct) = content_type {
        headers.insert("content-type".to_string(), ct.to_string());
    }
    ResponseData {
        status_code: 200,
        status_message: Some("OK".to_string()),
        headers,
        body: Some(body.to_vec()),
        content_type: content_type.map(|s| s.to_string()),
        duration_ms: 42,
        http_version: None,
    }
}

fn make_writer(dir: &Path, enabled: bool, host_filter: Option<Vec<String>>) -> Arc<MirrorWriter> {
    let config = MirrorConfig {
        enabled,
        output_dir: dir.to_string_lossy().to_string(),
        host_filter,
        save_request_bodies: false,
    };
    MirrorWriter::new(config)
}

#[test]
fn test_basic_mirror() {
    let dir = tmpdir("mirror-basic");
    let writer = make_writer(dir.path(), true, None);
    let response = make_response(b"hello world", Some("text/html"));
    let ts = Utc::now();

    let path = writer
        .write_response(
            "example.com",
            "/index.html",
            "GET",
            "https://example.com/index.html",
            &response,
            ts,
            false,
        )
        .unwrap();

    assert!(path.starts_with(dir.path()));
    assert!(path.exists());
    let content = std::fs::read(&path).unwrap();
    assert_eq!(content, b"hello world");
}

#[test]
fn test_path_mapping_trailing_slash() {
    let dir = tmpdir("mirror-slash");
    let writer = make_writer(dir.path(), true, None);
    let response = make_response(b"{}", Some("application/json"));
    let ts = Utc::now();

    let path = writer
        .write_response(
            "api.example.com",
            "/v1/users/",
            "GET",
            "https://api.example.com/v1/users/",
            &response,
            ts,
            false,
        )
        .unwrap();

    assert!(path.ends_with("api.example.com/v1/users/index.json"));
    assert!(path.exists());
}

#[test]
fn test_path_mapping_no_extension() {
    let dir = tmpdir("mirror-noext");
    let writer = make_writer(dir.path(), true, None);
    let response = make_response(b"<html></html>", Some("text/html"));
    let ts = Utc::now();

    let path = writer
        .write_response(
            "example.com",
            "/v1/users/123",
            "GET",
            "https://example.com/v1/users/123",
            &response,
            ts,
            false,
        )
        .unwrap();

    assert!(path.ends_with("example.com/v1/users/123/index.html"));
    assert!(path.exists());
}

#[test]
fn test_path_mapping_with_extension() {
    let dir = tmpdir("mirror-ext");
    let writer = make_writer(dir.path(), true, None);
    let response = make_response(b"\x89PNG", Some("image/png"));
    let ts = Utc::now();

    let path = writer
        .write_response(
            "cdn.example.com",
            "/assets/img/logo.png",
            "GET",
            "https://cdn.example.com/assets/img/logo.png",
            &response,
            ts,
            false,
        )
        .unwrap();

    assert!(path.ends_with("cdn.example.com/assets/img/logo.png"));
    assert!(path.exists());
}

#[test]
fn test_query_string_stripped() {
    let dir = tmpdir("mirror-query");
    let writer = make_writer(dir.path(), true, None);
    let response = make_response(b"data", Some("text/html"));
    let ts = Utc::now();

    let path = writer
        .write_response(
            "example.com",
            "/page?format=json&foo=bar",
            "GET",
            "https://example.com/page?format=json&foo=bar",
            &response,
            ts,
            false,
        )
        .unwrap();

    // The query string should not appear in the path. "page" has no
    // extension, so it becomes a directory with an index file.
    assert!(path.ends_with("example.com/page/index.html"));
    assert!(path.exists());
}

#[test]
fn test_host_filter_matching() {
    let dir = tmpdir("mirror-filter");
    let writer = make_writer(dir.path(), true, Some(vec!["*.example.com".to_string()]));
    let response = make_response(b"ok", Some("text/html"));
    let ts = Utc::now();

    // Matching host — should write.
    let path = writer
        .write_response(
            "api.example.com",
            "/data",
            "GET",
            "https://api.example.com/data",
            &response,
            ts,
            false,
        )
        .unwrap();
    assert!(path.exists());

    // Non-matching host — should not write.
    let path2 = writer
        .write_response(
            "other.com",
            "/data",
            "GET",
            "https://other.com/data",
            &response,
            ts,
            false,
        )
        .unwrap();
    assert!(!path2.exists());
}

#[test]
fn test_overwrite() {
    let dir = tmpdir("mirror-overwrite");
    let writer = make_writer(dir.path(), true, None);
    let ts = Utc::now();

    let resp1 = make_response(b"first", Some("text/html"));
    writer
        .write_response(
            "example.com",
            "/page.html",
            "GET",
            "https://example.com/page.html",
            &resp1,
            ts,
            false,
        )
        .unwrap();

    let resp2 = make_response(b"second", Some("text/html"));
    let path = writer
        .write_response(
            "example.com",
            "/page.html",
            "GET",
            "https://example.com/page.html",
            &resp2,
            ts,
            false,
        )
        .unwrap();

    let content = std::fs::read(&path).unwrap();
    assert_eq!(content, b"second");
}

#[test]
fn test_path_safety_traversal() {
    let dir = tmpdir("mirror-traversal");
    let writer = make_writer(dir.path(), true, None);
    let response = make_response(b"bad", Some("text/html"));
    let ts = Utc::now();

    // The `..` component should be stripped, not escape the output dir.
    let result = writer.write_response(
        "example.com",
        "/../../etc/passwd",
        "GET",
        "https://example.com/../../etc/passwd",
        &response,
        ts,
        false,
    );

    // Should either return an error (escaping) or a safe path.
    match result {
        Ok(path) => {
            // Verify the path is within the output dir (no `..` escape).
            // The `..` components are stripped by sanitization, so the
            // path should be within the base directory.
            let base = dir.path().to_path_buf();
            assert!(
                path.starts_with(&base),
                "Path {} escaped base {}",
                path.display(),
                base.display()
            );
            // Verify no `..` in the relative part.
            let relative = path.strip_prefix(&base).unwrap();
            for comp in relative.components() {
                assert!(
                    !matches!(comp, std::path::Component::ParentDir),
                    "Path {} contains parent dir component",
                    path.display()
                );
            }
        }
        Err(_) => {
            // Escaping is correctly rejected.
        }
    }
}

#[test]
fn test_metadata_sidecar() {
    let dir = tmpdir("mirror-meta");
    let writer = make_writer(dir.path(), true, None);
    let mut headers = HashMap::new();
    headers.insert("content-type".to_string(), "application/json".to_string());
    let response = ResponseData {
        status_code: 200,
        status_message: Some("OK".to_string()),
        headers,
        body: Some(b"{}".to_vec()),
        content_type: Some("application/json".to_string()),
        duration_ms: 145,
        http_version: None,
    };
    let ts = Utc::now();

    let path = writer
        .write_response(
            "api.example.com",
            "/v1/users/123",
            "GET",
            "https://api.example.com/v1/users/123",
            &response,
            ts,
            false,
        )
        .unwrap();

    // Find the .meta.json sidecar.
    let meta_path = path.with_extension("json.meta.json");
    assert!(
        meta_path.exists(),
        "Metadata sidecar not found at {}",
        meta_path.display()
    );

    let content = std::fs::read_to_string(&meta_path).unwrap();
    let meta: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(meta["url"], "https://api.example.com/v1/users/123");
    assert_eq!(meta["method"], "GET");
    assert_eq!(meta["status_code"], 200);
    assert_eq!(meta["duration_ms"], 145);
    assert_eq!(meta["truncated"], false);
}

#[test]
fn test_disabled_no_files() {
    let dir = tmpdir("mirror-disabled");
    let writer = make_writer(dir.path(), false, None);
    let response = make_response(b"data", Some("text/html"));
    let ts = Utc::now();

    let path = writer
        .write_response(
            "example.com",
            "/page.html",
            "GET",
            "https://example.com/page.html",
            &response,
            ts,
            false,
        )
        .unwrap();

    assert!(path.as_os_str().is_empty());
    assert_eq!(writer.stats().files_written, 0);
}

#[test]
fn test_stats_counters() {
    let dir = tmpdir("mirror-stats");
    let writer = make_writer(dir.path(), true, None);
    let ts = Utc::now();

    let resp1 = make_response(b"hello", Some("text/html"));
    writer
        .write_response(
            "example.com",
            "/a.html",
            "GET",
            "https://example.com/a.html",
            &resp1,
            ts,
            false,
        )
        .unwrap();

    let resp2 = make_response(b"world!", Some("text/html"));
    writer
        .write_response(
            "example.com",
            "/b.html",
            "GET",
            "https://example.com/b.html",
            &resp2,
            ts,
            false,
        )
        .unwrap();

    let stats = writer.stats();
    assert_eq!(stats.files_written, 2);
    assert_eq!(
        stats.bytes_written,
        (b"hello".len() + b"world!".len()) as u64
    );
}

#[test]
fn test_request_body_saved() {
    let dir = tmpdir("mirror-request");
    let config = MirrorConfig {
        enabled: true,
        output_dir: dir.path().to_string_lossy().to_string(),
        host_filter: None,
        save_request_bodies: true,
    };
    let writer = MirrorWriter::new(config);
    let ts = Utc::now();

    let response = make_response(b"response", Some("text/html"));
    let path = writer
        .write_response(
            "example.com",
            "/page.html",
            "POST",
            "https://example.com/page.html",
            &response,
            ts,
            false,
        )
        .unwrap();

    let request = RequestData {
        method: HttpMethod::Post,
        url: "https://example.com/page.html".to_string(),
        host: "example.com".to_string(),
        path: "/page.html".to_string(),
        headers: HashMap::new(),
        body: Some(b"request body".to_vec()),
        content_type: Some("text/plain".to_string()),
        http_version: None,
    };
    writer.write_request_body(&path, &request).unwrap();

    let req_path = path.with_extension("html.request");
    assert!(
        req_path.exists(),
        "Request body file not found at {}",
        req_path.display()
    );
    let content = std::fs::read(&req_path).unwrap();
    assert_eq!(content, b"request body");
}

//! Mirror tool — save response bodies to disk as a site mirror.
//!
//! The [`MirrorWriter`] writes each captured response body to disk following
//! the URL path structure (`output_dir/host/path/content`), along with a
//! `.meta.json` sidecar containing request/response metadata. This is the
//! equivalent of Charles Proxy's "Mirror" / "Save Responses" feature and is
//! useful for offline browsing, debugging, and archiving.
//!
//! # Path mapping
//!
//! The host becomes the top-level directory and the URL path maps directly to
//! a filesystem path. Paths ending with `/` or having no file extension are
//! saved as `index.html` (or `index.json` based on content-type). Query
//! strings are stored in the metadata sidecar to keep filenames clean.
//!
//! | URL | Filesystem path |
//! |-----|-----------------|
//! | `https://api.example.com/v1/users/123` | `output_dir/api.example.com/v1/users/123/index.json` |
//! | `https://cdn.example.com/assets/img/logo.png` | `output_dir/cdn.example.com/assets/img/logo.png` |
//!
//! See [`docs/MIRROR.md`](../../docs/MIRROR.md) for the end-user guide.

use crate::config::MirrorConfig;
use crate::traffic::{host_matches_pattern, RequestData, ResponseData};
use crate::Error;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::Serialize;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::{debug, error};

/// Statistics reported by [`MirrorWriter::stats`].
#[derive(Debug, Clone, Serialize)]
pub struct MirrorStats {
    /// Whether mirroring is currently enabled.
    pub enabled: bool,
    /// Output directory for mirrored files.
    pub output_dir: String,
    /// Number of files written to disk.
    pub files_written: u64,
    /// Total bytes written to disk.
    pub bytes_written: u64,
}

/// Metadata sidecar written alongside each mirrored response body.
#[derive(Debug, Clone, Serialize)]
struct MirrorMetadata {
    url: String,
    method: String,
    status_code: u16,
    headers: std::collections::HashMap<String, String>,
    timestamp: String,
    duration_ms: u64,
    /// Whether the response body was truncated before mirroring.
    truncated: bool,
}

/// Writes response bodies to disk following the URL path structure.
///
/// The writer holds a live-updatable [`MirrorConfig`] (shared with the API
/// layer so runtime changes take effect) and atomic counters tracking the
/// number of files and bytes written.
///
/// Writes are performed synchronously within [`MirrorWriter::write_response`];
/// callers should spawn the call on a background task (e.g. `tokio::spawn`)
/// to avoid blocking the proxy pipeline.
pub struct MirrorWriter {
    config: Arc<RwLock<MirrorConfig>>,
    files_written: AtomicU64,
    bytes_written: AtomicU64,
}

impl MirrorWriter {
    /// Create a new `MirrorWriter` with the given configuration.
    pub fn new(config: MirrorConfig) -> Arc<Self> {
        Arc::new(Self {
            config: Arc::new(RwLock::new(config)),
            files_written: AtomicU64::new(0),
            bytes_written: AtomicU64::new(0),
        })
    }

    /// Create a new `MirrorWriter` sharing an existing config `Arc`.
    ///
    /// This is used when the API layer holds the same `Arc<RwLock<MirrorConfig>>`
    /// so that runtime config changes are visible to the writer without a
    /// restart.
    pub fn with_shared_config(config: Arc<RwLock<MirrorConfig>>) -> Arc<Self> {
        Arc::new(Self {
            config,
            files_written: AtomicU64::new(0),
            bytes_written: AtomicU64::new(0),
        })
    }

    /// Get a reference to the shared config.
    pub fn config(&self) -> &Arc<RwLock<MirrorConfig>> {
        &self.config
    }

    /// Get the current mirror statistics.
    pub fn stats(&self) -> MirrorStats {
        let cfg = self.config.read();
        MirrorStats {
            enabled: cfg.enabled,
            output_dir: cfg.output_dir.clone(),
            files_written: self.files_written.load(Ordering::Relaxed),
            bytes_written: self.bytes_written.load(Ordering::Relaxed),
        }
    }

    /// Whether mirroring is currently enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.read().enabled
    }

    /// Write a response body to disk following the URL path structure.
    ///
    /// Returns the path to the written body file. When mirroring is disabled
    /// or the host does not match the filter, returns an empty `PathBuf`
    /// without writing anything.
    ///
    /// # Errors
    ///
    /// Returns an error if the output directory cannot be created or the
    /// body/metadata files cannot be written.
    #[allow(clippy::too_many_arguments)]
    pub fn write_response(
        &self,
        host: &str,
        path: &str,
        method: &str,
        url: &str,
        response: &ResponseData,
        timestamp: DateTime<Utc>,
        body_truncated: bool,
    ) -> crate::Result<PathBuf> {
        let config = self.config.read().clone();
        if !config.enabled {
            return Ok(PathBuf::new());
        }

        // Check host filter.
        if let Some(filter) = &config.host_filter {
            if !filter.is_empty() && !filter.iter().any(|p| host_matches_pattern(host, p)) {
                return Ok(PathBuf::new());
            }
        }

        let file_path = self.url_to_file_path(&config.output_dir, host, path, response)?;

        // Create parent directories as needed.
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                Error::Proxy(format!(
                    "Failed to create mirror directory {}: {}",
                    parent.display(),
                    e
                ))
            })?;
        }

        // Write the response body (if present).
        if let Some(body) = &response.body {
            std::fs::write(&file_path, body).map_err(|e| {
                Error::Proxy(format!(
                    "Failed to write mirror file {}: {}",
                    file_path.display(),
                    e
                ))
            })?;
            let len = body.len() as u64;
            self.files_written.fetch_add(1, Ordering::Relaxed);
            self.bytes_written.fetch_add(len, Ordering::Relaxed);
            debug!(
                "Mirrored response {} {} ({} bytes) to {}",
                method,
                url,
                len,
                file_path.display()
            );
        } else {
            // No body — write an empty file so the mirror entry exists.
            std::fs::write(&file_path, []).map_err(|e| {
                Error::Proxy(format!(
                    "Failed to write mirror file {}: {}",
                    file_path.display(),
                    e
                ))
            })?;
            self.files_written.fetch_add(1, Ordering::Relaxed);
            debug!(
                "Mirrored response {} {} (empty body) to {}",
                method,
                url,
                file_path.display()
            );
        }

        // Write the metadata sidecar.
        if let Err(e) = self.write_metadata_sidecar(
            &file_path,
            url,
            method,
            response,
            timestamp,
            body_truncated,
        ) {
            error!("Failed to write mirror metadata sidecar: {}", e);
        }

        Ok(file_path)
    }

    /// Write a request body to disk alongside the mirrored response.
    ///
    /// The request body is written as `<response_file>.request`. This is only
    /// done when `save_request_bodies` is enabled in the config.
    pub fn write_request_body(
        &self,
        response_file: &Path,
        request: &RequestData,
    ) -> crate::Result<()> {
        let config = self.config.read();
        if !config.enabled || !config.save_request_bodies {
            return Ok(());
        }
        let body = match &request.body {
            Some(b) if !b.is_empty() => b,
            _ => return Ok(()),
        };
        let req_path = response_file.with_extension(format!(
            "{}.request",
            response_file
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("bin")
        ));
        std::fs::write(&req_path, body)
            .map_err(|e| Error::Proxy(format!("Failed to write mirror request body: {}", e)))?;
        self.bytes_written
            .fetch_add(body.len() as u64, Ordering::Relaxed);
        Ok(())
    }

    /// Map a URL (host + path) to a filesystem path within the output dir.
    ///
    /// Rules:
    /// - Host becomes the top-level directory.
    /// - URL path maps directly to a filesystem path.
    /// - If the path ends with `/` or has no file extension, save as
    ///   `index.html` (or `index.json` based on content-type).
    /// - Query strings are stripped (stored in the metadata sidecar).
    /// - Path components are sanitized (no `..`, no absolute paths, no null
    ///   bytes) to prevent directory traversal.
    fn url_to_file_path(
        &self,
        output_dir: &str,
        host: &str,
        path: &str,
        response: &ResponseData,
    ) -> crate::Result<PathBuf> {
        let base = PathBuf::from(output_dir);
        let mut full = base;

        // Sanitize and append the host.
        let host = sanitize_component(host);
        if !host.is_empty() {
            full.push(&host);
        }

        // Strip the query string from the path.
        let path_only = path.split('?').next().unwrap_or(path);

        // Split the path into sanitized components.
        let components: Vec<String> = path_only
            .split('/')
            .filter(|c| !c.is_empty())
            .map(sanitize_component)
            .filter(|c| !c.is_empty())
            .collect();

        let has_trailing_slash = path_only.ends_with('/');

        if components.is_empty() {
            // Root path — use index file.
            full.push(index_filename(response));
        } else if has_trailing_slash {
            // Path ends with '/' — all components are directories, then index.
            for comp in &components {
                full.push(comp);
            }
            full.push(index_filename(response));
        } else {
            // Last component is the filename (if it has an extension),
            // otherwise treat all as directories + index.
            let last_idx = components.len() - 1;
            for comp in &components {
                full.push(comp);
            }
            // If the last component had no extension, append an index file.
            if !has_extension(&components[last_idx]) {
                full.push(index_filename(response));
            }
        }

        // Final safety check: ensure the resolved path does not contain any
        // `..` or root components in the part relative to the output dir
        // (which would escape the output directory). The per-component
        // sanitization already strips `..`, but this is a defense-in-depth
        // check against the assembled path.
        let base = Path::new(output_dir);
        let relative = full.strip_prefix(base).map_err(|_| {
            Error::Proxy(format!(
                "Mirror path escapes output directory: {}",
                full.display()
            ))
        })?;
        for comp in relative.components() {
            match comp {
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    return Err(Error::Proxy(format!(
                        "Mirror path escapes output directory: {}",
                        full.display()
                    )));
                }
                _ => {}
            }
        }

        Ok(full)
    }

    /// Write the `.meta.json` sidecar alongside a mirrored body file.
    fn write_metadata_sidecar(
        &self,
        body_file: &Path,
        url: &str,
        method: &str,
        response: &ResponseData,
        timestamp: DateTime<Utc>,
        body_truncated: bool,
    ) -> crate::Result<()> {
        let meta_path = body_file.with_extension(format!(
            "{}.meta.json",
            body_file
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("bin")
        ));

        let metadata = MirrorMetadata {
            url: url.to_string(),
            method: method.to_string(),
            status_code: response.status_code,
            headers: response.headers.clone(),
            timestamp: timestamp.to_rfc3339(),
            duration_ms: response.duration_ms,
            truncated: body_truncated,
        };

        let json = serde_json::to_string_pretty(&metadata)?;
        std::fs::write(&meta_path, json)
            .map_err(|e| Error::Proxy(format!("Failed to write mirror metadata: {}", e)))?;
        Ok(())
    }
}

/// Determine the index filename based on the response content-type.
///
/// JSON content-types use `index.json`; everything else uses `index.html`.
fn index_filename(response: &ResponseData) -> &'static str {
    if let Some(ct) = &response.content_type {
        let ct = ct.to_lowercase();
        if ct.contains("json") {
            return "index.json";
        }
    }
    // Also check the headers for content-type.
    if let Some(ct) = response
        .headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .map(|(_, v)| v.as_str())
    {
        if ct.to_lowercase().contains("json") {
            return "index.json";
        }
    }
    "index.html"
}

/// Whether a path component looks like a filename (has a dot-extension).
fn has_extension(component: &str) -> bool {
    // A component like "logo.png" or "v1.2" has an extension. We treat a
    // trailing dot or a dot in the last segment as an extension indicator.
    // Avoid treating version-like segments ("v1", "123") as filenames.
    if let Some(idx) = component.rfind('.') {
        // Require at least one char after the dot and at least one before.
        idx > 0 && idx < component.len() - 1
    } else {
        false
    }
}

/// Sanitize a single path component for safe filesystem use.
///
/// - Rejects `..` (directory traversal).
/// - Strips null bytes and path separators.
/// - Replaces characters that are illegal on Windows with underscores.
fn sanitize_component(component: &str) -> String {
    let trimmed = component.trim();
    if trimmed.is_empty() || trimmed == "." || trimmed == ".." {
        return String::new();
    }
    let mut out = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        match ch {
            '\0' | '/' | '\\' => {}
            '<' | '>' | ':' | '"' | '|' | '?' | '*' => out.push('_'),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

/// Verify that a resolved path does not escape the base directory via
/// `Component::ParentDir` or `Component::RootDir`.
#[allow(dead_code)]
fn is_path_safe(base: &Path, candidate: &Path) -> bool {
    let mut resolved = base.to_path_buf();
    for comp in candidate.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                if !resolved.pop() {
                    return false;
                }
            }
            Component::Normal(part) => resolved.push(part),
            Component::RootDir | Component::Prefix(_) => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traffic::HttpMethod;
    use std::collections::HashMap;

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

    fn make_writer(
        dir: &Path,
        enabled: bool,
        host_filter: Option<Vec<String>>,
    ) -> Arc<MirrorWriter> {
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
        let dir = tempfile::tempdir().unwrap();
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
        let dir = tempfile::tempdir().unwrap();
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
        let dir = tempfile::tempdir().unwrap();
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
        let dir = tempfile::tempdir().unwrap();
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
        let dir = tempfile::tempdir().unwrap();
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
        let dir = tempfile::tempdir().unwrap();
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
        let dir = tempfile::tempdir().unwrap();
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
        let dir = tempfile::tempdir().unwrap();
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
                        !matches!(comp, Component::ParentDir),
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
        let dir = tempfile::tempdir().unwrap();
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
        let dir = tempfile::tempdir().unwrap();
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
        let dir = tempfile::tempdir().unwrap();
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
        let dir = tempfile::tempdir().unwrap();
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

    #[test]
    fn test_sanitize_component() {
        assert_eq!(sanitize_component(".."), "");
        assert_eq!(sanitize_component("."), "");
        assert_eq!(sanitize_component(""), "");
        assert_eq!(sanitize_component("normal"), "normal");
        assert_eq!(sanitize_component("file name"), "file name");
        assert_eq!(sanitize_component("a/b\\b"), "abb");
        assert_eq!(sanitize_component("a:b*c?d"), "a_b_c_d");
    }

    #[test]
    fn test_has_extension() {
        assert!(has_extension("logo.png"));
        assert!(has_extension("index.html"));
        assert!(!has_extension("index"));
        assert!(!has_extension("123"));
        assert!(!has_extension(""));
    }

    #[test]
    fn test_index_filename_json() {
        let response = make_response(b"{}", Some("application/json"));
        assert_eq!(index_filename(&response), "index.json");
    }

    #[test]
    fn test_index_filename_html() {
        let response = make_response(b"<html>", Some("text/html"));
        assert_eq!(index_filename(&response), "index.html");
    }
}

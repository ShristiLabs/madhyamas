//! Embedded web UI assets
//!
//! Web assets are embedded at compile time using `rust-embed` so the
//! binary is fully self-contained — no external `web/dist/` directory needed.
//! A `MADHYAMAS_WEB_DIR` env var can still override to serve from disk
//! (useful for development).
//!
//! When the `embedded-assets` feature is disabled (e.g. when publishing to
//! crates.io), only disk-based serving is available.
//!
//! When a base path is configured (Phase 6d — load-balancer context-path
//! routing), the served `index.html` is patched at runtime to inject a
//! `<meta name="madhyamas-base-path">` tag so the frontend API client and
//! WebSocket URL can resolve the correct prefix.

use axum::body::Body;
use axum::http::{header, Response, StatusCode};
use axum::response::IntoResponse;
use std::sync::OnceLock;

#[cfg(feature = "embedded-assets")]
use axum::http::HeaderValue;

#[cfg(feature = "embedded-assets")]
use rust_embed::RustEmbed;

/// Global base path for runtime injection into index.html. Set once by
/// [`set_base_path`] during router construction.
static BASE_PATH: OnceLock<String> = OnceLock::new();

/// Set the global base path used for runtime index.html meta-tag injection.
/// Called once by [`create_router`]. When unset, no injection occurs (root
/// deployment — default behaviour).
pub fn set_base_path(path: &str) {
    let normalized = if path.is_empty() || path == "/" {
        "/".to_string()
    } else {
        let p = path.trim_start_matches('/');
        let p = p.trim_end_matches('/');
        format!("/{p}/")
    };
    let _ = BASE_PATH.set(normalized);
}

/// Returns the configured base path (always starts and ends with `/`).
/// Defaults to `/` when unset.
fn get_base_path() -> &'static str {
    BASE_PATH.get().map(|s| s.as_str()).unwrap_or("/")
}

/// Inject the `<base>` and `<meta name="madhyamas-base-path">` tags into the
/// HTML `<head>` when a non-root base path is configured. Returns the
/// original bytes unchanged when the base path is root.
#[cfg(feature = "embedded-assets")]
fn inject_base_path(html: &[u8]) -> Vec<u8> {
    let base = get_base_path();
    if base == "/" {
        return html.to_vec();
    }
    let html_str = String::from_utf8_lossy(html);
    let injection =
        format!(r#"<base href="{base}"><meta name="madhyamas-base-path" content="{base}">"#,);
    if let Some(idx) = html_str.find("<head>") {
        let mut result = html_str[..idx + 6].to_string();
        result.push_str(&injection);
        result.push_str(&html_str[idx + 6..]);
        result.into_bytes()
    } else if let Some(idx) = html_str.find("<head ") {
        // <head class="..."> — inject after the opening tag closes
        if let Some(end) = html_str[idx..].find('>') {
            let pos = idx + end + 1;
            let mut result = html_str[..pos].to_string();
            result.push_str(&injection);
            result.push_str(&html_str[pos..]);
            result.into_bytes()
        } else {
            html.to_vec()
        }
    } else {
        html.to_vec()
    }
}

#[cfg(feature = "embedded-assets")]
#[derive(RustEmbed)]
#[folder = "../../web/dist/"]
struct WebAssets;

/// Serve an embedded file by path (e.g. "index.html", "assets/index-abc.js").
///
/// Returns `Some(Response)` if the file exists, `None` otherwise.
/// Falls back to `index.html` for unknown paths (SPA routing). When a
/// non-root base path is configured, `index.html` is patched with a
/// `<base>` + `<meta>` tag at runtime.
#[cfg(feature = "embedded-assets")]
pub fn serve_embedded(path: &str) -> Option<Response<Body>> {
    // Normalize: strip leading slash, default to index.html
    let path = path.trim_start_matches('/');

    // Try the exact path first, then SPA fallback to index.html
    let (file, served_path) = match WebAssets::get(path) {
        Some(f) => (f, path),
        None => {
            // SPA fallback: serve index.html for non-asset routes
            if !path.starts_with("assets/") && !path.starts_with("favicon") && !path.contains('.') {
                let f = WebAssets::get("index.html")?;
                (f, "index.html")
            } else {
                return None;
            }
        }
    };

    // Guess MIME from the ACTUAL file being served, not the request path.
    // This ensures index.html (served via SPA fallback) gets text/html.
    let mime = mime_guess::from_path(served_path).first_or_octet_stream();

    // Inject base-path meta tag into index.html at runtime.
    let body_bytes = if served_path == "index.html" {
        inject_base_path(&file.data)
    } else {
        file.data.into_owned()
    };

    let mut response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime.as_ref())
        .body(Body::from(body_bytes))
        .ok()?;

    // Set cache headers for hashed assets
    if served_path.starts_with("assets/") {
        let val = HeaderValue::from_static("public, max-age=31536000, immutable");
        response.headers_mut().insert(header::CACHE_CONTROL, val);
    }

    Some(response)
}

/// Serve an embedded file by path — stub when embedded-assets feature is off.
#[cfg(not(feature = "embedded-assets"))]
pub fn serve_embedded(_path: &str) -> Option<Response<Body>> {
    None
}

/// Check whether embedded assets are available.
#[cfg(feature = "embedded-assets")]
pub fn has_embedded() -> bool {
    WebAssets::iter().next().is_some()
}

/// Check whether embedded assets are available — stub when feature is off.
#[cfg(not(feature = "embedded-assets"))]
pub fn has_embedded() -> bool {
    false
}

/// Axum fallback handler that tries embedded assets first, then disk.
pub async fn embedded_fallback(uri: axum::http::Uri) -> impl IntoResponse {
    let path = uri.path();

    // 1. Try embedded assets
    if let Some(resp) = serve_embedded(path) {
        return resp;
    }

    // 2. Try disk-based serving (MADHYAMAS_WEB_DIR or web/dist)
    let web_dir = std::env::var("MADHYAMAS_WEB_DIR").unwrap_or_else(|_| "web/dist".to_string());
    let disk_path = format!("{}/{}", web_dir, path.trim_start_matches('/'));

    if let Ok(metadata) = std::fs::metadata(&disk_path) {
        if metadata.is_file() {
            if let Ok(data) = std::fs::read(&disk_path) {
                let mime = mime_guess::from_path(&disk_path).first_or_octet_stream();
                return Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, mime.as_ref())
                    .body(Body::from(data))
                    .unwrap_or_else(|_| StatusCode::NOT_FOUND.into_response());
            }
        }
    }

    // 3. SPA fallback — try index.html from embedded or disk
    if let Some(resp) = serve_embedded("index.html") {
        return resp;
    }

    let index_path = format!("{}/index.html", web_dir);
    if let Ok(data) = std::fs::read(&index_path) {
        let body_bytes = inject_base_path_disk(&data);
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(Body::from(body_bytes))
            .unwrap_or_else(|_| StatusCode::NOT_FOUND.into_response());
    }

    StatusCode::NOT_FOUND.into_response()
}

/// Inject the base-path meta tag into disk-served index.html. Same logic as
/// [`inject_base_path`] but available even when the `embedded-assets` feature
/// is off.
fn inject_base_path_disk(html: &[u8]) -> Vec<u8> {
    let base = get_base_path();
    if base == "/" {
        return html.to_vec();
    }
    let html_str = String::from_utf8_lossy(html);
    let injection =
        format!(r#"<base href="{base}"><meta name="madhyamas-base-path" content="{base}">"#,);
    if let Some(idx) = html_str.find("<head>") {
        let mut result = html_str[..idx + 6].to_string();
        result.push_str(&injection);
        result.push_str(&html_str[idx + 6..]);
        result.into_bytes()
    } else if let Some(idx) = html_str.find("<head ") {
        if let Some(end) = html_str[idx..].find('>') {
            let pos = idx + end + 1;
            let mut result = html_str[..pos].to_string();
            result.push_str(&injection);
            result.push_str(&html_str[pos..]);
            result.into_bytes()
        } else {
            html.to_vec()
        }
    } else {
        html.to_vec()
    }
}

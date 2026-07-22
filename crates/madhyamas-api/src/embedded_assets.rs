//! Embedded web UI assets
//!
//! Web assets are embedded at compile time using `rust-embed` so the
//! binary is fully self-contained — no external `web/dist/` directory needed.
//! A `MADHYAMAS_WEB_DIR` env var can still override to serve from disk
//! (useful for development).

use axum::body::Body;
use axum::http::{header, HeaderValue, Response, StatusCode};
use axum::response::IntoResponse;
use rust_embed::RustEmbed;

/// Embedded static files from `web/dist/` at compile time.
/// Path is relative to the workspace root (via CARGO_MANIFEST_DIR).
#[derive(RustEmbed)]
#[folder = "../../web/dist/"]
struct WebAssets;

/// Serve an embedded file by path (e.g. "index.html", "assets/index-abc.js").
///
/// Returns `Some(Response)` if the file exists, `None` otherwise.
/// Falls back to `index.html` for unknown paths (SPA routing).
pub fn serve_embedded(path: &str) -> Option<Response<Body>> {
    // Normalize: strip leading slash, default to index.html
    let path = path.trim_start_matches('/');

    // Try the exact path first
    let file = WebAssets::get(path).or_else(|| {
        // SPA fallback: serve index.html for non-asset routes
        if !path.starts_with("assets/") && !path.starts_with("favicon") && !path.contains('.') {
            WebAssets::get("index.html")
        } else {
            None
        }
    })?;

    let mime = mime_guess::from_path(path).first_or_octet_stream();

    let mut response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime.as_ref())
        .body(Body::from(file.data.into_owned()))
        .ok()?;

    // Set cache headers for hashed assets
    if path.starts_with("assets/") {
        let val = HeaderValue::from_static("public, max-age=31536000, immutable");
        response.headers_mut().insert(header::CACHE_CONTROL, val);
    }

    Some(response)
}

/// Check whether embedded assets are available.
pub fn has_embedded() -> bool {
    WebAssets::iter().next().is_some()
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
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(Body::from(data))
            .unwrap_or_else(|_| StatusCode::NOT_FOUND.into_response());
    }

    StatusCode::NOT_FOUND.into_response()
}

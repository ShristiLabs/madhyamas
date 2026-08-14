//! Tiered body storage helpers (Phase 10a.1–10a.2).
//!
//! Shared utilities for the tiered body storage pattern described in
//! `docs/ENTERPRISE_PERF_SECURITY.md` §6.3–6.4:
//!
//! - Bodies smaller than [`INLINE_THRESHOLD`] are stored inline in the
//!   main `requests`/`responses` tables.
//! - Bodies >= [`INLINE_THRESHOLD`] are stored in a separate
//!   `traffic_bodies` table, compressed with zstd (level 3).
//! - The `compressed` flag on each row indicates whether zstd was
//!   applied; old (uncompressed) rows are read as-is.
//!
//! For bodies > 1MB, an S3-backed storage backend can be implemented by
//! extending the `storage_type` column with `'s3'` and storing the S3
//! key instead of the body bytes. See ENTERPRISE_PERF_SECURITY.md §6.3.

use crate::Error;

/// Bodies smaller than this (in bytes) are stored inline in the main
/// traffic table. Larger bodies go to the `traffic_bodies` table.
pub const INLINE_THRESHOLD: usize = 4 * 1024;

/// zstd compression level for body storage (level 3 — fast).
pub const ZSTD_LEVEL: i32 = 3;

/// Don't compress bodies smaller than this — the overhead outweighs
/// the savings.
pub const COMPRESSION_THRESHOLD: usize = 256;

/// Storage type for a body row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyStorageType {
    /// Stored inline in the main traffic table.
    Inline,
    /// Stored in the `traffic_bodies` table (PostgreSQL TOAST or
    /// SQLite BLOB).
    Toast,
    /// Stored in S3 (not implemented; the `traffic_bodies` row holds
    /// the S3 key instead of body bytes).
    S3,
}

impl BodyStorageType {
    /// Serialize to the string stored in the `storage_type` column.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Inline => "inline",
            Self::Toast => "toast",
            Self::S3 => "s3",
        }
    }

    /// Deserialize from the `storage_type` column string. Unknown
    /// values default to [`Self::Toast`] (the original behavior).
    pub fn parse_str(s: &str) -> Self {
        match s {
            "inline" => Self::Inline,
            "s3" => Self::S3,
            _ => Self::Toast,
        }
    }
}

/// Compress a body with zstd (level 3). Returns the compressed bytes
/// and `true` if compression actually reduced the size (otherwise the
/// original bytes are returned with `false`).
pub fn compress_body(body: &[u8]) -> (Vec<u8>, bool) {
    if body.len() <= COMPRESSION_THRESHOLD {
        return (body.to_vec(), false);
    }
    match zstd::stream::encode_all(body, ZSTD_LEVEL) {
        Ok(compressed) if compressed.len() < body.len() => (compressed, true),
        _ => (body.to_vec(), false),
    }
}

/// Decompress a body if the `compressed` flag is true; otherwise return
/// the bytes as-is.
pub fn decompress_body(body: &[u8], compressed: bool) -> Result<Vec<u8>, Error> {
    if !compressed {
        return Ok(body.to_vec());
    }
    zstd::stream::decode_all(body)
        .map_err(|e| Error::Config(format!("zstd decompression failed: {e}")))
}

/// Classify a body by size to determine its storage tier.
///
/// Returns `Some(type)` if the body should be stored (non-empty), or
/// `None` if the body is empty / not captured.
pub fn classify_body(body: Option<&[u8]>) -> Option<BodyStorageType> {
    let body = body?;
    if body.is_empty() {
        return None;
    }
    if body.len() < INLINE_THRESHOLD {
        Some(BodyStorageType::Inline)
    } else {
        Some(BodyStorageType::Toast)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_decompress_roundtrip() {
        let body = b"hello world ".repeat(100);
        let (compressed, was_compressed) = compress_body(&body);
        assert!(was_compressed);
        assert!(compressed.len() < body.len());
        let decompressed = decompress_body(&compressed, was_compressed).unwrap();
        assert_eq!(decompressed, body);
    }

    #[test]
    fn test_compress_small_body_not_compressed() {
        let body = b"hi";
        let (result, was_compressed) = compress_body(body);
        assert!(!was_compressed);
        assert_eq!(result, body);
    }

    #[test]
    fn test_decompress_uncompressed() {
        let body = b"plain bytes";
        let result = decompress_body(body, false).unwrap();
        assert_eq!(result, body);
    }

    #[test]
    fn test_classify_body() {
        assert_eq!(classify_body(None), None);
        assert_eq!(classify_body(Some(b"")), None);
        assert_eq!(classify_body(Some(b"small")), Some(BodyStorageType::Inline));
        let large = vec![0u8; INLINE_THRESHOLD + 1];
        assert_eq!(classify_body(Some(&large)), Some(BodyStorageType::Toast));
    }

    #[test]
    fn test_storage_type_roundtrip() {
        for t in [
            BodyStorageType::Inline,
            BodyStorageType::Toast,
            BodyStorageType::S3,
        ] {
            assert_eq!(BodyStorageType::parse_str(t.as_str()), t);
        }
    }
}

//! Rotating log file writer with on-demand rotation.
//!
//! This module implements a [`RotatingFileWriter`] that writes log events to
//! `<log_dir>/madhyamas.log` and rotates the file when:
//!
//! - The configured size cap is exceeded (per-write check), **or**
//! - The configured time period elapses (checked by a background task), **or**
//! - [`LogHandle::rotate_now`] is called (on-demand rotation).
//!
//! On rotation, the current file is renamed to
//! `madhyamas.log.<YYYY-MM-DD_HH-MM-SS>` and a fresh file is opened.
//! Archived files are pruned to `max_files` (oldest first).
//!
//! The writer implements [`std::io::Write`] and is wrapped in a
//! [`tracing_subscriber`] `MakeWriter` so it can be used as a `fmt` layer
//! alongside the stdout layer.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{Datelike, Local, Timelike};
use parking_lot::Mutex;
use serde_json::json;
use tracing_subscriber::fmt::MakeWriter;

use crate::{LogConfig, LogRotation};

/// Base log file name (without directory or rotation suffix).
const LOG_FILE_NAME: &str = "madhyamas.log";

/// Inner state guarded by the writer's mutex.
struct Inner {
    file: File,
    current_path: PathBuf,
    current_size: u64,
    /// (year, month, day) the current file was opened on (local time).
    opened_date: (i32, u32, u32),
    /// Hour (0-23) the current file was opened on (local time).
    opened_hour: u32,
}

/// A rotating log file writer.
///
/// Cloneable (inner state is behind an `Arc<Mutex<Inner>>`) so it can be
/// shared between the `MakeWriter` and the [`LogHandle`] that exposes
/// rotation/status/config APIs.
#[derive(Clone)]
pub struct RotatingFileWriter {
    inner: Arc<Mutex<Inner>>,
    config: Arc<Mutex<LogConfig>>,
    log_dir: PathBuf,
}

impl RotatingFileWriter {
    /// Create a new writer. Opens (or creates) `<log_dir>/madhyamas.log`
    /// in append mode. The `log_dir` is created if it does not exist.
    pub fn new(log_dir: impl Into<PathBuf>, config: LogConfig) -> io::Result<Self> {
        let log_dir = log_dir.into();
        std::fs::create_dir_all(&log_dir)?;

        let current_path = log_dir.join(LOG_FILE_NAME);
        let file = open_append(&current_path)?;
        let current_size = file.metadata().map(|m| m.len()).unwrap_or(0);
        let now = Local::now();
        let inner = Inner {
            file,
            current_path,
            current_size,
            opened_date: (now.year(), now.month(), now.day()),
            opened_hour: now.hour(),
        };

        Ok(Self {
            inner: Arc::new(Mutex::new(inner)),
            config: Arc::new(Mutex::new(config)),
            log_dir,
        })
    }

    /// Rotate the current file immediately (on-demand).
    ///
    /// The current file is flushed, closed, renamed to
    /// `madhyamas.log.<timestamp>`, and a fresh file opened. Old archived
    /// files are pruned to `max_files`. Returns the archived file path on
    /// success.
    pub fn rotate_now(&self) -> io::Result<PathBuf> {
        let archive_path = self.rotate_locked("manual")?;
        Ok(archive_path)
    }

    /// Update the rotation config at runtime. Takes effect immediately for
    /// subsequent writes and the background rotation task.
    pub fn update_config(&self, config: LogConfig) {
        *self.config.lock() = config;
    }

    /// Snapshot of the current config.
    pub fn config(&self) -> LogConfig {
        self.config.lock().clone()
    }

    /// The log directory.
    pub fn log_dir(&self) -> &Path {
        &self.log_dir
    }

    /// Current active log file path.
    pub fn current_path(&self) -> PathBuf {
        self.inner.lock().current_path.clone()
    }

    /// Current active log file size in bytes.
    pub fn current_size(&self) -> u64 {
        self.inner.lock().current_size
    }

    /// Check time-based rotation and rotate if the period has elapsed.
    /// Called periodically by the background task.
    pub fn check_time_rotation(&self) -> io::Result<bool> {
        let config = self.config.lock().clone();
        let should = match &config.rotation {
            LogRotation::Never | LogRotation::SizeMB { .. } => false,
            LogRotation::Hourly => {
                let now = Local::now();
                let inner = self.inner.lock();
                now.hour() != inner.opened_hour
                    || (now.year(), now.month(), now.day()) != inner.opened_date
            }
            LogRotation::Daily => {
                let now = Local::now();
                let inner = self.inner.lock();
                (now.year(), now.month(), now.day()) != inner.opened_date
            }
        };
        if should {
            self.rotate_locked("scheduled")?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Prune archived files to `max_files` (oldest first).
    pub fn prune(&self) -> io::Result<usize> {
        let max_files = self.config.lock().max_files;
        prune_archived(&self.log_dir, max_files)
    }

    /// List archived log files (newest first), with sizes.
    pub fn archived_files(&self) -> Vec<ArchivedLog> {
        list_archived(&self.log_dir)
    }

    /// Core rotation logic. Assumes the caller has not locked `inner`
    /// (this method acquires the lock internally).
    fn rotate_locked(&self, _reason: &str) -> io::Result<PathBuf> {
        let mut inner = self.inner.lock();
        // Flush before renaming so all buffered data lands in the archive.
        let _ = inner.file.flush();

        let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
        let archive_path = self
            .log_dir
            .join(format!("{}.{}", LOG_FILE_NAME, timestamp));

        // If the archive target already exists (rapid successive rotations
        // within the same second), append a counter.
        let archive_path = ensure_unique(&archive_path);

        // Rename the current file to the archive path. If rename fails
        // (e.g. cross-device), fall back to copy+truncate so we never lose
        // the ability to keep logging.
        if let Err(e) = std::fs::rename(&inner.current_path, &archive_path) {
            tracing::warn!(
                "log rename failed ({}), falling back to copy+truncate: {}",
                e,
                inner.current_path.display()
            );
            copy_and_truncate(&inner.current_path, &archive_path)?;
        }

        // Open a fresh file at the original path.
        inner.file = open_append(&inner.current_path)?;
        inner.current_size = 0;
        let now = Local::now();
        inner.opened_date = (now.year(), now.month(), now.day());
        inner.opened_hour = now.hour();

        Ok(archive_path)
    }

    /// Write bytes, rotating if the size cap is exceeded.
    fn write_checked(&self, buf: &[u8]) -> io::Result<()> {
        let config = self.config.lock().clone();
        let cap_bytes = config
            .rotation
            .effective_size_cap_mb(config.max_file_size_mb)
            * 1024
            * 1024;

        let mut inner = self.inner.lock();
        if cap_bytes > 0 && inner.current_size + buf.len() as u64 > cap_bytes {
            // Size cap exceeded — rotate. Drop the lock momentarily by
            // performing rotation inline (we already hold the lock; the
            // rotate logic operates on `inner` directly).
            let _ = inner.file.flush();
            let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
            let archive_path = self
                .log_dir
                .join(format!("{}.{}", LOG_FILE_NAME, timestamp));
            let archive_path = ensure_unique(&archive_path);
            if std::fs::rename(&inner.current_path, &archive_path).is_err() {
                let _ = copy_and_truncate(&inner.current_path, &archive_path);
            }
            inner.file = open_append(&inner.current_path)?;
            inner.current_size = 0;
            let now = Local::now();
            inner.opened_date = (now.year(), now.month(), now.day());
            inner.opened_hour = now.hour();
        }

        inner.file.write_all(buf)?;
        inner.current_size += buf.len() as u64;
        Ok(())
    }
}

impl Write for RotatingFileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.write_checked(buf)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.lock().file.flush()
    }
}

/// `MakeWriter` impl so `RotatingFileWriter` can be used with
/// `tracing_subscriber::fmt::layer().with_writer(...)`.
///
/// Each call to `make_writer` returns a fresh clone (the underlying state is
/// shared via `Arc`), so concurrent events serialize through the single
/// mutex-guarded file handle.
impl<'a> MakeWriter<'a> for RotatingFileWriter {
    type Writer = RotatingFileWriter;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// A handle to the logging subsystem, returned by [`init_logging`](super::init_logging).
///
/// Held for the program lifetime (the `tracing_appender` guard pattern) and
/// also stored in the API `AppState` so handlers can trigger on-demand
/// rotation and query status.
#[derive(Clone)]
pub struct LogHandle {
    writer: RotatingFileWriter,
    /// Non-blocking producer side of the async log writer (when async file
    /// logging is enabled). Clones share the same bounded buffer and
    /// dedicated writer thread.
    async_writer: Option<crate::async_log::AsyncFileWriter>,
    /// Guards the writer thread's lifetime; dropping the last handle
    /// flushes all buffered events.
    guard: Option<Arc<crate::async_log::WriterGuard>>,
}

impl LogHandle {
    /// Wrap a writer in a handle (synchronous file logging).
    pub fn new(writer: RotatingFileWriter) -> Self {
        Self {
            writer,
            async_writer: None,
            guard: None,
        }
    }

    /// Wrap a writer and its non-blocking layer in a handle. The guard must
    /// be the one returned alongside `async_writer` by
    /// [`AsyncFileWriter::new`](crate::async_log::AsyncFileWriter::new).
    pub fn with_async(
        writer: RotatingFileWriter,
        async_writer: crate::async_log::AsyncFileWriter,
        guard: crate::async_log::WriterGuard,
    ) -> Self {
        Self {
            writer,
            async_writer: Some(async_writer),
            guard: Some(Arc::new(guard)),
        }
    }

    /// The async producer-side writer (for building the `MakeWriter`
    /// layer). `None` when async file logging is disabled.
    pub fn async_writer(&self) -> Option<&crate::async_log::AsyncFileWriter> {
        self.async_writer.as_ref()
    }

    /// The underlying synchronous writer (for building the `MakeWriter`
    /// layer when async logging is disabled).
    pub fn writer(&self) -> &RotatingFileWriter {
        &self.writer
    }

    /// Rotate the current log file immediately.
    pub fn rotate_now(&self) -> io::Result<PathBuf> {
        let archive = self.writer.rotate_now()?;
        let pruned = self.writer.prune()?;
        if pruned > 0 {
            tracing::info!("pruned {} archived log file(s)", pruned);
        }
        Ok(archive)
    }

    /// Drain and flush all buffered log events (async mode). No-op in
    /// synchronous mode. Called on the graceful-shutdown path so no log
    /// line is lost; the final flush also happens when the last handle
    /// clone is dropped.
    pub fn flush(&self) {
        if let Some(guard) = &self.guard {
            guard.flush();
        } else {
            let mut w = self.writer.clone();
            let _ = w.flush();
        }
    }

    /// Update the rotation config at runtime. The async overflow policy
    /// (`async_mode`) takes effect immediately; `async_writing` and
    /// `async_buffer_size` take effect on the next restart (the writer
    /// thread is created once at startup).
    pub fn update_config(&self, config: LogConfig) {
        if let Some(async_writer) = &self.async_writer {
            async_writer.set_mode(config.async_mode);
        }
        self.writer.update_config(config);
    }

    /// Snapshot of the current config.
    pub fn config(&self) -> LogConfig {
        self.writer.config()
    }

    /// Build a JSON status payload describing the logging subsystem.
    pub fn status_json(&self) -> serde_json::Value {
        let cfg = self.writer.config();
        let current_path = self.writer.current_path();
        let current_size = self.writer.current_size();
        let archived = self.writer.archived_files();

        let archived_json: Vec<serde_json::Value> = archived
            .iter()
            .map(|a| {
                json!({
                    "name": a.name,
                    "path": a.path.to_string_lossy(),
                    "size_bytes": a.size_bytes,
                    "modified": a.modified,
                })
            })
            .collect();

        let async_json = match &self.async_writer {
            Some(w) => serde_json::to_value(w.status()).unwrap_or(serde_json::Value::Null),
            None => json!({
                "enabled": false,
                "mode": cfg.async_mode.as_str(),
                "buffer_size": cfg.async_buffer_size,
                "buffer_depth": 0,
                "high_water": 0,
                "dropped_events": 0,
                "written_events": 0,
            }),
        };

        json!({
            "enabled": cfg.enabled,
            "rotation": cfg.rotation.label(),
            "max_files": cfg.max_files,
            "max_file_size_mb": cfg.max_file_size_mb,
            "json_format": cfg.json_format,
            "async_writing": cfg.async_writing,
            "async": async_json,
            "log_dir": self.writer.log_dir().to_string_lossy(),
            "current_file": {
                "path": current_path.to_string_lossy(),
                "size_bytes": current_size,
            },
            "archived_files": archived_json,
            "archived_count": archived.len(),
        })
    }
}

/// An archived (rotated) log file entry.
#[derive(Debug, Clone)]
pub struct ArchivedLog {
    pub name: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub modified: String,
}

// ── helpers ──────────────────────────────────────────────────────────────

/// Open a file in append mode, creating it if necessary.
fn open_append(path: &Path) -> io::Result<File> {
    OpenOptions::new().create(true).append(true).open(path)
}

/// Append a numeric suffix if the path already exists, so rapid rotations
/// within the same second don't clobber each other.
fn ensure_unique(path: &Path) -> PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let base = path.to_string_lossy().to_string();
    for i in 1..1000 {
        let candidate = PathBuf::from(format!("{}.{}", base, i));
        if !candidate.exists() {
            return candidate;
        }
    }
    // Extremely unlikely fallback.
    path.to_path_buf()
}

/// Copy a file's contents to a destination then truncate the source to 0.
/// Used when `rename` fails (e.g. cross-device link).
fn copy_and_truncate(src: &Path, dst: &Path) -> io::Result<()> {
    std::fs::copy(src, dst)?;
    // Truncate by opening with write+truncate.
    let _ = OpenOptions::new().write(true).truncate(true).open(src)?;
    Ok(())
}

/// List archived log files (`madhyamas.log.*`) in a directory, newest first.
fn list_archived(dir: &Path) -> Vec<ArchivedLog> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    let mut files: Vec<ArchivedLog> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if !name.starts_with(&format!("{}.", LOG_FILE_NAME)) {
                return None;
            }
            let path = e.path();
            let size_bytes = e.metadata().ok()?.len();
            let modified = e
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .map(|t| {
                    let dt: chrono::DateTime<Local> = t.into();
                    dt.to_rfc3339()
                })
                .unwrap_or_default();
            Some(ArchivedLog {
                name,
                path,
                size_bytes,
                modified,
            })
        })
        .collect();
    // Newest first by name (timestamps sort lexicographically).
    files.sort_by(|a, b| b.name.cmp(&a.name));
    files
}

/// Prune archived log files to keep at most `max_files` (oldest deleted first).
fn prune_archived(dir: &Path, max_files: usize) -> io::Result<usize> {
    let mut files = list_archived(dir);
    if files.len() <= max_files {
        return Ok(0);
    }
    // Oldest first (reverse of the newest-first sort).
    files.reverse();
    let to_remove = files.len().saturating_sub(max_files);
    let mut removed = 0;
    for f in files.into_iter().take(to_remove) {
        if std::fs::remove_file(&f.path).is_ok() {
            removed += 1;
        }
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prune_keeps_max_files() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        // Create 5 archived files.
        for i in 0..5 {
            let path = dir.join(format!("{}.2026-01-0{}-00-00-00", LOG_FILE_NAME, i + 1));
            std::fs::write(&path, b"x").unwrap();
        }
        let removed = prune_archived(dir, 3).unwrap();
        assert_eq!(removed, 2);
        let remaining = list_archived(dir);
        assert_eq!(remaining.len(), 3);
    }

    #[test]
    fn size_rotation_triggers() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        let cfg = LogConfig {
            enabled: true,
            rotation: LogRotation::SizeMB { size_mb: 1 },
            max_files: 5,
            max_file_size_mb: 1,
            json_format: false,
            ..LogConfig::default()
        };
        let writer = RotatingFileWriter::new(dir, cfg).unwrap();
        // Write > 1 MB to trigger size-based rotation.
        let big = "x".repeat(2 * 1024 * 1024);
        {
            let mut w = writer.clone();
            w.write_all(big.as_bytes()).unwrap();
        }
        // At least one archive should exist now.
        let archived = list_archived(dir);
        assert!(!archived.is_empty(), "expected at least one archived file");
    }
}

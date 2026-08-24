//! Integration tests for the public logging API: async writer mode toggling,
//! write semantics, rotation labels, and rotation by rename.

use std::io::Write;

use madhyamas_core::async_log::AsyncFileWriter;
use madhyamas_core::log_rotation::RotatingFileWriter;
use madhyamas_core::{AsyncLogMode, LogConfig, LogRotation};
use madhyamas_test_utils::tmpdir;

// ── async writer ─────────────────────────────────────────────────────────

#[test]
fn mode_toggles_at_runtime() {
    let tmp = tmpdir("async-mode");
    let rotating =
        RotatingFileWriter::new(tmp.path(), LogConfig::default()).expect("create rotating writer");
    let (writer, guard) = AsyncFileWriter::new(rotating, 8, AsyncLogMode::Lossless);
    assert_eq!(writer.mode(), AsyncLogMode::Lossless);
    writer.set_mode(AsyncLogMode::Lossy);
    assert_eq!(writer.mode(), AsyncLogMode::Lossy);
    assert_eq!(writer.status().mode, "lossy");
    writer.set_mode(AsyncLogMode::Lossless);
    assert_eq!(writer.status().mode, "lossless");
    drop(guard);
}

#[test]
fn write_never_errors_and_reports_full_length() {
    let tmp = tmpdir("async-write");
    let rotating =
        RotatingFileWriter::new(tmp.path(), LogConfig::default()).expect("create rotating writer");
    let (mut writer, guard) = AsyncFileWriter::new(rotating, 8, AsyncLogMode::Lossless);
    let n = writer.write(b"hello async log\n").unwrap();
    assert_eq!(n, 16);
    assert!(writer.flush().is_ok());
    drop(guard);
}

// ── rotation ─────────────────────────────────────────────────────────────

#[test]
fn rotation_label() {
    assert_eq!(LogRotation::Never.label(), "never");
    assert_eq!(LogRotation::Hourly.label(), "hourly");
    assert_eq!(LogRotation::Daily.label(), "daily");
    assert_eq!(LogRotation::SizeMB { size_mb: 50 }.label(), "size (50 MB)");
}

#[test]
fn effective_size_cap() {
    assert_eq!(LogRotation::Never.effective_size_cap_mb(100), 100);
    assert_eq!(LogRotation::Daily.effective_size_cap_mb(100), 100);
    assert_eq!(
        LogRotation::SizeMB { size_mb: 25 }.effective_size_cap_mb(100),
        25
    );
}

#[test]
fn rotate_now_renames_and_reopens() {
    let tmp = tmpdir("rotate-now");
    let dir = tmp.path();
    let writer = RotatingFileWriter::new(dir, LogConfig::default()).unwrap();
    // Write some data.
    {
        let mut w = writer.clone();
        writeln!(w, "hello world").unwrap();
    }
    assert!(dir.join("madhyamas.log").exists());
    let archive = writer.rotate_now().unwrap();
    assert!(archive.exists());
    // A fresh file should now exist and be empty.
    assert_eq!(writer.current_size(), 0);
}

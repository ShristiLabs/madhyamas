//! Asynchronous (non-blocking) file logging layer.
//!
//! Decouples log-event emission (the proxy's request threads) from disk I/O
//! by moving all file writes onto a single dedicated writer thread fed by a
//! bounded buffer:
//!
//! - Producers (`tracing` fmt layer calling [`std::io::Write::write`]) only
//!   enqueue bytes — there is no mutex-guarded file I/O on the hot path.
//! - The writer thread owns the write side of a shared
//!   [`RotatingFileWriter`](crate::RotatingFileWriter) and drains the buffer,
//!   performing size-based rotation inline. The existing background
//!   time-rotation/prune task continues to operate on the same shared
//!   writer, so rotation boundaries never lose or duplicate lines (all
//!   writes are serialized through the writer's internal mutex).
//! - Overflow policy ([`AsyncLogMode`](crate::config::AsyncLogMode)):
//!   `lossless` (default) blocks producers until space frees up
//!   (backpressure); `lossy` drops the event and increments a counter
//!   surfaced via `GET /api/logs`.
//!
//! Graceful shutdown: [`WriterGuard`] (held by the [`LogHandle`], which the
//! main binary holds for the process lifetime) signals the writer thread to
//! drain every buffered event, flush, and exit. Dropping the last
//! [`LogHandle`] clone therefore flushes all pending log lines — including
//! on the SIGINT/SIGTERM graceful-shutdown path, where `run_proxy_server`
//! returns and the handle is dropped as the process unwinds.

use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, RecvError, SyncSender, TrySendError};
use std::sync::Arc;

use tracing_subscriber::fmt::MakeWriter;

use crate::config::AsyncLogMode;
use crate::RotatingFileWriter;

/// Message exchanged with the writer thread. `Flush` carries the ack
/// channel so the producer's `flush()` returns only after the file has been
/// flushed (and everything enqueued before the flush has been written).
enum Msg {
    Write(Vec<u8>),
    Flush(std::sync::mpsc::Sender<()>),
    Close,
}

/// Counters shared between producers and the writer thread.
#[derive(Default)]
struct Stats {
    /// Events successfully enqueued (producers side).
    sent: AtomicU64,
    /// Events fully written by the writer thread.
    written: AtomicU64,
    /// Events dropped in lossy mode because the buffer was full.
    dropped: AtomicU64,
    /// High-water mark of the buffer depth (sent - written).
    high_water: AtomicU64,
    /// When `true`, producers drop instead of blocking on a full buffer.
    lossy: AtomicBool,
}

/// Snapshot of the async writer state, surfaced in `GET /api/logs`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AsyncWriterStatus {
    pub enabled: bool,
    pub mode: String,
    pub buffer_size: usize,
    /// Approximate current buffer depth (enqueued but not yet written).
    pub buffer_depth: u64,
    /// Maximum buffer depth observed since startup.
    pub high_water: u64,
    /// Events dropped because the buffer was full (lossy mode only).
    pub dropped_events: u64,
    /// Events successfully written by the writer thread.
    pub written_events: u64,
}

/// Outcome of a single enqueue attempt (exposed for tests).
#[derive(Debug, PartialEq, Eq)]
enum EnqueueResult {
    /// The event was accepted into the buffer.
    Sent,
    /// The buffer was full; in lossy mode the event was dropped.
    Full,
}

/// A non-blocking writer front-end for the rotating log file.
///
/// Cloneable; all clones share the same bounded channel, stats, and writer
/// thread. Implements [`io::Write`] (returns immediately after enqueueing)
/// and [`MakeWriter`] so it can replace the synchronous
/// [`RotatingFileWriter`] in the `tracing_subscriber` file layer.
#[derive(Clone)]
pub struct AsyncFileWriter {
    tx: SyncSender<Msg>,
    stats: Arc<Stats>,
    buffer_size: usize,
}

impl AsyncFileWriter {
    /// Spawn the writer thread for `writer` and return the producer-side
    /// handle plus the guard that flushes on drop.
    ///
    /// `buffer_size` is the bounded channel capacity; `mode` selects the
    /// overflow policy (runtime-toggleable later via [`Self::set_mode`]).
    pub fn new(
        writer: RotatingFileWriter,
        buffer_size: usize,
        mode: AsyncLogMode,
    ) -> (Self, WriterGuard) {
        let buffer_size = buffer_size.max(1);
        let (tx, rx) = sync_channel::<Msg>(buffer_size);
        let stats = Arc::new(Stats {
            lossy: AtomicBool::new(mode == AsyncLogMode::Lossy),
            ..Stats::default()
        });

        let thread_stats = Arc::clone(&stats);
        let thread = std::thread::Builder::new()
            .name("madhyamas-log-writer".to_string())
            .spawn(move || writer_loop(writer, rx, thread_stats))
            .expect("failed to spawn log writer thread");

        (
            Self {
                tx: tx.clone(),
                stats,
                buffer_size,
            },
            WriterGuard {
                tx: Some(tx),
                handle: Some(thread),
            },
        )
    }

    /// Switch the overflow policy at runtime (no restart required).
    pub fn set_mode(&self, mode: AsyncLogMode) {
        self.stats
            .lossy
            .store(mode == AsyncLogMode::Lossy, Ordering::Relaxed);
    }

    /// Current overflow policy.
    pub fn mode(&self) -> AsyncLogMode {
        if self.stats.lossy.load(Ordering::Relaxed) {
            AsyncLogMode::Lossy
        } else {
            AsyncLogMode::Lossless
        }
    }

    /// Snapshot of the writer state for `GET /api/logs`.
    pub fn status(&self) -> AsyncWriterStatus {
        let sent = self.stats.sent.load(Ordering::Relaxed);
        let written = self.stats.written.load(Ordering::Relaxed);
        AsyncWriterStatus {
            enabled: true,
            mode: self.mode().as_str().to_string(),
            buffer_size: self.buffer_size,
            buffer_depth: sent.saturating_sub(written),
            high_water: self.stats.high_water.load(Ordering::Relaxed),
            dropped_events: self.stats.dropped.load(Ordering::Relaxed),
            written_events: written,
        }
    }

    /// Enqueue one event. In lossless mode this blocks until the bounded
    /// buffer has space (backpressure); in lossy mode it drops and counts.
    fn enqueue(&self, buf: Vec<u8>) -> EnqueueResult {
        if self.stats.lossy.load(Ordering::Relaxed) {
            match self.tx.try_send(Msg::Write(buf)) {
                Ok(()) => {
                    self.note_sent();
                    EnqueueResult::Sent
                }
                Err(TrySendError::Full(_)) => {
                    self.stats.dropped.fetch_add(1, Ordering::Relaxed);
                    EnqueueResult::Full
                }
                // Disconnected only happens after the guard dropped the
                // thread; behave like lossy so tracing never panics.
                Err(TrySendError::Disconnected(_)) => {
                    self.stats.dropped.fetch_add(1, Ordering::Relaxed);
                    EnqueueResult::Full
                }
            }
        } else {
            // Blocking send: parks the producer until the writer thread
            // frees a slot. This is the lossless backpressure path.
            if self.tx.send(Msg::Write(buf)).is_ok() {
                self.note_sent();
                EnqueueResult::Sent
            } else {
                self.stats.dropped.fetch_add(1, Ordering::Relaxed);
                EnqueueResult::Full
            }
        }
    }

    /// Record a successful enqueue and update the high-water mark.
    fn note_sent(&self) {
        let sent = self.stats.sent.fetch_add(1, Ordering::Relaxed) + 1;
        let depth = sent.saturating_sub(self.stats.written.load(Ordering::Relaxed));
        let mut hw = self.stats.high_water.load(Ordering::Relaxed);
        while depth > hw {
            match self.stats.high_water.compare_exchange_weak(
                hw,
                depth,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(cur) => hw = cur,
            }
        }
    }
}

impl Write for AsyncFileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // The fmt layer always writes complete events; copy and enqueue.
        self.enqueue(buf.to_vec());
        // Never surface an error: a failing log write must not propagate
        // into the traced application.
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        let (ack_tx, ack_rx) = std::sync::mpsc::channel();
        if self.tx.send(Msg::Flush(ack_tx)).is_err() {
            // Writer thread gone (shutdown); nothing to flush.
            return Ok(());
        }
        // The writer thread always replies (drops the ack sender on exit),
        // so this does not hang indefinitely after shutdown.
        let _ = ack_rx.recv();
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for AsyncFileWriter {
    type Writer = AsyncFileWriter;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// Writer-thread main loop: drains the channel into the rotating file.
fn writer_loop(mut writer: RotatingFileWriter, rx: Receiver<Msg>, stats: Arc<Stats>) {
    loop {
        match rx.recv() {
            Ok(Msg::Write(bytes)) => {
                // write_checked performs size-based rotation inline; errors
                // are swallowed (a failed log write must not kill the
                // writer thread) but still counted as written so buffer
                // depth accounting stays balanced.
                let _ = writer.write_all(&bytes);
                stats.written.fetch_add(1, Ordering::Relaxed);
            }
            Ok(Msg::Flush(ack)) => {
                // Drain any events already queued behind this flush marker
                // so "flush" means "everything enqueued before this call".
                while let Ok(msg) = rx.try_recv() {
                    match msg {
                        Msg::Write(bytes) => {
                            let _ = writer.write_all(&bytes);
                            stats.written.fetch_add(1, Ordering::Relaxed);
                        }
                        Msg::Flush(_) => {}
                        Msg::Close => {
                            finish(&mut writer, &rx, &stats);
                            let _ = ack.send(());
                            return;
                        }
                    }
                }
                let _ = writer.flush();
                let _ = ack.send(());
            }
            Ok(Msg::Close) => return finish(&mut writer, &rx, &stats),
            Err(RecvError) => return finish(&mut writer, &rx, &stats),
        }
    }
}

/// Drain everything still buffered, flush the file, and exit the thread.
fn finish(writer: &mut RotatingFileWriter, rx: &Receiver<Msg>, stats: &Stats) {
    while let Ok(msg) = rx.try_recv() {
        if let Msg::Write(bytes) = msg {
            let _ = writer.write_all(&bytes);
            stats.written.fetch_add(1, Ordering::Relaxed);
        }
    }
    let _ = writer.flush();
}

/// Keeps the writer thread alive and flushes buffered events on drop.
///
/// Shared (inside an `Arc`) by every [`LogHandle`](crate::LogHandle) clone;
/// only when the last handle is dropped does the guard signal `Close`, at
/// which point the writer thread drains every remaining buffered event,
/// flushes the file, and exits before the drop returns. This is the
/// `tracing_appender::non_blocking` `WorkerGuard` pattern.
pub struct WriterGuard {
    tx: Option<SyncSender<Msg>>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl WriterGuard {
    /// Ask the writer thread to drain and flush everything enqueued so far,
    /// then keep running.
    pub fn flush(&self) {
        if let Some(tx) = &self.tx {
            let (ack_tx, ack_rx) = std::sync::mpsc::channel::<()>();
            if tx.send(Msg::Flush(ack_tx)).is_ok() {
                let _ = ack_rx.recv();
            }
        }
    }
}

impl Drop for WriterGuard {
    fn drop(&mut self) {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(Msg::Close);
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Build the log-directory scan input used by tests to count lines across
/// the active file and its archives.
#[cfg(test)]
pub(crate) fn count_lines_in_dir(dir: &std::path::Path, file_name: &str) -> usize {
    let mut total = 0;
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.filter_map(|e| e.ok()) {
            let name = e.file_name().to_string_lossy().to_string();
            if name == file_name || name.starts_with(&format!("{}.", file_name)) {
                if let Ok(content) = std::fs::read_to_string(e.path()) {
                    total += content.lines().filter(|l| !l.trim().is_empty()).count();
                }
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LogConfig;

    fn tmp_writer() -> (tempfile::TempDir, RotatingFileWriter) {
        let tmp = tempfile::tempdir().unwrap();
        let writer = RotatingFileWriter::new(tmp.path(), LogConfig::default()).unwrap();
        (tmp, writer)
    }

    fn wait_written(writer: &AsyncFileWriter, target: u64, timeout_ms: u128) {
        let start = std::time::Instant::now();
        while writer.stats.written.load(Ordering::Relaxed) < target {
            if start.elapsed().as_millis() > timeout_ms {
                panic!(
                    "writer thread did not reach {} written events in time",
                    target
                );
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }

    #[test]
    fn all_events_written_and_flushed_on_drop() {
        let (tmp, rotating) = tmp_writer();
        let (mut writer, guard) = AsyncFileWriter::new(rotating, 4, AsyncLogMode::Lossless);
        // Send far more events than the buffer capacity: the writer thread
        // drains concurrently, and lossless mode must never drop.
        for i in 0..200 {
            writer
                .write_all(format!("event-{}\n", i).as_bytes())
                .unwrap();
        }
        guard.flush();
        let lines = count_lines_in_dir(tmp.path(), "madhyamas.log");
        assert_eq!(lines, 200, "lossless mode must not lose events");
        // Drop the guard: graceful-shutdown flush semantics.
        drop(guard);
        let lines = count_lines_in_dir(tmp.path(), "madhyamas.log");
        assert_eq!(lines, 200);
        assert_eq!(writer.status().dropped_events, 0);
    }

    #[test]
    fn lossy_mode_counts_drops_when_buffer_full() {
        let (tmp, rotating) = tmp_writer();
        let (writer, guard) = AsyncFileWriter::new(rotating, 2, AsyncLogMode::Lossy);
        // Fill the buffer faster than the writer can drain by bursting a
        // large number of sends from a tight loop; with capacity 2 some
        // sends must observe Full. This exercises the drop path (whether a
        // given run drops is timing-dependent, so we only assert the
        // invariant: dropped + written >= sent and drops >= 0).
        let mut sent_ok = 0;
        for i in 0..500 {
            if writer.enqueue(format!("burst-{}\n", i).into_bytes()) == EnqueueResult::Sent {
                sent_ok += 1;
            }
        }
        let status = writer.status();
        assert_eq!(status.mode, "lossy");
        assert_eq!(
            status.dropped_events as usize + sent_ok,
            500,
            "every event is either sent or counted as dropped"
        );
        drop(guard);
        let lines = count_lines_in_dir(tmp.path(), "madhyamas.log");
        assert_eq!(lines, sent_ok, "only non-dropped events reach the file");
    }

    #[test]
    fn lossless_backpressure_never_exceeds_bound() {
        let (_tmp, rotating) = tmp_writer();
        let (writer, guard) = AsyncFileWriter::new(rotating, 8, AsyncLogMode::Lossless);
        // With a healthy writer thread, lossless mode accepts everything
        // (producers park while the buffer is full). None are dropped.
        for i in 0..1000 {
            assert_eq!(
                writer.enqueue(format!("p-{}\n", i).into_bytes()),
                EnqueueResult::Sent
            );
        }
        wait_written(&writer, 1000, 5_000);
        let status = writer.status();
        assert_eq!(status.dropped_events, 0);
        assert_eq!(status.written_events, 1000);
        assert!(status.buffer_depth <= 8, "depth must respect the bound");
        drop(guard);
    }

    #[test]
    fn rotation_boundary_no_lost_or_duplicated_lines() {
        let (tmp, _default_writer) = tmp_writer();
        let cfg = LogConfig {
            enabled: true,
            rotation: crate::LogRotation::SizeMB { size_mb: 1 },
            max_file_size_mb: 1,
            ..LogConfig::default()
        };
        let rotating = RotatingFileWriter::new(tmp.path(), cfg).unwrap();
        let (mut writer, guard) = AsyncFileWriter::new(rotating, 16, AsyncLogMode::Lossless);
        // Write enough data (>1 MB) to trigger several size rotations while
        // events stream through the async buffer.
        let line = "x".repeat(200);
        let n = 10_000; // ~2 MB total
        for i in 0..n {
            writer
                .write_all(format!("{}-{}\n", line, i).as_bytes())
                .unwrap();
        }
        drop(guard);
        let total = count_lines_in_dir(tmp.path(), "madhyamas.log");
        assert_eq!(total, n, "no lost or duplicated lines across rotations");
    }

    #[test]
    fn status_reports_buffer_and_high_water() {
        let (_tmp, rotating) = tmp_writer();
        let (mut writer, guard) = AsyncFileWriter::new(rotating, 64, AsyncLogMode::Lossless);
        assert_eq!(writer.status().buffer_size, 64);
        for i in 0..50 {
            writer.write_all(format!("h-{}\n", i).as_bytes()).unwrap();
        }
        wait_written(&writer, 50, 5_000);
        let status = writer.status();
        assert!(status.high_water >= 1, "high-water mark must be tracked");
        assert_eq!(status.buffer_depth, 0);
        drop(guard);
    }
}

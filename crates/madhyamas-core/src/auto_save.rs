//! Auto Save — periodic session backup and rotation.
//!
//! Traffic is stored in SQLite in real time (every request/response is
//! persisted immediately), so Auto Save is not the primary persistence
//! mechanism. Instead, the [`AutoSaveManager`] runs a background task that
//! periodically exports the current session to a backup directory for
//! disaster recovery, prunes old backups, and optionally rotates the
//! session (starts a new one) after a configurable number of requests or
//! elapsed minutes.
//!
//! See [`docs/AUTO_SAVE.md`](../../docs/AUTO_SAVE.md) for the end-user guide.

use crate::config::AutoSaveConfig;
use crate::session::SessionManager;
use crate::storage::TrafficStoreBackend;
use crate::Error;
use chrono::Utc;
use parking_lot::RwLock;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;
use tracing::{debug, error, info};

/// Manages periodic session export (Auto Save) for disaster recovery and
/// session rotation.
///
/// The manager holds a live-updatable [`AutoSaveConfig`] (shared with the
/// API layer so runtime changes take effect), a reference to the
/// [`TrafficStore`] and [`SessionManager`], and a `oneshot` stop token used
/// for graceful shutdown of the background task.
///
/// The background task is started via [`AutoSaveManager::start`] and stopped
/// via [`AutoSaveManager::stop`]. Only one task runs at a time; calling
/// `start` again after `stop` is supported.
pub struct AutoSaveManager {
    config: Arc<RwLock<AutoSaveConfig>>,
    traffic_store: Arc<dyn TrafficStoreBackend + Send + Sync>,
    session_manager: Arc<SessionManager>,
    stop_token: RwLock<Option<oneshot::Sender<()>>>,
}

impl AutoSaveManager {
    /// Create a new AutoSaveManager.
    pub fn new(
        config: AutoSaveConfig,
        traffic_store: Arc<dyn TrafficStoreBackend + Send + Sync>,
        session_manager: Arc<SessionManager>,
    ) -> Arc<Self> {
        Arc::new(Self {
            config: Arc::new(RwLock::new(config)),
            traffic_store,
            session_manager,
            stop_token: RwLock::new(None),
        })
    }

    /// Create a new AutoSaveManager sharing an existing config `Arc`.
    ///
    /// This is used when the API layer holds the same `Arc<RwLock<AutoSaveConfig>>`
    /// so that runtime config changes are visible to the background task
    /// without a restart.
    pub fn with_shared_config(
        config: Arc<RwLock<AutoSaveConfig>>,
        traffic_store: Arc<dyn TrafficStoreBackend + Send + Sync>,
        session_manager: Arc<SessionManager>,
    ) -> Arc<Self> {
        Arc::new(Self {
            config,
            traffic_store,
            session_manager,
            stop_token: RwLock::new(None),
        })
    }

    /// Get a reference to the shared config.
    pub fn config(&self) -> &Arc<RwLock<AutoSaveConfig>> {
        &self.config
    }

    /// Start the background auto-save task.
    ///
    /// If the config is disabled, the task returns immediately. If a task is
    /// already running, this is a no-op. The task runs until [`stop`] is
    /// called or the stop token is dropped.
    pub fn start(self: Arc<Self>) {
        // Don't start a second task if one is already running.
        if self.stop_token.read().is_some() {
            return;
        }

        let (tx, rx) = oneshot::channel::<()>();
        *self.stop_token.write() = Some(tx);

        let manager = self.clone();
        tokio::spawn(async move {
            let config = manager.config.read().clone();
            if !config.enabled {
                debug!("Auto Save is disabled, background task exiting");
                return;
            }

            info!(
                "Auto Save enabled: interval={}s, format={}, output_dir={}",
                config.interval_seconds, config.export_format, config.output_dir
            );

            let mut interval = tokio::time::interval(Duration::from_secs(config.interval_seconds));
            // Skip the immediate first tick so the first snapshot happens
            // after one full interval has elapsed.
            interval.tick().await;

            // Pin the stop receiver so it can be polled across loop
            // iterations without being moved.
            let mut rx = std::pin::pin!(rx);

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = manager.run_cycle().await {
                            error!("Auto Save cycle failed: {}", e);
                        }
                    }
                    _ = &mut rx => {
                        info!("Auto Save manager stopped");
                        break;
                    }
                }
            }
        });
    }

    /// Stop the background auto-save task (graceful shutdown).
    ///
    /// Sends the stop signal to the background task. If no task is running,
    /// this is a no-op.
    pub fn stop(&self) {
        if let Some(tx) = self.stop_token.write().take() {
            let _ = tx.send(());
        }
    }

    /// Run a single auto-save cycle: check rotation, export snapshot, prune.
    ///
    /// This is the core logic invoked on each interval tick. It is also
    /// `pub` so it can be called directly (e.g. for a "save now" API or
    /// for tests).
    pub async fn run_cycle(&self) -> crate::Result<()> {
        let config = self.config.read().clone();
        if !config.enabled {
            return Ok(());
        }

        // Session rotation (request-count based).
        if let Some(threshold) = config.rotate_after_requests {
            let count = self.traffic_store.count().await.unwrap_or(0);
            if count >= threshold {
                info!(
                    "Auto Save: rotating session ({} requests >= threshold {})",
                    count, threshold
                );
                self.rotate_session().await?;
            }
        }

        // Session rotation (time based).
        if let Some(minutes) = config.rotate_after_minutes {
            let session_id = self.traffic_store.current_session_id();
            if let Ok(Some(session)) = self.session_manager.get_session(&session_id).await {
                let elapsed = Utc::now().signed_duration_since(session.created_at);
                if elapsed.num_minutes() >= minutes as i64 {
                    info!(
                        "Auto Save: rotating session ({} minutes >= threshold {})",
                        elapsed.num_minutes(),
                        minutes
                    );
                    self.rotate_session().await?;
                }
            }
        }

        self.save_snapshot(&config).await?;
        Ok(())
    }

    /// Start a new session, archiving the current one.
    ///
    /// The new session becomes the active session; subsequent traffic is
    /// recorded against it.
    ///
    /// Uses the shared instance state as a distributed lock to prevent both
    /// instances from rotating simultaneously: if another instance rotated
    /// within the last 30 seconds, this instance skips rotation and syncs
    /// the current session from shared state instead.
    async fn rotate_session(&self) -> crate::Result<()> {
        // Use shared state as a distributed lock to prevent both instances
        // from rotating simultaneously.
        let lock_key = "autosave_rotation_lock";
        let lock_value = format!("{}", Utc::now().timestamp());

        // If another instance just rotated (within the last 30 seconds),
        // skip and sync the current session from shared state instead.
        if let Ok(Some(existing)) = self.traffic_store.get_shared_state(lock_key).await {
            let existing_ts: i64 = existing.parse().unwrap_or(0);
            let now = Utc::now().timestamp();
            if now - existing_ts < 30 {
                info!("Auto Save: skipping rotation, another instance rotated recently");
                // Sync our local session id from shared state so we record
                // new traffic against the session the other instance just
                // created.
                if let Err(e) = self.traffic_store.sync_current_session().await {
                    debug!("Auto Save: session sync after skipped rotation failed: {e}");
                }
                return Ok(());
            }
        }

        // Perform the rotation.
        let new_session = self.session_manager.create_session(None).await?;
        self.traffic_store.switch_session(&new_session.id).await?;

        // Record the rotation timestamp so the other instance skips if its
        // timer fires within the lock window.
        let _ = self
            .traffic_store
            .set_shared_state(lock_key, &lock_value)
            .await;

        info!("Auto Save: rotated to new session {}", new_session.id);
        Ok(())
    }

    /// Export the current session to the backup directory and prune old
    /// backups.
    ///
    /// The file is named `session-<YYYYMMDD-HHMMSS>.<ext>` where `<ext>` is
    /// `har` or `json` depending on the export format. The output directory
    /// is created if it does not exist.
    pub async fn save_snapshot(&self, config: &AutoSaveConfig) -> crate::Result<()> {
        let session_id = self.traffic_store.current_session_id();
        if session_id.is_empty() {
            debug!("Auto Save: no active session, skipping snapshot");
            return Ok(());
        }

        // Ensure the output directory exists.
        fs::create_dir_all(&config.output_dir)
            .map_err(|e| Error::Config(format!("Failed to create auto-save directory: {}", e)))?;

        let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
        let ext = match config.export_format.as_str() {
            "har" => "har",
            "session" => "json",
            other => {
                return Err(Error::Config(format!(
                    "Unknown auto-save export format: '{}' (expected 'har' or 'session')",
                    other
                )));
            }
        };
        let filename = format!("session-{}.{}", timestamp, ext);
        let path = Path::new(&config.output_dir).join(&filename);

        match config.export_format.as_str() {
            "har" => {
                let har = self.traffic_store.export_har(&session_id).await?;
                let bytes = serde_json::to_vec_pretty(&har)?;
                fs::write(&path, bytes)?;
            }
            "session" => {
                let export = self.session_manager.export_session(&session_id).await?;
                let bytes = serde_json::to_vec_pretty(&export)?;
                fs::write(&path, bytes)?;
            }
            _ => unreachable!("format validated above"),
        }

        info!("Auto Save: wrote snapshot to {}", path.display());

        // Prune old backups.
        if let Err(e) = self.prune_backups(config) {
            error!("Auto Save: failed to prune backups: {}", e);
        }

        Ok(())
    }

    /// Delete the oldest backup files so that at most `max_backups` remain.
    ///
    /// Files are identified by the `session-*` prefix and sorted by
    /// modification time (oldest first). The oldest files beyond the limit
    /// are deleted.
    pub fn prune_backups(&self, config: &AutoSaveConfig) -> crate::Result<()> {
        let dir = Path::new(&config.output_dir);
        if !dir.exists() {
            return Ok(());
        }

        let entries = fs::read_dir(dir)
            .map_err(|e| Error::Config(format!("Failed to read auto-save directory: {}", e)))?;

        // Collect (path, modified_time) for files matching the backup pattern.
        let mut backups: Vec<(std::path::PathBuf, std::time::SystemTime)> = Vec::new();
        for entry in entries {
            let entry = entry
                .map_err(|e| Error::Config(format!("Failed to read directory entry: {}", e)))?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n,
                None => continue,
            };
            if !name.starts_with("session-") {
                continue;
            }
            let mtime = entry
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            backups.push((path, mtime));
        }

        // Sort oldest first (ascending by modified time).
        backups.sort_by_key(|(_, mtime)| *mtime);

        let excess = backups.len().saturating_sub(config.max_backups);
        for (path, _) in backups.iter().take(excess) {
            if let Err(e) = fs::remove_file(path) {
                error!(
                    "Auto Save: failed to delete old backup {}: {}",
                    path.display(),
                    e
                );
            } else {
                debug!("Auto Save: pruned old backup {}", path.display());
            }
        }

        Ok(())
    }
}

impl Drop for AutoSaveManager {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traffic::TrafficStore;

    /// Build an in-memory TrafficStore + SessionManager pair for tests.
    async fn test_store() -> (Arc<TrafficStore>, Arc<SessionManager>) {
        let store = TrafficStore::in_memory()
            .await
            .expect("failed to create in-memory store");
        let session_manager = Arc::new(SessionManager::new(store.clone()));
        (store, session_manager)
    }

    #[test]
    fn autosave_config_defaults() {
        let cfg = AutoSaveConfig::default();
        assert!(!cfg.enabled);
        assert_eq!(cfg.interval_seconds, 300);
        assert_eq!(cfg.export_format, "har");
        assert_eq!(cfg.max_backups, 10);
        assert!(cfg.rotate_after_requests.is_none());
        assert!(cfg.rotate_after_minutes.is_none());
        assert!(!cfg.output_dir.is_empty());
    }

    #[test]
    fn autosave_config_serializes_roundtrip() {
        let cfg = AutoSaveConfig {
            enabled: true,
            interval_seconds: 60,
            export_format: "session".to_string(),
            output_dir: "/tmp/backups".to_string(),
            max_backups: 5,
            rotate_after_requests: Some(100),
            rotate_after_minutes: Some(30),
        };
        let json = serde_json::to_string(&cfg).expect("serialize");
        let back: AutoSaveConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(cfg, back);
    }

    #[test]
    fn autosave_config_deserializes_with_defaults_for_missing_fields() {
        // A config JSON that omits auto_save entirely (as written by older
        // versions of Madhyamas) must deserialize with defaults.
        let json = r#"{"enabled": true}"#;
        let cfg: AutoSaveConfig = serde_json::from_str(json).expect("deserialize");
        assert!(cfg.enabled);
        assert_eq!(cfg.interval_seconds, 300);
        assert_eq!(cfg.export_format, "har");
        assert_eq!(cfg.max_backups, 10);
    }

    #[tokio::test]
    async fn save_snapshot_creates_har_file() {
        let tmp = tempfile::tempdir().expect("failed to create temp dir");
        let (store, session_manager) = test_store().await;

        let cfg = AutoSaveConfig {
            enabled: true,
            interval_seconds: 300,
            export_format: "har".to_string(),
            output_dir: tmp.path().to_string_lossy().to_string(),
            max_backups: 10,
            rotate_after_requests: None,
            rotate_after_minutes: None,
        };

        let manager = AutoSaveManager::new(cfg, store, session_manager);
        let cfg_snapshot = manager.config.read().clone();
        manager
            .save_snapshot(&cfg_snapshot)
            .await
            .expect("save_snapshot");

        // At least one file should exist.
        let files: Vec<_> = std::fs::read_dir(tmp.path())
            .expect("read_dir")
            .filter_map(|e| e.ok())
            .collect();
        assert!(!files.is_empty(), "expected at least one backup file");

        // The file should be valid HAR JSON with a "log" field.
        let har_file = files
            .iter()
            .find(|e| e.path().extension().and_then(|s| s.to_str()) == Some("har"))
            .expect("expected a .har file");
        let content = std::fs::read_to_string(har_file.path()).expect("read har file");
        let parsed: serde_json::Value = serde_json::from_str(&content).expect("parse har json");
        assert!(
            parsed.get("log").is_some(),
            "HAR JSON should have a 'log' field"
        );
    }

    #[tokio::test]
    async fn save_snapshot_creates_session_file() {
        let tmp = tempfile::tempdir().expect("failed to create temp dir");
        let (store, session_manager) = test_store().await;

        let cfg = AutoSaveConfig {
            enabled: true,
            interval_seconds: 300,
            export_format: "session".to_string(),
            output_dir: tmp.path().to_string_lossy().to_string(),
            max_backups: 10,
            rotate_after_requests: None,
            rotate_after_minutes: None,
        };

        let manager = AutoSaveManager::new(cfg, store, session_manager);
        let cfg_snapshot = manager.config.read().clone();
        manager
            .save_snapshot(&cfg_snapshot)
            .await
            .expect("save_snapshot");

        let files: Vec<_> = std::fs::read_dir(tmp.path())
            .expect("read_dir")
            .filter_map(|e| e.ok())
            .collect();
        assert!(!files.is_empty(), "expected at least one backup file");

        let session_file = files
            .iter()
            .find(|e| {
                e.path()
                    .file_name()
                    .and_then(|s| s.to_str())
                    .map(|n| n.starts_with("session-") && n.ends_with(".json"))
                    .unwrap_or(false)
            })
            .expect("expected a session-*.json file");
        let content = std::fs::read_to_string(session_file.path()).expect("read session file");
        let parsed: serde_json::Value = serde_json::from_str(&content).expect("parse session json");
        assert!(
            parsed.get("version").is_some(),
            "SessionExport should have a 'version' field"
        );
        assert!(
            parsed.get("session").is_some(),
            "SessionExport should have a 'session' field"
        );
    }

    #[tokio::test]
    async fn prune_backups_deletes_oldest() {
        let tmp = tempfile::tempdir().expect("failed to create temp dir");
        let (store, session_manager) = test_store().await;

        // Pre-create 5 backup files with distinct modification times.
        let dir = tmp.path();
        for i in 0..5u32 {
            let path = dir.join(format!("session-2024010{}-00000{}.har", i + 1, i));
            std::fs::write(&path, b"{}").expect("write file");
            // Set mtime so older indices are older in time.
            let time = std::time::SystemTime::now() + std::time::Duration::from_secs(i as u64);
            let _ = filetime::set_file_mtime(&path, filetime::FileTime::from_system_time(time));
        }

        let cfg = AutoSaveConfig {
            enabled: true,
            interval_seconds: 300,
            export_format: "har".to_string(),
            output_dir: dir.to_string_lossy().to_string(),
            max_backups: 2,
            rotate_after_requests: None,
            rotate_after_minutes: None,
        };

        let manager = AutoSaveManager::new(cfg, store, session_manager);
        manager
            .prune_backups(&manager.config.read().clone())
            .expect("prune_backups");

        let remaining: Vec<_> = std::fs::read_dir(dir)
            .expect("read_dir")
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .file_name()
                    .and_then(|s| s.to_str())
                    .map(|n| n.starts_with("session-"))
                    .unwrap_or(false)
            })
            .collect();
        assert_eq!(
            remaining.len(),
            2,
            "expected exactly 2 backups after pruning, got {}",
            remaining.len()
        );
    }

    #[tokio::test]
    async fn rotate_session_creates_new_session() {
        let (store, session_manager) = test_store().await;
        let old_session_id = store.current_session_id();

        let cfg = AutoSaveConfig {
            enabled: true,
            interval_seconds: 300,
            export_format: "har".to_string(),
            output_dir: std::env::temp_dir().to_string_lossy().to_string(),
            max_backups: 10,
            rotate_after_requests: Some(0),
            rotate_after_minutes: None,
        };

        let manager = AutoSaveManager::new(cfg, store.clone(), session_manager);
        manager.run_cycle().await.expect("run_cycle");

        let new_session_id = store.current_session_id();
        assert_ne!(
            old_session_id, new_session_id,
            "session should have rotated to a new one"
        );
    }

    #[tokio::test]
    async fn run_cycle_disabled_is_noop() {
        let tmp = tempfile::tempdir().expect("failed to create temp dir");
        let (store, session_manager) = test_store().await;

        let cfg = AutoSaveConfig {
            enabled: false,
            interval_seconds: 300,
            export_format: "har".to_string(),
            output_dir: tmp.path().to_string_lossy().to_string(),
            max_backups: 10,
            rotate_after_requests: None,
            rotate_after_minutes: None,
        };

        let manager = AutoSaveManager::new(cfg, store, session_manager);
        manager.run_cycle().await.expect("run_cycle");

        // No files should be written when disabled.
        let files: Vec<_> = std::fs::read_dir(tmp.path())
            .expect("read_dir")
            .filter_map(|e| e.ok())
            .collect();
        assert!(files.is_empty(), "no backup files expected when disabled");
    }

    #[tokio::test]
    async fn save_snapshot_rejects_unknown_format() {
        let tmp = tempfile::tempdir().expect("failed to create temp dir");
        let (store, session_manager) = test_store().await;

        let cfg = AutoSaveConfig {
            enabled: true,
            interval_seconds: 300,
            export_format: "xml".to_string(),
            output_dir: tmp.path().to_string_lossy().to_string(),
            max_backups: 10,
            rotate_after_requests: None,
            rotate_after_minutes: None,
        };

        let manager = AutoSaveManager::new(cfg, store, session_manager);
        let cfg_snapshot = manager.config.read().clone();
        let result = manager.save_snapshot(&cfg_snapshot).await;
        assert!(result.is_err(), "unknown format should produce an error");
    }
}

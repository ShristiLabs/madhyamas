//! Integration tests for the public auto-save API: config defaults/serde,
//! snapshot export (HAR and session formats), backup pruning, rotation,
//! and the disabled no-op path.

use std::sync::Arc;

use madhyamas_core::auto_save::AutoSaveManager;
use madhyamas_core::config::AutoSaveConfig;
use madhyamas_core::session::SessionManager;
use madhyamas_core::traffic::TrafficStore;
use madhyamas_test_utils::tmpdir;

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
    let tmp = tmpdir("autosave-har");
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
    let cfg_snapshot = manager.config().read().clone();
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
    let tmp = tmpdir("autosave-session");
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
    let cfg_snapshot = manager.config().read().clone();
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
    let tmp = tmpdir("autosave-prune");
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
        .prune_backups(&manager.config().read().clone())
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
    let tmp = tmpdir("autosave-disabled");
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
    let tmp = tmpdir("autosave-unknown");
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
    let cfg_snapshot = manager.config().read().clone();
    let result = manager.save_snapshot(&cfg_snapshot).await;
    assert!(result.is_err(), "unknown format should produce an error");
}

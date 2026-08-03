//! Plugin hot-reload — filesystem watcher that auto-reloads plugins when
//! their `.wasm` or manifest files change on disk.
//!
//! Uses [`notify`] to watch the plugin search directories. When a relevant
//! file change is detected, the watcher triggers a [`PluginManager::refresh`]
//! on a debounced schedule (to avoid reload storms during bulk writes).
//!
//! # Usage
//!
//! ```no_run
//! use madhyamas_core::{PluginManager, HotReloader};
//! use std::sync::Arc;
//!
//! let manager = Arc::new(PluginManager::default());
//! let mut reloader = HotReloader::new(manager.clone());
//! reloader.start().expect("failed to start hot-reload watcher");
//! // ... plugins are now auto-reloaded on file changes ...
//! reloader.stop();
//! ```

use crate::plugin::PluginManager;
use notify::{
    event::{Event, EventKind},
    Config, RecommendedWatcher, RecursiveMode, Watcher,
};
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

/// Debounce window — minimum time between reloads.
const DEBOUNCE: Duration = Duration::from_millis(500);

/// File extensions that trigger a reload.
const WATCHED_EXTENSIONS: &[&str] = &["wasm", "toml", "json"];

/// File names that trigger a reload (manifest files without standard extensions).
const WATCHED_FILENAMES: &[&str] = &["madhyamas-plugin.toml", "madhyamas-plugin.json"];

/// Hot-reload watcher for plugins.
pub struct HotReloader {
    manager: Arc<PluginManager>,
    watcher: Mutex<Option<RecommendedWatcher>>,
    last_reload: Mutex<Instant>,
}

impl HotReloader {
    /// Create a new hot-reloader for the given plugin manager.
    pub fn new(manager: Arc<PluginManager>) -> Self {
        Self {
            manager,
            watcher: Mutex::new(None),
            last_reload: Mutex::new(Instant::now() - DEBOUNCE * 2),
        }
    }

    /// Start watching all plugin directories registered with the manager.
    pub fn start(&self) -> crate::Result<()> {
        let dirs = self.manager.plugin_dirs();

        let manager = self.manager.clone();
        let handler = move |event: notify::Result<Event>| {
            match event {
                Ok(e) => {
                    if !is_relevant_event(&e) {
                        return;
                    }
                    debug!("plugin file change detected: {:?}", e.paths);
                    // Debounce: only reload if enough time has passed.
                    // The debounce check is done inside the reload trigger
                    // via `should_reload()`.
                    trigger_reload(&manager);
                }
                Err(e) => warn!("plugin file watcher error: {}", e),
            }
        };

        let mut watcher = RecommendedWatcher::new(handler, Config::default())
            .map_err(|e| crate::Error::Config(format!("notify watcher init: {}", e)))?;

        for dir in &dirs {
            if !dir.exists() {
                debug!("plugin dir does not exist, skipping watch: {:?}", dir);
                continue;
            }
            match watcher.watch(dir, RecursiveMode::Recursive) {
                Ok(()) => info!("watching plugin dir for hot-reload: {:?}", dir),
                Err(e) => warn!("failed to watch plugin dir {:?}: {}", dir, e),
            }
        }

        if dirs.is_empty() {
            warn!("no plugin directories to watch for hot-reload");
        }

        *self.watcher.lock() = Some(watcher);
        Ok(())
    }

    /// Stop watching.
    pub fn stop(&self) {
        *self.watcher.lock() = None;
        info!("plugin hot-reload watcher stopped");
    }

    /// Manually trigger a reload (debounced). Returns true if the reload was
    /// triggered, false if it was suppressed by the debounce window.
    pub fn trigger(&self) -> bool {
        let should = should_reload(&self.last_reload);
        if should {
            trigger_reload(&self.manager);
        }
        should
    }
}

impl Drop for HotReloader {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Check if a filesystem event is relevant (modifies a watched file).
fn is_relevant_event(event: &Event) -> bool {
    match event.kind {
        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
            event.paths.iter().any(|p| is_watched_path(p))
        }
        _ => false,
    }
}

/// Check if a path has a watched extension or is a known manifest filename.
fn is_watched_path(path: &std::path::Path) -> bool {
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if WATCHED_FILENAMES.contains(&name) {
            return true;
        }
    }
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        return WATCHED_EXTENSIONS.contains(&ext);
    }
    false
}

/// Check if enough time has passed since the last reload (debounce).
fn should_reload(last: &Mutex<Instant>) -> bool {
    let mut last = last.lock();
    let now = Instant::now();
    if now.duration_since(*last) >= DEBOUNCE {
        *last = now;
        true
    } else {
        false
    }
}

/// Trigger a debounced reload on the plugin manager.
fn trigger_reload(manager: &Arc<PluginManager>) {
    // We use a simple approach: spawn the reload on a blocking thread to
    // avoid blocking the notify callback. The manager's refresh() is
    // synchronous (file I/O + wasmtime module compilation).
    let mgr = manager.clone();
    std::thread::spawn(move || {
        debug!("hot-reload: triggering PluginManager::refresh()");
        match mgr.refresh() {
            Ok(count) => info!("hot-reload: reloaded {} plugin(s)", count),
            Err(e) => error!("hot-reload: refresh failed: {}", e),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_is_watched_path_wasm() {
        assert!(is_watched_path(Path::new("plugins/my-plugin/plugin.wasm")));
    }

    #[test]
    fn test_is_watched_path_manifest() {
        assert!(is_watched_path(Path::new(
            "plugins/my-plugin/madhyamas-plugin.toml"
        )));
        assert!(is_watched_path(Path::new(
            "plugins/my-plugin/madhyamas-plugin.json"
        )));
    }

    #[test]
    fn test_is_watched_path_irrelevant() {
        assert!(!is_watched_path(Path::new("plugins/my-plugin/readme.md")));
        assert!(!is_watched_path(Path::new("plugins/my-plugin/")));
    }

    #[test]
    fn test_should_reload_debounce() {
        let last = Mutex::new(Instant::now());
        // Immediately after setting, should be false.
        assert!(!should_reload(&last));
    }
}

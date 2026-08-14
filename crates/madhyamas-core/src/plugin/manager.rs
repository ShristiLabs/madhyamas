//! Plugin manager for loading, managing, and executing plugins.
//!
//! The manager ties together manifest discovery, the sandboxed WASM runtime
//! (when the `wasm-runtime` feature is enabled), SQLite persistence, the
//! installer, and lifecycle hook dispatch.
//!
//! # Execution model
//!
//! [`PluginManager::execute_hook`] iterates the enabled plugins subscribed to
//! a hook and dispatches each via [`PluginManager::execute_plugin_hook`].
//! That method records invocation statistics and (when a [`WasmRuntime`] is
//! attached) runs the plugin's `plugin.wasm` module. Manifest-only plugins
//! (no `.wasm`) are no-ops. Every invocation is recorded in the persistence
//! layer's audit log.

use super::installer::{InstallResult, InstallSource, PluginInstaller};
use super::persistence::PluginInvocationRow;
#[cfg(feature = "wasm-runtime")]
use super::WasmRuntime;
use super::{
    Plugin, PluginContext, PluginError, PluginHook, PluginManifest, PluginResult, PluginState,
    PluginStats,
};
use crate::storage::PluginStoreBackend;
use crate::Error;
use parking_lot::RwLock;
use semver::{Version, VersionReq};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Plugin manager
pub struct PluginManager {
    /// Loaded plugins
    plugins: RwLock<HashMap<String, Plugin>>,
    /// Plugin statistics
    stats: RwLock<HashMap<String, PluginStats>>,
    /// Plugin directories to search
    plugin_dirs: Vec<PathBuf>,
    /// Whether plugins are enabled globally
    enabled: RwLock<bool>,
    /// Sandboxed WASM runtime (present when the `wasm-runtime` feature is on).
    #[cfg(feature = "wasm-runtime")]
    wasm_runtime: Option<Arc<WasmRuntime>>,
    /// Async plugin store for plugin state, settings, and invocation logs.
    persistence: Option<Arc<dyn PluginStoreBackend + Send + Sync>>,
    /// Installer used for download/uninstall.
    installer: Option<Arc<PluginInstaller>>,
    /// Active timer task handles (plugin_id -> JoinHandle), so timers can be
    /// cancelled when a plugin is disabled/unloaded.
    timer_tasks: RwLock<HashMap<String, tokio::task::JoinHandle<()>>>,
}

impl PluginManager {
    pub fn new() -> Self {
        let home_plugin_dir = dirs::home_dir()
            .map(|h| h.join(".madhyamas/plugins"))
            .unwrap_or_else(|| {
                warn!(
                    "Could not determine home directory; \
                       falling back to ./plugins in the working directory"
                );
                PathBuf::from("./plugins")
            });

        Self {
            plugins: RwLock::new(HashMap::new()),
            stats: RwLock::new(HashMap::new()),
            plugin_dirs: vec![PathBuf::from("./plugins"), home_plugin_dir],
            enabled: RwLock::new(true),
            #[cfg(feature = "wasm-runtime")]
            wasm_runtime: None,
            persistence: None,
            installer: None,
            timer_tasks: RwLock::new(HashMap::new()),
        }
    }

    /// Attach a sandboxed WASM runtime (enables plugin code execution).
    #[cfg(feature = "wasm-runtime")]
    pub fn with_wasm_runtime(mut self, rt: Arc<WasmRuntime>) -> Self {
        self.wasm_runtime = Some(rt);
        self
    }

    /// Attach an async plugin store (enables state/settings/invocation logging).
    pub fn with_persistence(mut self, p: Arc<dyn PluginStoreBackend + Send + Sync>) -> Self {
        self.persistence = Some(p);
        self
    }

    /// Attach a plugin installer (enables install/uninstall).
    pub fn with_installer(mut self, i: Arc<PluginInstaller>) -> Self {
        self.installer = Some(i);
        self
    }

    /// Expand a leading `~` in a path to the user's home directory.
    ///
    /// Paths that do not start with `~` are returned unchanged. If the home
    /// directory cannot be determined, the original path is returned as-is
    /// (the caller will simply fail to find plugins there).
    fn expand_tilde(path: &Path) -> PathBuf {
        let s = path.to_string_lossy();
        if s == "~" {
            if let Some(home) = dirs::home_dir() {
                return home;
            }
        } else if let Some(rest) = s.strip_prefix("~/") {
            if let Some(home) = dirs::home_dir() {
                return home.join(rest);
            }
        } else if let Some(rest) = s.strip_prefix("~") {
            // `~user` style paths are not supported; return as-is.
            let _ = rest;
        }
        path.to_path_buf()
    }

    /// Add a plugin search directory.
    ///
    /// A leading `~` is expanded to the user's home directory using
    /// [`dirs::home_dir`], so callers may safely pass paths such as
    /// `~/some/dir`.
    pub fn add_plugin_dir(&mut self, path: PathBuf) {
        let expanded = Self::expand_tilde(&path);
        if expanded != path {
            debug!("Expanded plugin dir {:?} -> {:?}", path, expanded);
        }
        self.plugin_dirs.push(expanded);
    }

    /// Returns the list of plugin search directories.
    pub fn plugin_dirs(&self) -> Vec<PathBuf> {
        self.plugin_dirs.clone()
    }

    /// Discover plugins in all plugin directories
    pub fn discover_plugins(&self) -> crate::Result<Vec<PathBuf>> {
        let mut discovered = Vec::new();

        for dir in &self.plugin_dirs {
            if !dir.exists() {
                continue;
            }

            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                // Check for plugin manifest
                let manifest_path = path.join("madhyamas-plugin.toml");
                let manifest_json = path.join("madhyamas-plugin.json");

                if manifest_path.exists() || manifest_json.exists() {
                    discovered.push(path);
                }
            }
        }

        Ok(discovered)
    }

    /// Load a plugin from a directory.
    pub async fn load_plugin(&self, path: &Path) -> crate::Result<String> {
        // Try TOML first, then JSON
        let manifest_path = path.join("madhyamas-plugin.toml");
        let manifest_json = path.join("madhyamas-plugin.json");

        let manifest_content = if manifest_path.exists() {
            std::fs::read_to_string(&manifest_path)?
        } else if manifest_json.exists() {
            std::fs::read_to_string(&manifest_json)?
        } else {
            return Err(Error::Config("No plugin manifest found".into()));
        };

        // Parse manifest
        let manifest: PluginManifest = if manifest_path.exists() {
            toml::from_str(&manifest_content)
                .map_err(|e| Error::Config(format!("Failed to parse plugin manifest: {}", e)))?
        } else {
            serde_json::from_str(&manifest_content)
                .map_err(|e| Error::Config(format!("Failed to parse plugin manifest: {}", e)))?
        };

        let plugin_id = manifest.id.clone();

        // Validate the plugin's own version string is valid semver.
        if Version::parse(&manifest.version).is_err() {
            return Err(Error::Config(format!(
                "Plugin {} has invalid semver version: {}",
                plugin_id, manifest.version
            )));
        }

        // Restore persisted state (enabled flag + settings) if available.
        let (initial_state, restored_settings) = match &self.persistence {
            Some(p) => match p.load_state(&plugin_id).await? {
                Some(row) => {
                    let state = if row.enabled {
                        PluginState::Enabled
                    } else {
                        PluginState::Disabled
                    };
                    (state, row.settings)
                }
                None => (PluginState::Loaded, HashMap::new()),
            },
            None => (PluginState::Loaded, HashMap::new()),
        };

        let mut plugin = Plugin::from_manifest(manifest, &path.to_string_lossy());
        if !restored_settings.is_empty() {
            plugin.settings = restored_settings;
        }
        // Override state from persistence (or enabled_by_default from manifest).
        if self.persistence.is_some() {
            plugin.state = initial_state;
        }

        // Initialize stats
        self.stats
            .write()
            .insert(plugin_id.clone(), PluginStats::default());

        // Store plugin
        self.plugins
            .write()
            .insert(plugin_id.clone(), plugin.clone());

        // Fire on_load lifecycle hook (best-effort; errors are logged, not fatal).
        self.dispatch_lifecycle(&plugin_id, PluginHook::OnLoad);

        // If the plugin ended up enabled, fire on_enable and start its timer.
        if plugin.is_enabled() {
            self.dispatch_lifecycle(&plugin_id, PluginHook::OnEnable);
            self.maybe_start_timer(&plugin_id);
        }

        info!("Loaded plugin: {} from {:?}", plugin_id, path);
        Ok(plugin_id)
    }

    /// Re-scan all plugin directories and update the in-memory registry.
    ///
    /// This unloads plugins that no longer exist on disk, (re)loads newly
    /// discovered or changed manifests, and preserves settings for plugins
    /// that are still present. This is the counterpart to
    /// [`PluginRegistry::refresh`]: the registry refreshes the *catalog* of
    /// available plugins, while this refreshes the set of *loaded* plugins.
    pub async fn refresh(&self) -> crate::Result<usize> {
        let discovered = self.discover_plugins()?;

        // Collect current plugin ids and their settings so we can preserve
        // settings across the reload for plugins that are still present.
        let previous: HashMap<String, HashMap<String, serde_json::Value>> = {
            let plugins = self.plugins.read();
            plugins
                .iter()
                .map(|(id, p)| (id.clone(), p.settings.clone()))
                .collect()
        };

        // Unload all current plugins.
        let plugin_ids: Vec<String> = self.plugins.read().keys().cloned().collect();
        for id in plugin_ids {
            self.unload_plugin(&id);
        }

        // Load all discovered plugins, restoring preserved settings.
        let mut count = 0;
        for path in discovered {
            match self.load_plugin(&path).await {
                Ok(id) => {
                    if let Some(settings) = previous.get(&id) {
                        if !settings.is_empty() && self.persistence.is_none() {
                            self.update_settings(&id, settings.clone()).await;
                        }
                    }
                    count += 1;
                }
                Err(e) => warn!("Failed to load plugin from {:?}: {}", path, e),
            }
        }

        info!("Refreshed plugins: {} loaded", count);
        Ok(count)
    }

    /// Unload a plugin
    pub fn unload_plugin(&self, id: &str) -> bool {
        // Cancel any timer and fire on_unload.
        self.stop_timer(id);
        self.dispatch_lifecycle(id, PluginHook::OnUnload);
        #[cfg(feature = "wasm-runtime")]
        if let Some(rt) = &self.wasm_runtime_opt() {
            rt.drop_module(id);
        }

        let mut plugins = self.plugins.write();
        if let Some(mut plugin) = plugins.remove(id) {
            plugin.state = PluginState::Unloading;
            info!("Unloaded plugin: {}", id);
            true
        } else {
            false
        }
    }

    /// Enable a plugin
    pub async fn enable_plugin(&self, id: &str) -> Result<(), PluginError> {
        // Check dependencies and set state in a single write lock.
        let deps = {
            let mut plugins = self.plugins.write();
            let plugin = plugins.get_mut(id).ok_or_else(|| PluginError::NotFound {
                plugin_id: id.to_string(),
            })?;
            // Collect dependencies to check after releasing the lock.
            plugin.manifest.dependencies.clone()
        };
        // Check dependencies (needs a read lock, so must be outside the write lock).
        self.check_dependencies(&deps)?;
        // Now set the enabled state.
        {
            let mut plugins = self.plugins.write();
            let plugin = plugins.get_mut(id).ok_or_else(|| PluginError::NotFound {
                plugin_id: id.to_string(),
            })?;
            plugin.state = PluginState::Enabled;
        }
        // Persist + lifecycle.
        self.persist_state(id, true).await;
        self.dispatch_lifecycle(id, PluginHook::OnEnable);
        self.maybe_start_timer(id);
        info!("Enabled plugin: {}", id);
        Ok(())
    }

    /// Disable a plugin
    pub async fn disable_plugin(&self, id: &str) -> Result<(), PluginError> {
        self.stop_timer(id);
        {
            let mut plugins = self.plugins.write();
            let plugin = plugins.get_mut(id).ok_or_else(|| PluginError::NotFound {
                plugin_id: id.to_string(),
            })?;
            plugin.state = PluginState::Disabled;
        }
        self.persist_state(id, false).await;
        self.dispatch_lifecycle(id, PluginHook::OnDisable);
        info!("Disabled plugin: {}", id);
        Ok(())
    }

    /// Check plugin dependencies, including semver version constraints.
    ///
    /// Each entry in `dependencies` maps a dependency plugin id to a semver
    /// version requirement (e.g. `^1.2.3`, `>=2.0`, `*`). The dependency must
    /// already be loaded and its version must satisfy the requirement.
    fn check_dependencies(
        &self,
        dependencies: &HashMap<String, String>,
    ) -> Result<(), PluginError> {
        let plugins = self.plugins.read();

        for (dep_id, required_version) in dependencies {
            let plugin = plugins
                .get(dep_id)
                .ok_or_else(|| PluginError::DependencyError {
                    plugin_id: dep_id.clone(),
                    required_version: required_version.clone(),
                })?;

            // Parse the required version constraint as a semver VersionReq.
            let req =
                VersionReq::parse(required_version).map_err(|e| PluginError::VersionError {
                    required: required_version.clone(),
                    actual: format!("(invalid constraint: {})", e),
                })?;

            // Parse the dependency's actual version.
            let actual = Version::parse(&plugin.manifest.version).map_err(|e| {
                PluginError::VersionError {
                    required: required_version.clone(),
                    actual: format!("(invalid version: {})", e),
                }
            })?;

            if !req.matches(&actual) {
                return Err(PluginError::VersionError {
                    required: required_version.clone(),
                    actual: plugin.manifest.version.clone(),
                });
            }
        }

        Ok(())
    }

    /// Get a plugin
    pub fn get_plugin(&self, id: &str) -> Option<Plugin> {
        self.plugins.read().get(id).cloned()
    }

    /// Get all plugins
    pub fn get_plugins(&self) -> Vec<Plugin> {
        self.plugins.read().values().cloned().collect()
    }

    /// Get enabled plugins
    pub fn get_enabled_plugins(&self) -> Vec<Plugin> {
        self.plugins
            .read()
            .values()
            .filter(|p| p.is_enabled())
            .cloned()
            .collect()
    }

    /// Get plugins for a specific hook
    pub fn get_plugins_for_hook(&self, hook: PluginHook) -> Vec<Plugin> {
        let hook_str = hook.as_str();
        self.plugins
            .read()
            .values()
            .filter(|p| p.is_enabled() && p.manifest.hooks.iter().any(|h| h == hook_str))
            .cloned()
            .collect()
    }

    /// Get the settings schema for a plugin (for UI generation).
    pub fn get_settings_schema(&self, id: &str) -> Option<super::PluginSettingsSchema> {
        self.plugins
            .read()
            .get(id)
            .and_then(|p| p.manifest.settings.clone())
    }

    /// Get the current settings for a plugin.
    pub fn get_settings(&self, id: &str) -> Option<HashMap<String, serde_json::Value>> {
        self.plugins.read().get(id).map(|p| p.settings.clone())
    }

    /// Get recent invocation logs for a plugin.
    pub async fn get_invocations(&self, id: &str, limit: u32) -> Vec<PluginInvocationRow> {
        match &self.persistence {
            Some(p) => p.list_invocations(id, limit).await.unwrap_or_default(),
            None => Vec::new(),
        }
    }

    /// Execute a hook for all relevant plugins
    pub fn execute_hook(
        &self,
        hook: PluginHook,
        mut context: PluginContext,
    ) -> Vec<(String, PluginResult)> {
        let plugins = self.get_plugins_for_hook(hook);
        let mut results = Vec::new();

        for plugin in plugins {
            context.plugin_id = plugin.manifest.id.clone();
            context.settings = plugin.settings.clone();

            let result = self.execute_plugin_hook(&plugin.manifest.id, hook, &context);
            results.push((plugin.manifest.id.clone(), result));
        }

        results
    }

    /// Execute a hook for a specific plugin.
    ///
    /// When a [`WasmRuntime`] is attached (the `wasm-runtime` feature is on),
    /// this dispatches into the plugin's `plugin.wasm` module. Otherwise (or
    /// for manifest-only plugins with no `.wasm`), it records invocation
    /// statistics and returns a no-op [`PluginResult::cont`].
    fn execute_plugin_hook(
        &self,
        plugin_id: &str,
        hook: PluginHook,
        context: &PluginContext,
    ) -> PluginResult {
        let start = std::time::Instant::now();
        let plugin = self.plugins.read().get(plugin_id).cloned();

        let result = match plugin {
            Some(plugin) => self.run_plugin_code(&plugin, hook, context),
            None => PluginResult::error(&format!("plugin not found: {}", plugin_id)),
        };

        let duration_ms = start.elapsed().as_millis() as u64;

        // Update stats.
        {
            let mut stats = self.stats.write();
            if let Some(s) = stats.get_mut(plugin_id) {
                s.invocations += 1;
                s.last_invoked = Some(chrono::Utc::now());
                s.total_time_ms += duration_ms;
                if result.error.is_some() {
                    s.errors += 1;
                }
            }
        }

        // Record invocation in the audit log (fire-and-forget via
        // tokio::spawn — this method is called from sync contexts such as
        // the extension trait and timer tasks, so we cannot `.await` the
        // async store directly).
        if let Some(p) = &self.persistence {
            let row = PluginInvocationRow {
                id: uuid::Uuid::new_v4().to_string(),
                plugin_id: plugin_id.to_string(),
                hook: hook.as_str().to_string(),
                duration_ms,
                fuel_consumed: None,
                success: result.error.is_none(),
                error: result.error.clone(),
                logs: result.logs.clone(),
                modified: result.modified,
                timestamp: chrono::Utc::now(),
            };
            let store = p.clone();
            if let Ok(handle) = tokio::runtime::Handle::try_current() {
                handle.spawn(async move {
                    if let Err(e) = store.record_invocation(&row).await {
                        warn!("failed to record plugin invocation: {}", e);
                    }
                });
            }
        }

        result
    }

    /// Manifest-only / no-runtime result: a continue with a log line.
    fn noop_result(&self, plugin_id: &str, hook: PluginHook) -> PluginResult {
        PluginResult {
            logs: vec![format!(
                "plugin {} has no runtime (wasm-runtime feature disabled); skipping {}",
                plugin_id, hook
            )],
            ..PluginResult::cont()
        }
    }

    /// Dispatch a hook into the plugin's executable code.
    ///
    /// When the `wasm-runtime` feature is enabled and a [`WasmRuntime`] is
    /// attached, this runs the plugin's `plugin.wasm` module. Otherwise it
    /// returns a no-op continue result.
    fn run_plugin_code(
        &self,
        plugin: &Plugin,
        hook: PluginHook,
        context: &PluginContext,
    ) -> PluginResult {
        #[cfg(feature = "wasm-runtime")]
        {
            if let Some(rt) = self.wasm_runtime_opt() {
                return rt.execute_hook(plugin, hook, context);
            }
        }
        let _ = (hook, context);
        self.noop_result(&plugin.manifest.id, hook)
    }

    /// Dispatch a lifecycle hook (on_load/on_enable/on_disable/on_unload) for
    /// a plugin. Errors are logged and do not propagate.
    fn dispatch_lifecycle(&self, plugin_id: &str, hook: PluginHook) {
        // Only dispatch if the plugin subscribes to this hook.
        let subscribed = self
            .plugins
            .read()
            .get(plugin_id)
            .map(|p| p.manifest.hooks.iter().any(|h| h == hook.as_str()))
            .unwrap_or(false);
        if !subscribed {
            return;
        }
        let ctx = PluginContext::new(plugin_id, hook);
        let _ = self.execute_plugin_hook(plugin_id, hook, &ctx);
    }

    /// Persist the enabled flag (and current settings) for a plugin.
    async fn persist_state(&self, id: &str, enabled: bool) {
        if let Some(p) = &self.persistence {
            let settings = self.get_settings(id).unwrap_or_default();
            if let Err(e) = p.save_state(id, enabled, &settings).await {
                warn!("failed to persist plugin state for {}: {}", id, e);
            }
        }
    }

    /// Update plugin settings.
    pub async fn update_settings(
        &self,
        id: &str,
        settings: HashMap<String, serde_json::Value>,
    ) -> bool {
        {
            let mut plugins = self.plugins.write();
            if let Some(plugin) = plugins.get_mut(id) {
                plugin.settings = settings.clone();
                debug!("Updated settings for plugin: {}", id);
            } else {
                return false;
            }
        }
        // Persist + fire on_settings_change.
        if let Some(p) = &self.persistence {
            let enabled = self
                .plugins
                .read()
                .get(id)
                .map(|pl| pl.is_enabled())
                .unwrap_or(false);
            let _ = p.save_state(id, enabled, &settings).await;
        }
        self.dispatch_lifecycle(id, PluginHook::OnSettingsChange);
        true
    }

    /// Install a plugin from a source (download + verify + extract + load).
    pub async fn install_plugin(
        &self,
        source: &InstallSource,
        expected_checksum: Option<&str>,
    ) -> crate::Result<InstallResult> {
        let installer = self
            .installer
            .as_ref()
            .ok_or_else(|| Error::Config("no plugin installer configured".into()))?;
        let result = installer.install(source, expected_checksum).await?;
        // Load the newly installed plugin.
        let path = PathBuf::from(&result.path);
        if let Err(e) = self.load_plugin(&path).await {
            warn!("plugin installed but failed to load: {}", e);
        }
        Ok(result)
    }

    /// Uninstall a plugin (remove from disk + persistence + unload).
    pub async fn uninstall_plugin(&self, id: &str) -> crate::Result<()> {
        self.stop_timer(id);
        self.unload_plugin(id);
        if let Some(installer) = &self.installer {
            installer.uninstall(id).await?;
        } else {
            // Fallback: remove from the first plugin dir that contains it.
            if let Some(plugin) = self.get_plugin(id) {
                let _ = std::fs::remove_dir_all(&plugin.path);
            }
        }
        Ok(())
    }

    /// Get plugin stats
    pub fn get_stats(&self, id: &str) -> Option<PluginStats> {
        self.stats.read().get(id).cloned()
    }

    /// Enable/disable all plugins globally
    pub fn set_enabled(&self, enabled: bool) {
        *self.enabled.write() = enabled;
    }

    /// Check if plugins are enabled globally
    pub fn is_enabled(&self) -> bool {
        *self.enabled.read()
    }

    /// Reload all plugins.
    ///
    /// This is an alias for [`Self::refresh`]: it re-scans the plugin
    /// directories and reloads all discovered plugins.
    pub async fn reload_all(&self) -> crate::Result<usize> {
        self.refresh().await
    }

    /// Start timer tasks for all enabled plugins that declare a timer interval.
    /// Call this once at startup (within a tokio runtime).
    ///
    /// This is a no-op stub on a non-`Arc` reference; use
    /// [`Self::start_timers_arc`] (which requires `Arc<PluginManager>`) for
    /// real timer dispatch.
    pub fn start_all_timers(&self) {
        let _ = self;
    }

    /// Start the timer for a single plugin if it declares an interval.
    ///
    /// This is a no-op when called on a non-`Arc` reference: real timer
    /// dispatch requires `Arc<PluginManager>` (see [`Self::start_timers_arc`])
    /// because the spawned task must own a reference to the manager. We keep
    /// this method so that `enable_plugin` does not error when timers cannot
    /// be started inline; the caller is expected to call `start_timers_arc`
    /// once at startup.
    fn maybe_start_timer(&self, id: &str) {
        let _ = id;
    }

    /// Fully-wired timer start: requires the manager to be wrapped in an
    /// `Arc`. Spawns per-plugin `on_timer` dispatch loops.
    pub fn start_timers_arc(self: &Arc<Self>) {
        let ids: Vec<(String, u64)> = self
            .plugins
            .read()
            .iter()
            .filter(|(_, p)| p.is_enabled())
            .filter_map(|(id, p)| p.manifest.timer_interval_seconds.map(|s| (id.clone(), s)))
            .collect();
        for (id, secs) in ids {
            self.spawn_timer_arc(id, secs);
        }
    }

    fn spawn_timer_arc(self: &Arc<Self>, id: String, secs: u64) {
        if self.timer_tasks.read().contains_key(&id) {
            return;
        }
        let mgr = self.clone();
        let id_for_task = id.clone();
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(secs.max(1)));
            loop {
                interval.tick().await;
                if !mgr.is_enabled()
                    || !mgr
                        .get_plugin(&id_for_task)
                        .map(|p| p.is_enabled())
                        .unwrap_or(false)
                {
                    continue;
                }
                let ctx = PluginContext::new(&id_for_task, PluginHook::OnTimer);
                mgr.execute_plugin_hook(&id_for_task, PluginHook::OnTimer, &ctx);
            }
        });
        self.timer_tasks.write().insert(id, handle);
    }

    /// Stop the timer task for a plugin (if any).
    fn stop_timer(&self, id: &str) {
        if let Some(handle) = self.timer_tasks.write().remove(id) {
            handle.abort();
        }
    }

    /// Shut down all timers (called on application shutdown).
    pub fn shutdown_timers(&self) {
        let tasks: Vec<_> = self.timer_tasks.write().drain().map(|(_, h)| h).collect();
        for h in tasks {
            h.abort();
        }
    }

    #[cfg(feature = "wasm-runtime")]
    fn wasm_runtime_opt(&self) -> Option<Arc<WasmRuntime>> {
        self.wasm_runtime.clone()
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

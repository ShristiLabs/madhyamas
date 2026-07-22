//! Plugin manager for loading, managing, and executing plugins

use super::{
    Plugin, PluginContext, PluginError, PluginHook, PluginManifest, PluginResult, PluginState,
    PluginStats,
};
use crate::Error;
use parking_lot::RwLock;
use semver::{Version, VersionReq};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
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
        }
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

    /// Load a plugin from a directory
    pub fn load_plugin(&self, path: &Path) -> crate::Result<String> {
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

        let plugin = Plugin::from_manifest(manifest, &path.to_string_lossy());

        // Initialize stats
        self.stats
            .write()
            .insert(plugin_id.clone(), PluginStats::default());

        // Store plugin
        self.plugins.write().insert(plugin_id.clone(), plugin);

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
    pub fn refresh(&self) -> crate::Result<usize> {
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
            match self.load_plugin(&path) {
                Ok(id) => {
                    if let Some(settings) = previous.get(&id) {
                        if !settings.is_empty() {
                            self.update_settings(&id, settings.clone());
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
    pub fn enable_plugin(&self, id: &str) -> Result<(), PluginError> {
        let mut plugins = self.plugins.write();

        if let Some(plugin) = plugins.get_mut(id) {
            // Check dependencies
            self.check_dependencies(&plugin.manifest.dependencies)?;

            plugin.state = PluginState::Enabled;
            info!("Enabled plugin: {}", id);
            Ok(())
        } else {
            Err(PluginError::NotFound {
                plugin_id: id.to_string(),
            })
        }
    }

    /// Disable a plugin
    pub fn disable_plugin(&self, id: &str) -> Result<(), PluginError> {
        let mut plugins = self.plugins.write();

        if let Some(plugin) = plugins.get_mut(id) {
            plugin.state = PluginState::Disabled;
            info!("Disabled plugin: {}", id);
            Ok(())
        } else {
            Err(PluginError::NotFound {
                plugin_id: id.to_string(),
            })
        }
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
    /// # Not yet implemented: actual plugin code execution
    ///
    /// Discovering and parsing plugin manifests works (see [`Self::load_plugin`]
    /// and [`Self::refresh`]), but **invoking plugin code is not implemented**.
    /// There is no runtime (WASM via `wasmtime`, dynamic library via
    /// `libloading`, or embedded script engine) to execute plugin logic yet.
    ///
    /// This method currently records invocation statistics and returns a
    /// no-op [`PluginResult::cont`] for every hook, so the proxy pipeline is
    /// unaffected. When a runtime is added, replace the placeholder below with
    /// a dispatch into the loaded plugin's entry point.
    fn execute_plugin_hook(
        &self,
        plugin_id: &str,
        hook: PluginHook,
        _context: &PluginContext,
    ) -> PluginResult {
        let start = std::time::Instant::now();

        // Update stats
        {
            let mut stats = self.stats.write();
            if let Some(s) = stats.get_mut(plugin_id) {
                s.invocations += 1;
                s.last_invoked = Some(chrono::Utc::now());
            }
        }

        // TODO(plugin-runtime): Execute the plugin's hook handler.
        //
        // Candidate approaches (pick one; do NOT pull in `wasmtime` until the
        // design is settled):
        //   * WASM modules via `wasmtime` (sandboxed, portable, heavy dep).
        //   * Dynamic libraries via `libloading` (fast, unsafe, platform-specific).
        //   * Embedded scripting (e.g. `rune`, `mlua`, `boa`) for Lua/JS plugins.
        //
        // Until then, return a continue result so the proxy pipeline is a no-op.
        let _ = hook; // hook kind would select the plugin entry point
        let result = PluginResult::cont();

        // Update stats with execution time
        {
            let mut stats = self.stats.write();
            if let Some(s) = stats.get_mut(plugin_id) {
                s.total_time_ms += start.elapsed().as_millis() as u64;
                if result.error.is_some() {
                    s.errors += 1;
                }
            }
        }

        result
    }

    /// Update plugin settings
    pub fn update_settings(&self, id: &str, settings: HashMap<String, serde_json::Value>) -> bool {
        let mut plugins = self.plugins.write();

        if let Some(plugin) = plugins.get_mut(id) {
            plugin.settings = settings;
            debug!("Updated settings for plugin: {}", id);
            true
        } else {
            false
        }
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
    pub fn reload_all(&self) -> crate::Result<usize> {
        self.refresh()
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

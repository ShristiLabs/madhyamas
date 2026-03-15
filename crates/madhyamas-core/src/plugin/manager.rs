//! Plugin manager for loading, managing, and executing plugins

use super::{
    Plugin, PluginContext, PluginError, PluginHook, PluginManifest, PluginResult, PluginState,
    PluginStats,
};
use crate::Error;
use parking_lot::RwLock;
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
        Self {
            plugins: RwLock::new(HashMap::new()),
            stats: RwLock::new(HashMap::new()),
            plugin_dirs: vec![
                PathBuf::from("./plugins"),
                PathBuf::from("~/.madhyamas/plugins"),
            ],
            enabled: RwLock::new(true),
        }
    }

    /// Add a plugin search directory
    pub fn add_plugin_dir(&mut self, path: PathBuf) {
        self.plugin_dirs.push(path);
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

    /// Check plugin dependencies
    fn check_dependencies(
        &self,
        dependencies: &HashMap<String, String>,
    ) -> Result<(), PluginError> {
        let plugins = self.plugins.read();

        for (dep_id, required_version) in dependencies {
            if !plugins.contains_key(dep_id) {
                return Err(PluginError::DependencyError {
                    plugin_id: dep_id.clone(),
                    required_version: required_version.clone(),
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

    /// Execute a hook for a specific plugin
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

        // For now, return a placeholder result
        // In a real implementation, this would call into the plugin's native code
        let result = match hook {
            PluginHook::OnLoad => PluginResult::cont(),
            PluginHook::OnEnable => PluginResult::cont(),
            PluginHook::OnDisable => PluginResult::cont(),
            PluginHook::OnUnload => PluginResult::cont(),
            PluginHook::OnRequest => PluginResult::cont(),
            PluginHook::OnResponse => PluginResult::cont(),
            PluginHook::OnWebSocket => PluginResult::cont(),
            PluginHook::OnGrpc => PluginResult::cont(),
            PluginHook::OnSettingsChange => PluginResult::cont(),
            PluginHook::OnTimer => PluginResult::cont(),
        };

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

    /// Reload all plugins
    pub fn reload_all(&self) -> crate::Result<usize> {
        let mut count = 0;

        // Discover plugins
        let discovered = self.discover_plugins()?;

        // Unload all current plugins
        let plugin_ids: Vec<String> = self.plugins.read().keys().cloned().collect();
        for id in plugin_ids {
            self.unload_plugin(&id);
        }

        // Load all discovered plugins
        for path in discovered {
            match self.load_plugin(&path) {
                Ok(_) => count += 1,
                Err(e) => warn!("Failed to load plugin from {:?}: {}", path, e),
            }
        }

        info!("Reloaded {} plugins", count);
        Ok(count)
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

//! Plugin registry for discovering and sharing plugins

use super::{PluginCapability, PluginManifest};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, warn};

/// Plugin registry entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    /// Plugin manifest
    pub manifest: PluginManifest,
    /// Download URL
    pub download_url: String,
    /// Checksum (SHA-256)
    pub checksum: String,
    /// Download count
    pub downloads: u64,
    /// Rating (0-5)
    pub rating: f32,
    /// Number of ratings
    pub rating_count: u32,
    /// Plugin capabilities
    pub capabilities: Vec<PluginCapability>,
    /// Tags for search
    pub tags: Vec<String>,
    /// When the plugin was added to registry
    pub added_at: chrono::DateTime<chrono::Utc>,
    /// When the plugin was last updated
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Plugin registry (remote or local)
pub struct PluginRegistry {
    /// Registry URL
    url: Option<String>,
    /// Cached entries
    cache: HashMap<String, RegistryEntry>,
    /// When cache was last updated
    cache_updated: Option<chrono::DateTime<chrono::Utc>>,
    /// Cache TTL in seconds
    cache_ttl: u64,
    /// Local directories to scan for plugin manifests during [`refresh`].
    local_dirs: Vec<PathBuf>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        let home_plugin_dir = dirs::home_dir()
            .map(|h| h.join(".madhyamas/plugins"))
            .unwrap_or_else(|| {
                warn!(
                    "Could not determine home directory; \
                       local plugin scan will skip the home plugins dir"
                );
                PathBuf::from("./plugins")
            });

        Self {
            url: Some("https://registry.madhyamas.dev".to_string()),
            cache: HashMap::new(),
            cache_updated: None,
            cache_ttl: 3600, // 1 hour
            local_dirs: vec![PathBuf::from("./plugins"), home_plugin_dir],
        }
    }

    /// Create an offline registry
    pub fn offline() -> Self {
        let home_plugin_dir = dirs::home_dir()
            .map(|h| h.join(".madhyamas/plugins"))
            .unwrap_or_else(|| PathBuf::from("./plugins"));

        Self {
            url: None,
            cache: HashMap::new(),
            cache_updated: None,
            cache_ttl: 0,
            local_dirs: vec![PathBuf::from("./plugins"), home_plugin_dir],
        }
    }

    /// Set registry URL
    pub fn set_url(&mut self, url: String) {
        self.url = Some(url);
    }

    /// Add a local directory to scan for plugin manifests during [`refresh`].
    ///
    /// A leading `~` is expanded to the user's home directory using
    /// [`dirs::home_dir`].
    pub fn add_local_dir(&mut self, path: PathBuf) {
        let expanded = expand_tilde(&path);
        if expanded != path {
            debug!("Expanded registry local dir {:?} -> {:?}", path, expanded);
        }
        self.local_dirs.push(expanded);
    }

    /// Check if cache is valid
    fn cache_valid(&self) -> bool {
        if let Some(updated) = self.cache_updated {
            let now = chrono::Utc::now();
            let elapsed = (now - updated).num_seconds() as u64;
            elapsed < self.cache_ttl
        } else {
            false
        }
    }

    /// Refresh the registry cache.
    ///
    /// This clears the cache and re-populates it from two sources:
    ///
    /// 1. **Built-in plugins** — always added so the registry is usable
    ///    without network access.
    /// 2. **Local plugin directories** — every directory in `local_dirs`
    ///    (set via [`new`]/[`offline`]/[`add_local_dir`]) is scanned for
    ///    `madhyamas-plugin.toml` / `madhyamas-plugin.json` manifests, which
    ///    are parsed and added as registry entries.
    ///
    /// # Not yet implemented: remote registry fetch
    ///
    /// When `url` is set, this method does **not** yet perform an HTTP fetch
    /// from the remote registry. That path is intentionally left as a
    /// documented TODO so the proxy works fully offline. Implementing it
    /// requires a published registry API contract; see the TODO below.
    pub async fn refresh(&mut self) -> crate::Result<()> {
        self.cache.clear();

        // 1. Built-in plugins (always available, no I/O required).
        self.add_builtin_plugins();

        // 2. Locally discovered plugin manifests.
        self.scan_local_dirs()?;

        // TODO(plugin-registry): When a remote registry API is published,
        // fetch entries from `self.url` (if `Some`) using `reqwest` and merge
        // them into the cache, validating checksums and semver constraints.
        // Until then the registry is local/built-in only.
        if let Some(url) = &self.url {
            debug!(
                "Remote registry fetch is not implemented; skipping {} \
                 (using built-in + local plugins only)",
                url
            );
        }

        self.cache_updated = Some(chrono::Utc::now());
        Ok(())
    }

    /// Scan all local plugin directories and add discovered manifests to the
    /// cache as registry entries.
    fn scan_local_dirs(&mut self) -> crate::Result<()> {
        for dir in &self.local_dirs {
            if !dir.exists() {
                continue;
            }
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                let manifest_path = path.join("madhyamas-plugin.toml");
                let manifest_json = path.join("madhyamas-plugin.json");

                let (manifest_content, is_toml) = if manifest_path.exists() {
                    (std::fs::read_to_string(&manifest_path)?, true)
                } else if manifest_json.exists() {
                    (std::fs::read_to_string(&manifest_json)?, false)
                } else {
                    continue;
                };

                let manifest: PluginManifest = if is_toml {
                    toml::from_str(&manifest_content).map_err(|e| {
                        crate::Error::Config(format!("Failed to parse plugin manifest: {}", e))
                    })?
                } else {
                    serde_json::from_str(&manifest_content).map_err(|e| {
                        crate::Error::Config(format!("Failed to parse plugin manifest: {}", e))
                    })?
                };

                debug!("Discovered local plugin: {} from {:?}", manifest.id, path);

                self.cache.insert(
                    manifest.id.clone(),
                    RegistryEntry {
                        manifest,
                        download_url: String::new(),
                        checksum: String::new(),
                        downloads: 0,
                        rating: 0.0,
                        rating_count: 0,
                        capabilities: Vec::new(),
                        tags: Vec::new(),
                        added_at: chrono::Utc::now(),
                        updated_at: chrono::Utc::now(),
                    },
                );
            }
        }
        Ok(())
    }

    /// Add built-in plugins to the cache
    fn add_builtin_plugins(&mut self) {
        use chrono::Utc;

        // CORS Helper Plugin
        let cors_manifest = PluginManifest {
            id: "madhyamas.cors-helper".to_string(),
            name: "CORS Helper".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Automatically add CORS headers to responses".to_string()),
            author: Some("Madhyamas Team".to_string()),
            homepage: None,
            repository: None,
            min_version: Some("0.1.0".to_string()),
            max_version: None,
            license: Some("MIT".to_string()),
            dependencies: HashMap::new(),
            hooks: vec!["on_response".to_string()],
            settings: None,
            enabled_by_default: true,
        };

        self.cache.insert(
            cors_manifest.id.clone(),
            RegistryEntry {
                manifest: cors_manifest,
                download_url: String::new(),
                checksum: String::new(),
                downloads: 0,
                rating: 5.0,
                rating_count: 1,
                capabilities: vec![PluginCapability::InterceptResponse],
                tags: vec![
                    "cors".to_string(),
                    "headers".to_string(),
                    "development".to_string(),
                ],
                added_at: Utc::now(),
                updated_at: Utc::now(),
            },
        );

        // Request Logger Plugin
        let logger_manifest = PluginManifest {
            id: "madhyamas.request-logger".to_string(),
            name: "Request Logger".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Log all requests with detailed information".to_string()),
            author: Some("Madhyamas Team".to_string()),
            homepage: None,
            repository: None,
            min_version: Some("0.1.0".to_string()),
            max_version: None,
            license: Some("MIT".to_string()),
            dependencies: HashMap::new(),
            hooks: vec!["on_request".to_string()],
            settings: None,
            enabled_by_default: false,
        };

        self.cache.insert(
            logger_manifest.id.clone(),
            RegistryEntry {
                manifest: logger_manifest,
                download_url: String::new(),
                checksum: String::new(),
                downloads: 0,
                rating: 4.5,
                rating_count: 2,
                capabilities: vec![PluginCapability::InterceptRequest],
                tags: vec!["logging".to_string(), "debugging".to_string()],
                added_at: Utc::now(),
                updated_at: Utc::now(),
            },
        );

        // API Mock Helper Plugin
        let mock_manifest = PluginManifest {
            id: "madhyamas.mock-helper".to_string(),
            name: "API Mock Helper".to_string(),
            version: "1.0.0".to_string(),
            description: Some("Easily create and manage API mocks".to_string()),
            author: Some("Madhyamas Team".to_string()),
            homepage: None,
            repository: None,
            min_version: Some("0.1.0".to_string()),
            max_version: None,
            license: Some("MIT".to_string()),
            dependencies: HashMap::new(),
            hooks: vec!["on_request".to_string()],
            settings: None,
            enabled_by_default: false,
        };

        self.cache.insert(
            mock_manifest.id.clone(),
            RegistryEntry {
                manifest: mock_manifest,
                download_url: String::new(),
                checksum: String::new(),
                downloads: 0,
                rating: 4.8,
                rating_count: 5,
                capabilities: vec![
                    PluginCapability::InterceptRequest,
                    PluginCapability::UiPanel,
                ],
                tags: vec![
                    "mocking".to_string(),
                    "api".to_string(),
                    "testing".to_string(),
                ],
                added_at: Utc::now(),
                updated_at: Utc::now(),
            },
        );
    }

    /// Search for plugins
    pub async fn search(&mut self, query: &str) -> crate::Result<Vec<RegistryEntry>> {
        if !self.cache_valid() {
            self.refresh().await?;
        }

        let query = query.to_lowercase();
        let results: Vec<RegistryEntry> = self
            .cache
            .values()
            .filter(|entry| {
                let name_match = entry.manifest.name.to_lowercase().contains(&query);
                let desc_match = entry
                    .manifest
                    .description
                    .as_ref()
                    .map(|d| d.to_lowercase().contains(&query))
                    .unwrap_or(false);
                let tag_match = entry.tags.iter().any(|t| t.to_lowercase().contains(&query));

                name_match || desc_match || tag_match
            })
            .cloned()
            .collect();

        Ok(results)
    }

    /// Get a specific plugin
    pub async fn get(&mut self, id: &str) -> crate::Result<Option<RegistryEntry>> {
        if !self.cache_valid() {
            self.refresh().await?;
        }

        Ok(self.cache.get(id).cloned())
    }

    /// List all plugins
    pub async fn list(&mut self) -> crate::Result<Vec<RegistryEntry>> {
        if !self.cache_valid() {
            self.refresh().await?;
        }

        Ok(self.cache.values().cloned().collect())
    }

    /// List plugins by capability
    pub async fn list_by_capability(
        &mut self,
        capability: PluginCapability,
    ) -> crate::Result<Vec<RegistryEntry>> {
        if !self.cache_valid() {
            self.refresh().await?;
        }

        Ok(self
            .cache
            .values()
            .filter(|e| e.capabilities.contains(&capability))
            .cloned()
            .collect())
    }

    /// Get popular plugins
    pub async fn get_popular(&mut self, limit: usize) -> crate::Result<Vec<RegistryEntry>> {
        if !self.cache_valid() {
            self.refresh().await?;
        }

        let mut entries: Vec<_> = self.cache.values().cloned().collect();
        entries.sort_by_key(|b| std::cmp::Reverse(b.downloads));
        entries.truncate(limit);
        Ok(entries)
    }

    /// Get top-rated plugins
    pub async fn get_top_rated(&mut self, limit: usize) -> crate::Result<Vec<RegistryEntry>> {
        if !self.cache_valid() {
            self.refresh().await?;
        }

        let mut entries: Vec<_> = self.cache.values().cloned().collect();
        entries.sort_by(|a, b| {
            b.rating
                .partial_cmp(&a.rating)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        entries.truncate(limit);
        Ok(entries)
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Expand a leading `~` in a path to the user's home directory.
///
/// Paths that do not start with `~` are returned unchanged. If the home
/// directory cannot be determined, the original path is returned as-is.
fn expand_tilde(path: &std::path::Path) -> PathBuf {
    let s = path.to_string_lossy();
    if s == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    } else if let Some(rest) = s.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    path.to_path_buf()
}

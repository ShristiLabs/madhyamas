//! Plugin registry — discovers plugins from a GitHub-hosted catalog.
//!
//! The registry is a JSON catalog (`registry.json`) hosted in a GitHub
//! repository. The catalog lists available plugins with their download
//! URLs (GitHub release assets or raw files), checksums, and metadata.
//!
//! ## Catalog format
//!
//! The catalog is a JSON file at `{repo_root}/registry.json`:
//!
//! ```json
//! {
//!   "version": 1,
//!   "plugins": [
//!     {
//!       "manifest": { "id": "com.example.plugin", "name": "Example", ... },
//!       "download_url": "https://github.com/owner/repo/releases/download/v1.0/example.zip",
//!       "checksum": "sha256:abc123...",
//!       "downloads": 1234,
//!       "rating": 4.5,
//!       "rating_count": 10,
//!       "tags": ["example"],
//!       "added_at": "2024-01-01T00:00:00Z",
//!       "updated_at": "2024-01-01T00:00:00Z"
//!     }
//!   ]
//! }
//! ```
//!
//! ## GitHub URL resolution
//!
//! The registry URL is specified as a GitHub repo reference:
//! - `github:owner/repo` — uses the `main` branch
//! - `github:owner/repo@branch` — uses the specified branch/tag
//! - Full raw URL — `https://raw.githubusercontent.com/owner/repo/main/registry.json`
//!
//! ## Local discovery
//!
//! In addition to the remote catalog, the registry scans local plugin
//! directories (`~/.madhyamas/plugins/`, `./plugins/`) for installed plugins.
//! These appear in the registry with `download_url: ""` (already installed)
//! and `source: "local"`.

use super::{PluginCapability, PluginManifest};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// Plugin registry entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    /// Plugin manifest
    pub manifest: PluginManifest,
    /// Download URL (GitHub release asset or raw file URL).
    /// Empty for locally-discovered (already installed) plugins.
    pub download_url: String,
    /// Checksum (SHA-256, hex-encoded, optionally prefixed with "sha256:")
    pub checksum: String,
    /// Download count
    #[serde(default)]
    pub downloads: u64,
    /// Rating (0-5)
    #[serde(default)]
    pub rating: f32,
    /// Number of ratings
    #[serde(default)]
    pub rating_count: u32,
    /// Plugin capabilities
    #[serde(default)]
    pub capabilities: Vec<PluginCapability>,
    /// Tags for search
    #[serde(default)]
    pub tags: Vec<String>,
    /// Source of this entry
    #[serde(default = "default_source")]
    pub source: String,
    /// When the plugin was added to registry
    #[serde(default)]
    pub added_at: chrono::DateTime<chrono::Utc>,
    /// When the plugin was last updated
    #[serde(default)]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

fn default_source() -> String {
    "registry".to_string()
}

/// The catalog response from the GitHub-hosted registry.json
#[derive(Debug, Deserialize)]
struct Catalog {
    /// Catalog format version (currently 1)
    version: u32,
    /// Plugin entries
    plugins: Vec<RegistryEntry>,
}

/// Default GitHub repository for the plugin registry.
///
/// The catalog file (`registry.json`) lives at [`DEFAULT_CATALOG_PATH`]
/// within this repo. Plugin packages are distributed as GitHub release
/// assets attached to the same repo.
pub const DEFAULT_REGISTRY_REPO: &str = "shristilabs/madhyamas";

/// Default branch for the registry catalog
const DEFAULT_BRANCH: &str = "main";

/// Path to the catalog file within the registry repo.
const DEFAULT_CATALOG_PATH: &str = "plugins/registry.json";

/// Plugin registry backed by a GitHub-hosted catalog.
pub struct PluginRegistry {
    /// GitHub repo reference (e.g. "owner/repo" or "owner/repo@branch"),
    /// or a full raw URL to the catalog JSON.
    registry_ref: String,
    /// Resolved catalog URL (raw.githubusercontent.com/.../plugins/registry.json)
    catalog_url: String,
    /// Cached entries
    cache: HashMap<String, RegistryEntry>,
    /// When cache was last updated
    cache_updated: Option<chrono::DateTime<chrono::Utc>>,
    /// Cache TTL in seconds
    cache_ttl: u64,
    /// Local directories to scan for installed plugin manifests
    local_dirs: Vec<PathBuf>,
}

impl PluginRegistry {
    /// Create a new registry using the default GitHub repo.
    pub fn new() -> Self {
        Self::with_repo(DEFAULT_REGISTRY_REPO.to_string())
    }

    /// Create a new registry using a specific GitHub repo.
    ///
    /// `repo` can be:
    /// - `"owner/repo"` — uses the `main` branch
    /// - `"owner/repo@branch"` — uses the specified branch/tag
    /// - A full URL to the catalog JSON (used as-is)
    pub fn with_repo(repo: String) -> Self {
        let catalog_url = resolve_catalog_url(&repo);
        info!("Plugin registry catalog: {}", catalog_url);

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
            registry_ref: repo,
            catalog_url,
            cache: HashMap::new(),
            cache_updated: None,
            cache_ttl: 3600, // 1 hour
            local_dirs: vec![PathBuf::from("./plugins"), home_plugin_dir],
        }
    }

    /// Create an offline registry (no remote fetch, local discovery only).
    pub fn offline() -> Self {
        let home_plugin_dir = dirs::home_dir()
            .map(|h| h.join(".madhyamas/plugins"))
            .unwrap_or_else(|| PathBuf::from("./plugins"));

        Self {
            registry_ref: String::new(),
            catalog_url: String::new(),
            cache: HashMap::new(),
            cache_updated: None,
            cache_ttl: 0,
            local_dirs: vec![PathBuf::from("./plugins"), home_plugin_dir],
        }
    }

    /// Set the registry repo (e.g. "owner/repo" or "owner/repo@branch").
    pub fn set_repo(&mut self, repo: String) {
        self.catalog_url = resolve_catalog_url(&repo);
        self.registry_ref = repo;
        self.cache_updated = None; // invalidate cache
    }

    /// Returns the registry repo reference.
    pub fn repo(&self) -> &str {
        &self.registry_ref
    }

    /// Returns the resolved catalog URL.
    pub fn catalog_url(&self) -> &str {
        &self.catalog_url
    }

    /// Add a local directory to scan for plugin manifests during [`refresh`].
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
    /// 1. **Local plugin directories** — every directory in `local_dirs`
    ///    is scanned for plugin manifests. These represent already-installed
    ///    plugins and appear with `source: "local"` and empty `download_url`.
    /// 2. **Remote GitHub catalog** (when `catalog_url` is set and reachable)
    ///    — fetches `registry.json` from the GitHub repo and merges entries
    ///    into the cache. Network errors are logged and do not prevent the
    ///    refresh from completing with local entries.
    pub async fn refresh(&mut self) -> crate::Result<()> {
        self.cache.clear();

        // 1. Locally discovered (installed) plugin manifests.
        self.scan_local_dirs()?;

        // 2. Remote GitHub catalog (best-effort).
        if !self.catalog_url.is_empty() {
            if let Err(e) = self.fetch_remote_catalog(&self.catalog_url.clone()).await {
                warn!(
                    "Remote registry fetch failed (using local entries only): {}",
                    e
                );
            }
        }

        self.cache_updated = Some(chrono::Utc::now());
        Ok(())
    }

    /// Fetch the remote catalog JSON and merge entries into the cache.
    ///
    /// The catalog is a JSON file at the GitHub repo root (`registry.json`)
    /// served via `raw.githubusercontent.com`.
    async fn fetch_remote_catalog(&mut self, url: &str) -> crate::Result<()> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent(concat!("madhyamas/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| crate::Error::Config(format!("registry http client: {}", e)))?;

        let resp = client
            .get(url)
            .send()
            .await
            .map_err(|e| crate::Error::Config(format!("registry fetch: {}", e)))?;

        if !resp.status().is_success() {
            return Err(crate::Error::Config(format!(
                "registry catalog returned HTTP {} — make sure registry.json exists in the repo",
                resp.status()
            )));
        }

        // Read body as text first, then parse — gives better error messages
        // and avoids HTTP/2 framing issues with .json().
        let body = resp
            .text()
            .await
            .map_err(|e| crate::Error::Config(format!("registry body read: {}", e)))?;

        let catalog: Catalog = serde_json::from_str(&body)
            .map_err(|e| crate::Error::Config(format!("registry catalog parse: {}", e)))?;

        info!(
            "Fetched {} plugin(s) from remote registry (catalog v{})",
            catalog.plugins.len(),
            catalog.version
        );

        for entry in catalog.plugins {
            // If a local entry with the same id already exists (installed
            // plugin), merge the download_url + checksum from the remote
            // entry so the plugin can be reinstalled from the registry.
            if let Some(existing) = self.cache.get_mut(&entry.manifest.id) {
                if existing.source == "local" && existing.download_url.is_empty() {
                    existing.download_url = entry.download_url;
                    existing.checksum = entry.checksum;
                    existing.rating = entry.rating;
                    existing.rating_count = entry.rating_count;
                    existing.downloads = entry.downloads;
                }
            } else {
                self.cache.insert(entry.manifest.id.clone(), entry);
            }
        }

        Ok(())
    }

    /// Scan all local plugin directories and add discovered manifests to the
    /// cache as registry entries with `source: "local"`.
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
                        source: "local".to_string(),
                        added_at: chrono::Utc::now(),
                        updated_at: chrono::Utc::now(),
                    },
                );
            }
        }
        Ok(())
    }

    /// Search for plugins by name, description, or tags.
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
                let id_match = entry.manifest.id.to_lowercase().contains(&query);

                name_match || desc_match || tag_match || id_match
            })
            .cloned()
            .collect();

        Ok(results)
    }

    /// Get a specific plugin by ID.
    pub async fn get(&mut self, id: &str) -> crate::Result<Option<RegistryEntry>> {
        if !self.cache_valid() {
            self.refresh().await?;
        }

        Ok(self.cache.get(id).cloned())
    }

    /// List all plugins in the registry.
    pub async fn list(&mut self) -> crate::Result<Vec<RegistryEntry>> {
        if !self.cache_valid() {
            self.refresh().await?;
        }

        let mut entries: Vec<_> = self.cache.values().cloned().collect();
        // Sort: remote (installable) entries first, then by downloads.
        entries.sort_by(|a, b| {
            let a_remote = !a.download_url.is_empty();
            let b_remote = !b.download_url.is_empty();
            b_remote
                .cmp(&a_remote)
                .then_with(|| b.downloads.cmp(&a.downloads))
        });
        Ok(entries)
    }

    /// List plugins by capability.
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

    /// Get popular plugins (sorted by download count).
    pub async fn get_popular(&mut self, limit: usize) -> crate::Result<Vec<RegistryEntry>> {
        if !self.cache_valid() {
            self.refresh().await?;
        }

        let mut entries: Vec<_> = self.cache.values().cloned().collect();
        entries.sort_by_key(|b| std::cmp::Reverse(b.downloads));
        entries.truncate(limit);
        Ok(entries)
    }

    /// Get top-rated plugins.
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

    /// Returns the number of cached entries.
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Returns true if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

/// Resolve a GitHub repo reference to a raw.githubusercontent.com catalog URL.
///
/// Accepts:
/// - `"owner/repo"` → `https://raw.githubusercontent.com/owner/repo/main/plugins/registry.json`
/// - `"owner/repo@branch"` → `https://raw.githubusercontent.com/owner/repo/branch/plugins/registry.json`
/// - `"owner/repo@branch:path/to/catalog.json"` → custom path
/// - Full URL (contains "://") → returned as-is
/// - Empty string → empty (offline mode)
pub fn resolve_catalog_url(repo: &str) -> String {
    if repo.is_empty() {
        return String::new();
    }
    if repo.contains("://") {
        // Already a full URL
        return repo.to_string();
    }
    // Parse "owner/repo@branch:path/to/catalog.json"
    // or    "owner/repo@branch"
    // or    "owner/repo"
    let (repo_part, rest) = match repo.split_once('@') {
        Some((r, b)) => (r, b),
        None => (repo, DEFAULT_BRANCH),
    };
    let (branch, catalog_path) = match rest.split_once(':') {
        Some((b, p)) => (b, p),
        None => (rest, DEFAULT_CATALOG_PATH),
    };
    format!(
        "https://raw.githubusercontent.com/{}/{}/{}",
        repo_part, branch, catalog_path
    )
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Expand a leading `~` in a path to the user's home directory.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_catalog_url_default_repo() {
        let url = resolve_catalog_url("owner/repo");
        assert_eq!(
            url,
            "https://raw.githubusercontent.com/owner/repo/main/plugins/registry.json"
        );
    }

    #[test]
    fn test_resolve_catalog_url_with_branch() {
        let url = resolve_catalog_url("owner/repo@v1.0");
        assert_eq!(
            url,
            "https://raw.githubusercontent.com/owner/repo/v1.0/plugins/registry.json"
        );
    }

    #[test]
    fn test_resolve_catalog_url_custom_path() {
        let url = resolve_catalog_url("owner/repo@dev:catalog/plugins.json");
        assert_eq!(
            url,
            "https://raw.githubusercontent.com/owner/repo/dev/catalog/plugins.json"
        );
    }

    #[test]
    fn test_resolve_catalog_url_full_url() {
        let full = "https://example.com/catalog.json";
        let url = resolve_catalog_url(full);
        assert_eq!(url, full);
    }

    #[test]
    fn test_resolve_catalog_url_empty() {
        assert_eq!(resolve_catalog_url(""), "");
    }

    #[test]
    fn test_offline_registry() {
        let r = PluginRegistry::offline();
        assert!(r.catalog_url.is_empty());
        assert_eq!(r.local_dirs.len(), 2);
    }

    #[test]
    fn test_default_repo() {
        let r = PluginRegistry::new();
        assert!(r.catalog_url.contains(DEFAULT_REGISTRY_REPO));
        assert!(r.catalog_url.contains("plugins/registry.json"));
    }

    #[test]
    fn test_set_repo_invalidates_cache() {
        let mut r = PluginRegistry::new();
        r.cache_updated = Some(chrono::Utc::now());
        assert!(r.cache_valid());
        r.set_repo("other/repo@dev".to_string());
        assert!(!r.cache_valid());
        assert!(r.catalog_url.contains("other/repo"));
        assert!(r.catalog_url.contains("dev"));
    }

    #[test]
    fn test_registry_entry_source_default() {
        let json = r#"{"manifest":{"id":"test","name":"Test","version":"1.0","main":"plugin.wasm","hooks":[],"capabilities":[],"dependencies":{},"enabled_by_default":false,"network":false,"max_memory_pages":64,"fuel_limit":10000000,"tags":[],"panels":[]},"download_url":"https://example.com/test.zip","checksum":"abc"}"#;
        let entry: RegistryEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.source, "registry");
    }

    #[test]
    fn test_registry_entry_source_local() {
        let json = r#"{"manifest":{"id":"test","name":"Test","version":"1.0","main":"plugin.wasm","hooks":[],"capabilities":[],"dependencies":{},"enabled_by_default":false,"network":false,"max_memory_pages":64,"fuel_limit":10000000,"tags":[],"panels":[]},"download_url":"","checksum":"","source":"local"}"#;
        let entry: RegistryEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.source, "local");
    }

    #[test]
    fn test_catalog_deserialize() {
        let json = r#"{"version":1,"plugins":[]}"#;
        let catalog: Catalog = serde_json::from_str(json).unwrap();
        assert_eq!(catalog.version, 1);
        assert!(catalog.plugins.is_empty());
    }
}

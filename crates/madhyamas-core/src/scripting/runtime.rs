//! JavaScript runtime for Madhyamas scripting

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;

/// Declarative match filter for a script.
///
/// When present, the proxy checks the request against these fields *before*
/// invoking the JS engine.  All non-`None` fields must match for the script
/// to fire.  An empty / `None` match filter (the default) matches every
/// request — preserving backward compatibility.
///
/// Pattern fields (`url_pattern`, `host_pattern`, `path_pattern`) support
/// glob-style wildcards: `*` matches any sequence, `?` matches a single
/// character.  Matching is case-insensitive.  Examples:
/// - `host_pattern: "*.example.com"` matches `api.example.com` and `www.example.com`
/// - `url_pattern: "*/api/v2/*"` matches any URL containing `/api/v2/`
/// - `method: "GET"` matches only GET requests
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ScriptMatch {
    /// Glob pattern matched against the full request URL (case-insensitive).
    /// `*` matches any sequence, `?` matches a single character.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_pattern: Option<String>,
    /// Glob pattern matched against the request host (case-insensitive).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_pattern: Option<String>,
    /// Glob pattern matched against the request path (case-insensitive).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_pattern: Option<String>,
    /// HTTP method to match (case-insensitive, exact match).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
}

impl ScriptMatch {
    /// Returns `true` if no filter fields are set (matches everything).
    pub fn is_empty(&self) -> bool {
        self.url_pattern.is_none()
            && self.host_pattern.is_none()
            && self.path_pattern.is_none()
            && self.method.is_none()
    }

    /// Check whether a request matches this filter.
    ///
    /// `method`, `host`, `path`, and `url` are the request fields.  All
    /// non-`None` filter fields must match.  An empty filter matches
    /// everything.
    pub fn matches(&self, method: &str, host: &str, path: &str, url: &str) -> bool {
        if let Some(ref m) = self.method {
            if !m.eq_ignore_ascii_case(method) {
                return false;
            }
        }
        if let Some(ref p) = self.host_pattern {
            if !glob_match(p, host) {
                return false;
            }
        }
        if let Some(ref p) = self.path_pattern {
            if !glob_match(p, path) {
                return false;
            }
        }
        if let Some(ref p) = self.url_pattern {
            if !glob_match(p, url) {
                return false;
            }
        }
        true
    }
}

/// Case-insensitive glob match supporting `*` (any sequence) and `?`
/// (single character).  All other characters match literally.
fn glob_match(pattern: &str, input: &str) -> bool {
    let pattern = pattern.to_lowercase();
    let input = input.to_lowercase();
    glob_match_inner(pattern.as_bytes(), input.as_bytes())
}

fn glob_match_inner(pattern: &[u8], input: &[u8]) -> bool {
    let (mut p, mut i) = (0usize, 0usize);
    let (mut star_p, mut star_i): (Option<usize>, usize) = (None, 0);

    while i < input.len() {
        if p < pattern.len() && (pattern[p] == b'?' || pattern[p] == input[i]) {
            p += 1;
            i += 1;
        } else if p < pattern.len() && pattern[p] == b'*' {
            star_p = Some(p);
            star_i = i;
            p += 1;
        } else if let Some(sp) = star_p {
            p = sp + 1;
            star_i += 1;
            i = star_i;
        } else {
            return false;
        }
    }
    while p < pattern.len() && pattern[p] == b'*' {
        p += 1;
    }
    p == pattern.len()
}

/// Script metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Script {
    /// Unique identifier
    pub id: String,
    /// Script name
    pub name: String,
    /// Script description
    pub description: Option<String>,
    /// Script source code
    pub source: String,
    /// Enabled hooks
    pub hooks: Vec<String>,
    /// Whether the script is enabled
    pub enabled: bool,
    /// When the script was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the script was last modified
    pub modified_at: chrono::DateTime<chrono::Utc>,
    /// Priority (lower = runs earlier)
    pub priority: u32,
    /// Optional declarative match filter.  When set, the script only fires
    /// on requests that match all specified fields.  `None` (the default)
    /// matches every request.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub match_filter: Option<ScriptMatch>,
    /// Per-script error policy: what happens to the script chain when this
    /// script returns an error.  Defaults to [`ScriptErrorPolicy::StopChain`]
    /// (subsequent scripts for the same hook are skipped).  Set to
    /// [`ScriptErrorPolicy::Continue`] to keep running subsequent scripts
    /// after this one fails.
    #[serde(default)]
    pub on_error: ScriptErrorPolicy,
}

impl Script {
    pub fn new(name: String, source: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            description: None,
            source,
            hooks: Vec::new(),
            enabled: true,
            created_at: now,
            modified_at: now,
            priority: 100,
            match_filter: None,
            on_error: ScriptErrorPolicy::default(),
        }
    }
}

/// Script execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptExecution {
    /// Script ID
    pub script_id: String,
    /// Execution time in milliseconds
    pub duration_ms: u64,
    /// Whether the script succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Console output
    pub console: Vec<String>,
    /// When the execution happened
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Traffic entry ID this execution was associated with, if any.
    /// Set when the script runs as part of the proxy pipeline (on_request
    /// or on_response hook).  Absent for manual test/dry-run executions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub traffic_entry_id: Option<String>,
    /// Which hook triggered this execution (e.g. "on_request",
    /// "on_response").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hook: Option<String>,
}

/// Policy for how script errors affect the execution chain.
///
/// When multiple scripts are registered for the same hook, this controls
/// what happens when a script returns an error.  This is a per-script
/// setting: a script with [`ScriptErrorPolicy::StopChain`] stops the chain
/// when *it* errors, while [`ScriptErrorPolicy::Continue`] lets subsequent
/// scripts run.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScriptErrorPolicy {
    /// Continue running subsequent scripts after this one fails.  The
    /// error is logged and recorded in history but does not stop the
    /// chain.  The request/response continues through the pipeline
    /// normally.
    Continue,
    /// Stop the script chain when this script fails.  No subsequent
    /// scripts in the same hook are executed.  The request itself still
    /// continues through the proxy pipeline (it is not aborted) — only
    /// script processing stops.  This is the default.
    #[default]
    StopChain,
}

/// Script runtime configuration
///
/// The `timeout_ms` limit is enforced as a *soft* limit: the script always
/// runs to completion (boa does not support mid-execution preemption), but
/// if the execution time exceeds `timeout_ms` the result is replaced with a
/// timeout error.  `allow_network` and `allow_fs` are enforced by
/// construction — boa has no network or filesystem access and we do not
/// register any host functions that would expose those capabilities.
/// `max_memory_bytes` is reserved for future enforcement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptConfig {
    /// Maximum execution time in milliseconds (soft-enforced)
    pub timeout_ms: u64,
    /// Maximum memory usage in bytes (reserved for future enforcement)
    pub max_memory_bytes: usize,
    /// Enable console logging
    pub enable_console: bool,
    /// Allow network access from scripts (enforced: always false — no
    /// network functions are registered)
    pub allow_network: bool,
    /// Allow file system access from scripts (enforced: always false — no
    /// filesystem functions are registered)
    pub allow_fs: bool,
}

impl Default for ScriptConfig {
    fn default() -> Self {
        Self {
            timeout_ms: 5000,
            max_memory_bytes: 10 * 1024 * 1024, // 10MB
            enable_console: true,
            allow_network: false,
            allow_fs: false,
        }
    }
}

/// Partial update fields for a script.  Only `Some` fields are applied.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct UpdateScriptFields {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hooks: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_filter: Option<Option<ScriptMatch>>,
    /// Update the script priority (lower runs first).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<u32>,
    /// Update the per-script error policy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_error: Option<ScriptErrorPolicy>,
}

/// JavaScript runtime manager
pub struct ScriptRuntime {
    /// Registered scripts
    scripts: RwLock<HashMap<String, Script>>,
    /// Execution history (in-memory ring buffer)
    history: RwLock<Vec<ScriptExecution>>,
    /// Configuration
    config: RwLock<ScriptConfig>,
    /// Maximum in-memory history size
    max_history: usize,
    /// Optional SQLite connection for persistence
    db: RwLock<Option<Arc<parking_lot::Mutex<rusqlite::Connection>>>>,
}

impl ScriptRuntime {
    pub fn new(config: ScriptConfig) -> Self {
        Self {
            scripts: RwLock::new(HashMap::new()),
            history: RwLock::new(Vec::new()),
            config: RwLock::new(config),
            max_history: 1000,
            db: RwLock::new(None),
        }
    }

    /// Attach a SQLite connection for script and execution persistence.
    /// Creates the `scripts` / `script_executions` tables and loads any
    /// previously saved scripts.
    pub fn with_persistence(
        &self,
        conn: Arc<parking_lot::Mutex<rusqlite::Connection>>,
    ) -> crate::Result<()> {
        {
            let conn = conn.lock();
            super::persistence::ScriptPersistence::create_tables(&conn)?;
            let loaded = super::persistence::ScriptPersistence::load_scripts(&conn)?;
            if !loaded.is_empty() {
                let mut scripts = self.scripts.write();
                for script in loaded {
                    scripts.insert(script.id.clone(), script);
                }
            }
        }
        *self.db.write() = Some(conn);
        Ok(())
    }

    /// Get the current script configuration.
    pub fn config(&self) -> ScriptConfig {
        self.config.read().clone()
    }

    /// Update the script configuration.
    pub fn set_config(&self, config: ScriptConfig) {
        *self.config.write() = config;
    }

    /// Register a script (and persist it if a DB connection is attached).
    pub fn register_script(&self, script: Script) -> String {
        let id = script.id.clone();
        if let Some(ref db) = *self.db.read() {
            let conn = db.lock();
            if let Err(e) = super::persistence::ScriptPersistence::save_script(&conn, &script) {
                tracing::warn!("Failed to persist script {}: {}", script.id, e);
            }
        }
        self.scripts.write().insert(id.clone(), script);
        id
    }

    /// Remove a script (and delete it from the DB if attached).
    pub fn remove_script(&self, id: &str) -> bool {
        if let Some(ref db) = *self.db.read() {
            let conn = db.lock();
            if let Err(e) = super::persistence::ScriptPersistence::delete_script(&conn, id) {
                tracing::warn!("Failed to delete script {} from DB: {}", id, e);
            }
        }
        self.scripts.write().remove(id).is_some()
    }

    /// Get a script
    pub fn get_script(&self, id: &str) -> Option<Script> {
        self.scripts.read().get(id).cloned()
    }

    /// Get all scripts
    pub fn get_scripts(&self) -> Vec<Script> {
        self.scripts.read().values().cloned().collect()
    }

    /// Get scripts for a specific hook, sorted by priority (lower runs first).
    /// Ties are broken by creation time (oldest first) for stable ordering.
    pub fn get_scripts_for_hook(&self, hook: &str) -> Vec<Script> {
        let mut scripts: Vec<Script> = self
            .scripts
            .read()
            .values()
            .filter(|s| s.enabled && s.hooks.iter().any(|h| h == hook))
            .cloned()
            .collect();
        scripts.sort_by(|a, b| {
            a.priority
                .cmp(&b.priority)
                .then_with(|| a.created_at.cmp(&b.created_at))
        });
        scripts
    }

    /// Toggle a script
    pub fn toggle_script(&self, id: &str, enabled: bool) -> bool {
        let mut scripts = self.scripts.write();
        if let Some(script) = scripts.get_mut(id) {
            script.enabled = enabled;
            script.modified_at = chrono::Utc::now();
            let script_clone = script.clone();
            drop(scripts);
            self.persist_script(&script_clone);
            true
        } else {
            false
        }
    }

    /// Update script source
    pub fn update_script(&self, id: &str, source: String) -> bool {
        let mut scripts = self.scripts.write();
        if let Some(script) = scripts.get_mut(id) {
            script.source = source;
            script.modified_at = chrono::Utc::now();
            let script_clone = script.clone();
            drop(scripts);
            self.persist_script(&script_clone);
            true
        } else {
            false
        }
    }

    /// Reorder a script relative to its neighbors.
    /// `direction` is `"up"` (run earlier = lower priority) or `"down"`
    /// (run later = higher priority).  Swaps the target script with the
    /// adjacent script in the requested direction (sorted by priority, then
    /// created_at), then **renumbers** all scripts' priorities to distinct
    /// values (100, 110, 120, …) so the new order is stable regardless of
    /// whether the scripts previously shared a priority.  Returns `true` if
    /// a swap occurred.
    pub fn reorder_script(&self, id: &str, direction: &str) -> bool {
        let scripts = self.scripts.read();
        let mut sorted: Vec<Script> = scripts.values().cloned().collect();
        sorted.sort_by(|a, b| {
            a.priority
                .cmp(&b.priority)
                .then_with(|| a.created_at.cmp(&b.created_at))
        });
        drop(scripts);

        let pos = match sorted.iter().position(|s| s.id == id) {
            Some(p) => p,
            None => return false,
        };

        let swap_pos = match direction {
            "up" if pos > 0 => Some(pos - 1),
            "down" if pos < sorted.len() - 1 => Some(pos + 1),
            _ => None,
        };

        let sp = match swap_pos {
            Some(sp) => sp,
            None => return false,
        };

        // Swap the two entries in the sorted list, then renumber all
        // scripts with distinct priorities so the order is stable.
        sorted.swap(pos, sp);

        const BASE: u32 = 100;
        const STEP: u32 = 10;
        let now = chrono::Utc::now();
        let mut to_persist = Vec::new();
        {
            let mut scripts = self.scripts.write();
            for (i, script) in sorted.iter().enumerate() {
                let new_priority = BASE + (i as u32) * STEP;
                if let Some(s) = scripts.get_mut(&script.id) {
                    s.priority = new_priority;
                    s.modified_at = now;
                    to_persist.push(s.clone());
                }
            }
        }
        for script in to_persist {
            self.persist_script(&script);
        }
        true
    }

    /// Update script metadata (source, name, description, hooks, match filter).
    /// Only fields provided in the `UpdateScriptFields` struct are applied;
    /// `None` fields are left unchanged.
    pub fn update_script_fields(&self, id: &str, fields: UpdateScriptFields) -> bool {
        let mut scripts = self.scripts.write();
        if let Some(script) = scripts.get_mut(id) {
            if let Some(source) = fields.source {
                script.source = source;
            }
            if let Some(name) = fields.name {
                script.name = name;
            }
            if let Some(description) = fields.description {
                script.description = Some(description);
            }
            if let Some(hooks) = fields.hooks {
                script.hooks = hooks;
            }
            if let Some(match_filter) = fields.match_filter {
                script.match_filter = match_filter;
            }
            if let Some(priority) = fields.priority {
                script.priority = priority;
            }
            if let Some(on_error) = fields.on_error {
                script.on_error = on_error;
            }
            script.modified_at = chrono::Utc::now();
            let script_clone = script.clone();
            drop(scripts);
            self.persist_script(&script_clone);
            true
        } else {
            false
        }
    }

    /// Persist a script to the database (if attached).
    fn persist_script(&self, script: &Script) {
        if let Some(ref db) = *self.db.read() {
            let conn = db.lock();
            if let Err(e) = super::persistence::ScriptPersistence::save_script(&conn, script) {
                tracing::warn!("Failed to persist script {}: {}", script.id, e);
            }
        }
    }

    /// Validate a script's source code.
    ///
    /// Performs a fast structural check (non-empty, balanced braces/parens)
    /// followed by a full syntax parse via [`JsEngine::validate`].  This
    /// catches both obvious typos and real ECMAScript syntax errors before
    /// the script is stored.
    pub fn validate(source: &str) -> Result<(), String> {
        let trimmed = source.trim();
        if trimmed.is_empty() {
            return Err("Script source is empty".to_string());
        }
        if trimmed.len() > 100 * 1024 {
            return Err("Script source exceeds 100KB limit".to_string());
        }
        // Fast structural check (catches obvious typos without spinning up
        // a full JS context).
        validate_structure(trimmed)?;
        // Full syntax parse via the JS engine.
        super::engine::JsEngine::validate(trimmed)
    }

    /// Execute a script with the given context using the embedded JS engine.
    ///
    /// The execution result is recorded in the in-memory history ring buffer
    /// and, if a database connection is attached, in the `script_executions`
    /// table.
    pub fn execute(&self, script_id: &str, context: &super::ScriptContext) -> super::ScriptResult {
        let script = match self.get_script(script_id) {
            Some(s) => s,
            None => {
                return super::ScriptResult {
                    error: Some(format!("Script not found: {script_id}")),
                    ..Default::default()
                };
            }
        };

        let config = self.config.read().clone();
        let result =
            super::engine::JsEngine::execute(&script.source, &context.hook, context, &config);

        // Record execution in history (in-memory + DB).
        let execution = ScriptExecution {
            script_id: script_id.to_string(),
            duration_ms: result.duration_ms,
            success: result.error.is_none(),
            error: result.error.clone(),
            console: result.console.clone(),
            timestamp: chrono::Utc::now(),
            traffic_entry_id: Some(context.request_id.clone()).filter(|id| {
                // Don't record the placeholder IDs used by test/dry-run
                // contexts (e.g. "test-req").
                !id.starts_with("test-")
            }),
            hook: Some(context.hook.clone()),
        };
        self.record_execution(execution);

        result
    }

    /// Execute all scripts for a hook
    pub fn execute_hook(
        &self,
        hook: &str,
        context: &mut super::ScriptContext,
    ) -> Vec<super::ScriptResult> {
        let scripts = self.get_scripts_for_hook(hook);
        let mut results = Vec::new();

        for script in scripts {
            // Declarative match filter: skip scripts whose match_filter
            // does not match the current request.  An empty/None filter
            // matches everything (backward compatible).
            if let Some(ref filter) = script.match_filter {
                if !filter.is_empty() {
                    let (method, host, path, url) = context
                        .request
                        .as_ref()
                        .map(|r| {
                            (r.method.as_str(), r.host.as_str(), r.path.as_str(), r.url.as_str())
                        })
                        .unwrap_or(("", "", "", ""));
                    if !filter.matches(method, host, path, url) {
                        continue;
                    }
                }
            }
            let result = self.execute(&script.id, context);
            // Per-script error policy: if this script errored and its
            // policy is StopChain, skip all subsequent scripts for this
            // hook.  The request itself still flows through the proxy
            // pipeline normally — only script processing stops.
            let stop_chain = result.error.is_some()
                && script.on_error == ScriptErrorPolicy::StopChain;
            results.push(result);
            if stop_chain {
                break;
            }
        }

        results
    }

    /// Record an execution in history (in-memory + DB if attached).
    fn record_execution(&self, execution: ScriptExecution) {
        if let Some(ref db) = *self.db.read() {
            let conn = db.lock();
            if let Err(e) = super::persistence::ScriptPersistence::save_execution(&conn, &execution)
            {
                tracing::debug!("Failed to persist script execution: {}", e);
            }
        }
        let mut history = self.history.write();
        history.push(execution);
        if history.len() > self.max_history {
            let excess = history.len() - self.max_history;
            history.drain(0..excess);
        }
    }

    /// Get execution history across all scripts (most recent first).
    ///
    /// Reads from the database if attached (which has the full history),
    /// otherwise falls back to the in-memory ring buffer.
    pub fn get_history(&self, limit: Option<usize>) -> Vec<ScriptExecution> {
        let limit = limit.unwrap_or(100);
        if let Some(ref db) = *self.db.read() {
            let conn = db.lock();
            if let Ok(execs) =
                super::persistence::ScriptPersistence::load_all_executions(&conn, limit)
            {
                return execs;
            }
        }
        // Fallback: in-memory ring buffer.
        let history = self.history.read();
        let limit = limit.min(history.len());
        history.iter().rev().take(limit).cloned().collect()
    }

    /// Get execution history for a specific script.
    ///
    /// Reads from the database if attached (which has the full history),
    /// otherwise falls back to the in-memory ring buffer.
    pub fn get_script_history(&self, script_id: &str, limit: usize) -> Vec<ScriptExecution> {
        if let Some(ref db) = *self.db.read() {
            let conn = db.lock();
            if let Ok(execs) =
                super::persistence::ScriptPersistence::load_executions(&conn, script_id, limit)
            {
                return execs;
            }
        }
        // Fallback: filter in-memory history.
        let history = self.history.read();
        history
            .iter()
            .rev()
            .filter(|e| e.script_id == script_id)
            .take(limit)
            .cloned()
            .collect()
    }

    /// Clear execution history (in-memory + DB if attached).
    pub fn clear_history(&self) {
        if let Some(ref db) = *self.db.read() {
            let conn = db.lock();
            let _ = super::persistence::ScriptPersistence::clear_executions(&conn, None);
        }
        self.history.write().clear();
    }

    /// Get execution history for a specific traffic entry.  Returns all
    /// script executions (across all scripts) that were recorded for the
    /// given traffic entry ID, most recent first.  Reads from the database
    /// if attached, otherwise falls back to the in-memory ring buffer.
    pub fn get_executions_for_traffic_entry(
        &self,
        traffic_entry_id: &str,
        limit: usize,
    ) -> Vec<ScriptExecution> {
        if let Some(ref db) = *self.db.read() {
            let conn = db.lock();
            if let Ok(execs) = super::persistence::ScriptPersistence::load_executions_by_traffic(
                &conn,
                traffic_entry_id,
                limit,
            ) {
                return execs;
            }
        }
        // Fallback: filter in-memory history.
        let history = self.history.read();
        history
            .iter()
            .rev()
            .filter(|e| e.traffic_entry_id.as_deref() == Some(traffic_entry_id))
            .take(limit)
            .cloned()
            .collect()
    }

    /// Clear execution history for a single script (in-memory + DB if
    /// attached).  Only the records for `script_id` are removed; other
    /// scripts' history is preserved.
    pub fn clear_script_history(&self, script_id: &str) {
        if let Some(ref db) = *self.db.read() {
            let conn = db.lock();
            let _ =
                super::persistence::ScriptPersistence::clear_executions(&conn, Some(script_id));
        }
        let mut history = self.history.write();
        history.retain(|e| e.script_id != script_id);
    }

    /// Test (dry-run) a script against a context without affecting live
    /// traffic.  Returns the [`ScriptResult`] without recording it in
    /// history.
    pub fn test_script(&self, source: &str, context: &super::ScriptContext) -> super::ScriptResult {
        let config = self.config.read().clone();
        super::engine::JsEngine::execute(source, &context.hook, context, &config)
    }

    /// Load scripts from a directory
    pub fn load_from_directory(&self, path: &Path) -> crate::Result<usize> {
        let mut count = 0;

        if !path.exists() {
            return Ok(0);
        }

        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();

            if path
                .extension()
                .map(|e| e == "js" || e == "ts")
                .unwrap_or(false)
            {
                if let Ok(source) = std::fs::read_to_string(&path) {
                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unnamed")
                        .to_string();

                    let script = Script::new(name, source);
                    self.register_script(script);
                    count += 1;
                }
            }
        }

        Ok(count)
    }

    /// Export scripts to JSON
    pub fn export_scripts(&self) -> crate::Result<String> {
        let scripts = self.get_scripts();
        Ok(serde_json::to_string_pretty(&scripts)?)
    }

    /// Import scripts from JSON
    pub fn import_scripts(&self, json: &str) -> crate::Result<usize> {
        let scripts: Vec<Script> = serde_json::from_str(json)?;
        let count = scripts.len();

        for script in scripts {
            self.register_script(script);
        }

        Ok(count)
    }
}

impl Default for ScriptRuntime {
    fn default() -> Self {
        Self::new(ScriptConfig::default())
    }
}

/// Fast structural validation (balanced braces/parens/brackets, accounting
/// for string literals).  This is not a full parser but catches obvious
/// typos before spinning up a JS context.
fn validate_structure(source: &str) -> Result<(), String> {
    let mut braces = 0i32;
    let mut parens = 0i32;
    let mut brackets = 0i32;
    let mut in_string: Option<char> = None;
    let mut escaped = false;
    for ch in source.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if let Some(quote) = in_string {
            if ch == '\\' {
                escaped = true;
            } else if ch == quote {
                in_string = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' | '`' => in_string = Some(ch),
            '{' => braces += 1,
            '}' => braces -= 1,
            '(' => parens += 1,
            ')' => parens -= 1,
            '[' => brackets += 1,
            ']' => brackets -= 1,
            _ => {}
        }
    }
    if braces != 0 {
        return Err(format!("Unbalanced braces: off by {}", braces));
    }
    if parens != 0 {
        return Err(format!("Unbalanced parentheses: off by {}", parens));
    }
    if brackets != 0 {
        return Err(format!("Unbalanced brackets: off by {}", brackets));
    }
    Ok(())
}

/// Script template examples
pub struct ScriptTemplates;

impl ScriptTemplates {
    /// Log all requests
    pub fn log_requests() -> Script {
        let mut s = Script::new(
            "Log Requests".to_string(),
            r#"
// Log all incoming requests
function onRequest(request, context) {
    console.log(request.method + ' ' + request.url);
    console.log('Headers: ' + JSON.stringify(request.headers));
    return { continue: true };
}
"#
            .to_string(),
        );
        s.hooks = vec!["on_request".to_string()];
        s.description = Some("Logs method, URL, and headers for every request.".to_string());
        s
    }

    /// Add CORS headers to responses
    pub fn add_cors() -> Script {
        let mut s = Script::new(
            "Add CORS".to_string(),
            r#"
// Add CORS headers to all responses
function onResponse(request, response, context) {
    response.headers['Access-Control-Allow-Origin'] = '*';
    response.headers['Access-Control-Allow-Methods'] = 'GET, POST, PUT, DELETE, OPTIONS';
    response.headers['Access-Control-Allow-Headers'] = '*';
    return { continue: true, modified: true };
}
"#
            .to_string(),
        );
        s.hooks = vec!["on_response".to_string()];
        s.description = Some("Adds Access-Control-Allow-* headers to all responses.".to_string());
        s
    }

    /// Block specific domains
    pub fn block_domains() -> Script {
        let mut s = Script::new(
            "Block Domains".to_string(),
            r#"
// Block requests to specific domains
var blockedDomains = ['ads.example.com', 'tracker.example.com'];

function onRequest(request, context) {
    var parts = url.parse(request.url);
    if (blockedDomains.indexOf(parts.host) !== -1) {
        console.log('Blocked request to: ' + parts.host);
        return {
            continue: false,
            response: {
                statusCode: 403,
                body: 'Blocked by Madhyamas'
            }
        };
    }
    return { continue: true };
}
"#
            .to_string(),
        );
        s.hooks = vec!["on_request".to_string()];
        s.description =
            Some("Blocks requests to ads.example.com and tracker.example.com.".to_string());
        s
    }

    /// Modify request headers
    pub fn modify_headers() -> Script {
        let mut s = Script::new(
            "Modify Headers".to_string(),
            r#"
// Add custom headers to requests
function onRequest(request, context) {
    request.headers['X-Madhyamas'] = 'true';
    request.headers['X-Request-ID'] = context.requestId;
    return { continue: true, modified: true };
}
"#
            .to_string(),
        );
        s.hooks = vec!["on_request".to_string()];
        s.description =
            Some("Adds X-Madhyamas and X-Request-ID headers to every request.".to_string());
        s
    }

    /// Mock API responses
    pub fn mock_api() -> Script {
        let mut s = Script::new(
            "Mock API".to_string(),
            r#"
// Mock API responses for testing
function onRequest(request, context) {
    if (request.url.indexOf('/api/user/') !== -1) {
        return {
            continue: false,
            response: {
                statusCode: 200,
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({
                    id: 123,
                    name: 'Mock User',
                    email: 'mock@example.com'
                })
            }
        };
    }
    return { continue: true };
}
"#
            .to_string(),
        );
        s.hooks = vec!["on_request".to_string()];
        s.description = Some("Returns a mock JSON user for /api/user/ paths.".to_string());
        s
    }

    /// Inject artificial latency to simulate slow networks
    pub fn inject_latency() -> Script {
        let mut s = Script::new(
            "Inject Latency".to_string(),
            r#"
// Simulate network latency by recording a start time on the request
// and logging the total elapsed time on the response.  Adjust the
// `minMs` / `maxMs` range to control the simulated delay window.
// Note: this script does not actually sleep — it tags the request
// with a start timestamp so you can measure real upstream latency
// in the console output.
var minMs = 100;
var maxMs = 500;

function onRequest(request, context) {
    context.data.startTime = Date.now();
    return { continue: true };
}

function onResponse(request, response, context) {
    var elapsed = Date.now() - (context.data.startTime || Date.now());
    var tag = elapsed > maxMs ? 'SLOW' : (elapsed < minMs ? 'FAST' : 'OK');
    console.log('[' + tag + '] ' + request.method + ' ' + request.host + request.path + ' — ' + elapsed + 'ms');
    return { continue: true };
}
"#
            .to_string(),
        );
        s.hooks = vec!["on_request".to_string(), "on_response".to_string()];
        s.description = Some(
            "Tags requests with a start time and logs elapsed latency on response (SLOW/OK/FAST)."
                .to_string(),
        );
        s
    }

    /// Rewrite the request URL to redirect to a different backend
    pub fn rewrite_url() -> Script {
        let mut s = Script::new(
            "Rewrite URL".to_string(),
            r#"
// Redirect requests from one host to another (e.g. production → staging).
// Useful for testing against a local or staging backend without changing
// the client application's configuration.
var fromHost = 'api.example.com';
var toHost = 'staging-api.example.com';

function onRequest(request, context) {
    if (request.host === fromHost) {
        var parts = url.parse(request.url);
        parts.host = toHost;
        request.url = url.build(parts);
        request.host = toHost;
        console.log('Rewrote ' + fromHost + ' -> ' + toHost + ' for ' + request.path);
        return { continue: true, modified: true };
    }
    return { continue: true };
}
"#
            .to_string(),
        );
        s.hooks = vec!["on_request".to_string()];
        s.description = Some(
            "Redirects requests from api.example.com to staging-api.example.com using url.parse/build."
                .to_string(),
        );
        s
    }

    /// Inject an authentication token into requests
    pub fn inject_auth_token() -> Script {
        let mut s = Script::new(
            "Inject Auth Token".to_string(),
            r#"
// Add a Bearer token to every request's Authorization header.
// Handy for testing authenticated APIs without embedding the token
// in the client.  Replace the token below with your own.
var token = 'your-test-token-here';

function onRequest(request, context) {
    request.headers['Authorization'] = 'Bearer ' + token;
    console.log('Injected auth token for ' + request.method + ' ' + request.path);
    return { continue: true, modified: true };
}
"#
            .to_string(),
        );
        s.hooks = vec!["on_request".to_string()];
        s.description =
            Some("Adds an Authorization: Bearer header to every request.".to_string());
        s
    }

    /// Modify a JSON response field
    pub fn modify_json_response() -> Script {
        let mut s = Script::new(
            "Modify JSON Response".to_string(),
            r#"
// Patch a field in a JSON response body.  Demonstrates parsing the
// response body, mutating a value, and re-serializing it.  Adjust
// the `fieldName` and `newValue` to suit your API.
var fieldName = 'price';
var newValue = 0.0;

function onResponse(request, response, context) {
    var ct = response.contentType || response.headers['Content-Type'] || '';
    if (ct.indexOf('json') === -1 || !response.body) {
        return { continue: true };
    }
    try {
        var data = JSON.parse(response.body);
        if (data && typeof data === 'object' && fieldName in data) {
            data[fieldName] = newValue;
            response.body = JSON.stringify(data);
            console.log('Set ' + fieldName + ' = ' + newValue + ' on ' + request.path);
            return { continue: true, modified: true };
        }
    } catch (e) {
        console.log('Could not parse JSON body: ' + e);
    }
    return { continue: true };
}
"#
            .to_string(),
        );
        s.hooks = vec!["on_response".to_string()];
        s.description = Some(
            "Patches a field (price) in JSON response bodies to a fixed value (0.0)."
                .to_string(),
        );
        s
    }

    /// Override the response status code
    pub fn override_status_code() -> Script {
        let mut s = Script::new(
            "Override Status Code".to_string(),
            r#"
// Force a specific HTTP status code on responses matching a path
// pattern.  Useful for testing how a client handles error codes
// (404, 500, 503, etc.) without reproducing the real server condition.
var pathPattern = '/api/unstable';
var forcedStatus = 503;

function onResponse(request, response, context) {
    if (request.path.indexOf(pathPattern) !== -1) {
        console.log('Forcing ' + forcedStatus + ' on ' + request.path + ' (was ' + response.statusCode + ')');
        response.statusCode = forcedStatus;
        return { continue: true, modified: true };
    }
    return { continue: true };
}
"#
            .to_string(),
        );
        s.hooks = vec!["on_response".to_string()];
        s.description = Some(
            "Forces a 503 status code on responses from /api/unstable paths for error-handling tests."
                .to_string(),
        );
        s
    }

    /// Add a cache-busting query parameter
    pub fn cache_buster() -> Script {
        let mut s = Script::new(
            "Cache Buster".to_string(),
            r#"
// Append a unique _=timestamp query parameter to every request URL
// to bypass caches (CDN, browser, upstream).  Useful when you need
// to verify that fresh content is being served.
function onRequest(request, context) {
    var parts = url.parse(request.url);
    parts.query['_'] = String(Date.now());
    request.url = url.build(parts);
    // Rebuild path so downstream code sees the updated query string
    var qs = [];
    for (var k in parts.query) {
        qs.push(encodeURIComponent(k) + '=' + encodeURIComponent(parts.query[k]));
    }
    request.path = parts.path + '?' + qs.join('&');
    console.log('Cache-busted: ' + request.url);
    return { continue: true, modified: true };
}
"#
            .to_string(),
        );
        s.hooks = vec!["on_request".to_string()];
        s.description = Some(
            "Appends a unique _=timestamp query parameter to every request to bypass caches."
                .to_string(),
        );
        s
    }

    /// Strip response headers (e.g. security headers) for testing
    pub fn strip_response_headers() -> Script {
        let mut s = Script::new(
            "Strip Response Headers".to_string(),
            r#"
// Remove selected response headers.  Common use cases: stripping
// Strict-Transport-Security / X-Frame-Options / Content-Security-Policy
// to test client behaviour without those protections, or removing
// Set-Cookie to test stateless flows.
var headersToRemove = [
    'Strict-Transport-Security',
    'X-Frame-Options',
    'Content-Security-Policy',
    'Set-Cookie'
];

function onResponse(request, response, context) {
    var removed = [];
    for (var i = 0; i < headersToRemove.length; i++) {
        var h = headersToRemove[i];
        if (response.headers[h]) {
            delete response.headers[h];
            removed.push(h);
        }
    }
    if (removed.length > 0) {
        console.log('Stripped headers from ' + request.host + request.path + ': ' + removed.join(', '));
        return { continue: true, modified: true };
    }
    return { continue: true };
}
"#
            .to_string(),
        );
        s.hooks = vec!["on_response".to_string()];
        s.description = Some(
            "Removes HSTS, X-Frame-Options, CSP, and Set-Cookie headers from responses."
                .to_string(),
        );
        s
    }

    /// Conditional mock based on query parameter
    pub fn conditional_mock() -> Script {
        let mut s = Script::new(
            "Conditional Mock".to_string(),
            r#"
// Mock a response only when a specific query parameter is present.
// This lets you toggle mock mode from the client without changing
// server state.  Example: GET /api/users?mock=true returns a canned
// response; without the flag the request is forwarded normally.
function onRequest(request, context) {
    var parts = url.parse(request.url);
    if (parts.query['mock'] === 'true' && request.path.indexOf('/api/users') !== -1) {
        console.log('Serving mock for ' + request.path);
        return {
            continue: false,
            response: {
                statusCode: 200,
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify([
                    { id: 1, name: 'Alice' },
                    { id: 2, name: 'Bob' }
                ])
            }
        };
    }
    return { continue: true };
}
"#
            .to_string(),
        );
        s.hooks = vec!["on_request".to_string()];
        s.description = Some(
            "Returns a mock user list when the ?mock=true query parameter is present on /api/users."
                .to_string(),
        );
        s
    }
}

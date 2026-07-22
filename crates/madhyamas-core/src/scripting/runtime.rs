//! JavaScript runtime for Madhyamas scripting

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

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
}

/// Script runtime configuration
///
/// **Note:** The `timeout_ms`, `max_memory_bytes`, `allow_network`, and
/// `allow_fs` limits are defined here for future use but are NOT yet
/// enforced. They will be enforced once a JS engine (e.g. `boa_engine`)
/// is integrated with sandboxing support.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptConfig {
    /// Maximum execution time in milliseconds (not yet enforced)
    pub timeout_ms: u64,
    /// Maximum memory usage in bytes (not yet enforced)
    pub max_memory_bytes: usize,
    /// Enable console logging
    pub enable_console: bool,
    /// Allow network access from scripts (not yet enforced)
    pub allow_network: bool,
    /// Allow file system access from scripts (not yet enforced)
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

/// JavaScript runtime manager
#[allow(dead_code)]
pub struct ScriptRuntime {
    /// Registered scripts
    scripts: RwLock<HashMap<String, Script>>,
    /// Execution history
    history: RwLock<Vec<ScriptExecution>>,
    /// Configuration
    config: ScriptConfig,
    /// Maximum history size
    max_history: usize,
}

impl ScriptRuntime {
    pub fn new(config: ScriptConfig) -> Self {
        Self {
            scripts: RwLock::new(HashMap::new()),
            history: RwLock::new(Vec::new()),
            config,
            max_history: 1000,
        }
    }

    /// Register a script
    pub fn register_script(&self, script: Script) -> String {
        let id = script.id.clone();
        self.scripts.write().insert(id.clone(), script);
        id
    }

    /// Remove a script
    pub fn remove_script(&self, id: &str) -> bool {
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

    /// Get scripts for a specific hook
    pub fn get_scripts_for_hook(&self, hook: &str) -> Vec<Script> {
        self.scripts
            .read()
            .values()
            .filter(|s| s.enabled && s.hooks.iter().any(|h| h == hook))
            .cloned()
            .collect()
    }

    /// Toggle a script
    pub fn toggle_script(&self, id: &str, enabled: bool) -> bool {
        if let Some(script) = self.scripts.write().get_mut(id) {
            script.enabled = enabled;
            script.modified_at = chrono::Utc::now();
            true
        } else {
            false
        }
    }

    /// Update script source
    pub fn update_script(&self, id: &str, source: String) -> bool {
        if let Some(script) = self.scripts.write().get_mut(id) {
            script.source = source;
            script.modified_at = chrono::Utc::now();
            true
        } else {
            false
        }
    }

    /// Validate a script's source code.
    ///
    /// Performs basic structural checks (non-empty, balanced braces/parens).
    /// Full syntax validation requires a JS engine and will be added when
    /// `boa_engine` or similar is integrated. Templates require ES6+ (const,
    /// let, template literals).
    pub fn validate(source: &str) -> Result<(), String> {
        let trimmed = source.trim();
        if trimmed.is_empty() {
            return Err("Script source is empty".to_string());
        }
        // Basic brace/paren balance check (not a full parser, but catches
        // obvious typos that would fail at runtime).
        let mut braces = 0i32;
        let mut parens = 0i32;
        let mut brackets = 0i32;
        let mut in_string: Option<char> = None;
        let mut escaped = false;
        for ch in trimmed.chars() {
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

    /// Execute a script with the given context
    ///
    /// Note: This is a placeholder implementation. Full JavaScript execution
    /// would require integrating a JS engine like V8, QuickJS, or Deno Core.
    /// For now, we provide the infrastructure for scripts to be stored and
    /// managed, with execution delegated to an embedded runtime.
    ///
    /// **Important:** No fake execution is recorded in history. The history
    /// only contains real executions (once a JS runtime is integrated).
    pub fn execute(&self, script_id: &str, _context: &super::ScriptContext) -> super::ScriptResult {
        let _script = match self.get_script(script_id) {
            Some(s) => s,
            None => {
                return super::ScriptResult {
                    modified: false,
                    continue_: true,
                    response: None,
                    error: Some(format!("Script not found: {}", script_id)),
                    console: Vec::new(),
                    duration_ms: 0,
                };
            }
        };

        // No JS runtime is integrated — return an error result without
        // recording a fake execution in history. This prevents users from
        // thinking scripts are running when they are not.
        super::ScriptResult {
            modified: false,
            continue_: true,
            response: None,
            error: Some("No JS runtime integrated".to_string()),
            console: Vec::new(),
            duration_ms: 0,
        }
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
            let result = self.execute(&script.id, context);
            results.push(result);
        }

        results
    }

    /// Record an execution in history
    #[allow(dead_code)]
    fn record_execution(&self, execution: ScriptExecution) {
        let mut history = self.history.write();
        history.push(execution);

        if history.len() > self.max_history {
            let excess = history.len() - self.max_history;
            history.drain(0..excess);
        }
    }

    /// Get execution history
    pub fn get_history(&self, limit: Option<usize>) -> Vec<ScriptExecution> {
        let history = self.history.read();
        let limit = limit.unwrap_or(100).min(history.len());
        history.iter().rev().take(limit).cloned().collect()
    }

    /// Clear execution history
    pub fn clear_history(&self) {
        self.history.write().clear();
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

/// Script template examples
pub struct ScriptTemplates;

impl ScriptTemplates {
    /// Log all requests
    pub fn log_requests() -> Script {
        Script::new(
            "Log Requests".to_string(),
            r#"
// Log all incoming requests
function onRequest(request, context) {
    console.log(`${request.method} ${request.url}`);
    console.log(`Headers: ${JSON.stringify(request.headers)}`);
    return { continue: true };
}
"#
            .to_string(),
        )
    }

    /// Add CORS headers to responses
    pub fn add_cors() -> Script {
        Script::new(
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
        )
    }

    /// Block specific domains
    pub fn block_domains() -> Script {
        Script::new(
            "Block Domains".to_string(),
            r#"
// Block requests to specific domains
const blockedDomains = ['ads.example.com', 'tracker.example.com'];

function onRequest(request, context) {
    const url = new URL(request.url);
    if (blockedDomains.includes(url.hostname)) {
        console.log(`Blocked request to: ${url.hostname}`);
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
        )
    }

    /// Modify request headers
    pub fn modify_headers() -> Script {
        Script::new(
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
        )
    }

    /// Mock API responses
    pub fn mock_api() -> Script {
        Script::new(
            "Mock API".to_string(),
            r#"
// Mock API responses for testing
function onRequest(request, context) {
    if (request.url.includes('/api/user/')) {
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
        )
    }
}

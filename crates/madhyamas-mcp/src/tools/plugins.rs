//! Plugin tools.

use reqwest::Client;
use serde_json::{json, Value};

use super::helpers::{api_result, get_id, json_text};
use super::tool_trait::McpTool;
use crate::types::{ContentBlock, McpError};

// ============ Internal helpers (existing free functions, kept as pub(super)) ============

/// List all plugins
pub(super) async fn list_plugins(client: &Client, api_url: &str) -> Result<Value, McpError> {
    let url = format!("{}/api/plugins", api_url);

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| McpError::Http(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
    }

    let plugins: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(plugins)
}

/// Get a specific plugin
pub(super) async fn get_plugin(
    client: &Client,
    api_url: &str,
    plugin_id: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/plugins/{}", api_url, plugin_id);

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| McpError::Http(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
    }

    let plugin: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(plugin)
}

/// Enable a plugin
pub(super) async fn enable_plugin(
    client: &Client,
    api_url: &str,
    plugin_id: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/plugins/{}/enable", api_url, plugin_id);

    let response = client
        .post(&url)
        .send()
        .await
        .map_err(|e| McpError::Http(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
    }

    Ok(json!({
        "success": true,
        "message": format!("Plugin {} enabled", plugin_id)
    }))
}

/// Disable a plugin
pub(super) async fn disable_plugin(
    client: &Client,
    api_url: &str,
    plugin_id: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/plugins/{}/disable", api_url, plugin_id);

    let response = client
        .post(&url)
        .send()
        .await
        .map_err(|e| McpError::Http(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
    }

    Ok(json!({
        "success": true,
        "message": format!("Plugin {} disabled", plugin_id)
    }))
}

/// Get statistics for a plugin
pub(super) async fn get_plugin_stats(
    client: &Client,
    api_url: &str,
    plugin_id: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/plugins/{}/stats", api_url, plugin_id);

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| McpError::Http(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
    }

    let stats: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;

    Ok(stats)
}

/// Reload all plugins
pub(super) async fn reload_plugins(client: &Client, api_url: &str) -> Result<Value, McpError> {
    let url = format!("{}/api/plugins/reload", api_url);

    let response = client
        .post(&url)
        .send()
        .await
        .map_err(|e| McpError::Http(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
    }

    Ok(json!({
        "success": true,
        "message": "Plugins reloaded"
    }))
}

/// Install a plugin from a URL or registry id.
pub(super) async fn install_plugin(
    client: &Client,
    api_url: &str,
    source: &str,
    target: &str,
    checksum: Option<&str>,
) -> Result<Value, McpError> {
    let url = format!("{}/api/plugins/install", api_url);
    let body = match source {
        "registry" => json!({ "source": "registry", "id": target }),
        _ => json!({ "source": "url", "url": target, "checksum": checksum }),
    };

    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| McpError::Http(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
    }

    let result: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;
    Ok(result)
}

/// Uninstall a plugin.
pub(super) async fn uninstall_plugin(
    client: &Client,
    api_url: &str,
    plugin_id: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/plugins/{}/uninstall", api_url, plugin_id);

    let response = client
        .delete(&url)
        .send()
        .await
        .map_err(|e| McpError::Http(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
    }

    Ok(json!({
        "success": true,
        "message": format!("Plugin {} uninstalled", plugin_id)
    }))
}

/// Search the plugin registry.
pub(super) async fn search_registry(
    client: &Client,
    api_url: &str,
    query: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/plugins/registry/search?q={}", api_url, query);

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| McpError::Http(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
    }

    let results: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;
    Ok(results)
}

/// List all registry entries.
pub(super) async fn list_registry(client: &Client, api_url: &str) -> Result<Value, McpError> {
    let url = format!("{}/api/plugins/registry", api_url);

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| McpError::Http(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
    }

    let entries: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;
    Ok(entries)
}

/// Get a plugin's settings schema.
pub(super) async fn get_plugin_schema(
    client: &Client,
    api_url: &str,
    plugin_id: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/plugins/{}/schema", api_url, plugin_id);

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| McpError::Http(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
    }

    let schema: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;
    Ok(schema)
}

/// Get a plugin's current settings.
pub(super) async fn get_plugin_settings(
    client: &Client,
    api_url: &str,
    plugin_id: &str,
) -> Result<Value, McpError> {
    let url = format!("{}/api/plugins/{}/settings", api_url, plugin_id);

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| McpError::Http(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
    }

    let settings: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;
    Ok(settings)
}

/// Update a plugin's settings.
pub(super) async fn update_plugin_settings(
    client: &Client,
    api_url: &str,
    plugin_id: &str,
    settings: Value,
) -> Result<Value, McpError> {
    let url = format!("{}/api/plugins/{}/settings", api_url, plugin_id);

    let response = client
        .put(&url)
        .json(&settings)
        .send()
        .await
        .map_err(|e| McpError::Http(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
    }

    Ok(json!({
        "success": true,
        "message": format!("Settings updated for plugin {}", plugin_id)
    }))
}

/// Get a plugin's recent invocation logs.
pub(super) async fn get_plugin_logs(
    client: &Client,
    api_url: &str,
    plugin_id: &str,
    limit: u32,
) -> Result<Value, McpError> {
    let url = format!("{}/api/plugins/{}/logs?limit={}", api_url, plugin_id, limit);

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| McpError::Http(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(McpError::Http(format!("HTTP {}: {}", status, body)));
    }

    let logs: Value = response
        .json()
        .await
        .map_err(|e| McpError::Parse(e.to_string()))?;
    Ok(logs)
}

// ============ Trait-based tool structs ============

pub struct ListPluginsTool;

#[async_trait::async_trait]
impl McpTool for ListPluginsTool {
    fn name(&self) -> &str {
        "madhyamas_list_plugins"
    }
    fn description(&self) -> &str {
        "List all loaded plugins."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        _arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let result = list_plugins(client, api_url).await?;
        Ok(json_text(&result))
    }
}

pub struct GetPluginTool;

#[async_trait::async_trait]
impl McpTool for GetPluginTool {
    fn name(&self) -> &str {
        "madhyamas_get_plugin"
    }
    fn description(&self) -> &str {
        "Get details of a specific plugin."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the plugin to retrieve"
                }
            },
            "required": ["id"]
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let id = get_id(arguments)?;
        let result = get_plugin(client, api_url, &id).await?;
        Ok(json_text(&result))
    }
}

pub struct EnablePluginTool;

#[async_trait::async_trait]
impl McpTool for EnablePluginTool {
    fn name(&self) -> &str {
        "madhyamas_enable_plugin"
    }
    fn description(&self) -> &str {
        "Enable a plugin."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the plugin to enable"
                }
            },
            "required": ["id"]
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let id = get_id(arguments)?;
        let result = enable_plugin(client, api_url, &id).await?;
        Ok(json_text(&result))
    }
}

pub struct DisablePluginTool;

#[async_trait::async_trait]
impl McpTool for DisablePluginTool {
    fn name(&self) -> &str {
        "madhyamas_disable_plugin"
    }
    fn description(&self) -> &str {
        "Disable a plugin."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the plugin to disable"
                }
            },
            "required": ["id"]
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let id = get_id(arguments)?;
        let result = disable_plugin(client, api_url, &id).await?;
        Ok(json_text(&result))
    }
}

pub struct GetPluginStatsTool;

#[async_trait::async_trait]
impl McpTool for GetPluginStatsTool {
    fn name(&self) -> &str {
        "madhyamas_get_plugin_stats"
    }
    fn description(&self) -> &str {
        "Get runtime statistics for a specific plugin."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the plugin"
                }
            },
            "required": ["id"]
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let id = get_id(arguments)?;
        let result = get_plugin_stats(client, api_url, &id).await?;
        Ok(json_text(&result))
    }
}

pub struct ReloadPluginsTool;

#[async_trait::async_trait]
impl McpTool for ReloadPluginsTool {
    fn name(&self) -> &str {
        "madhyamas_reload_plugins"
    }
    fn description(&self) -> &str {
        "Reload all plugins from disk."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        _arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let result = reload_plugins(client, api_url).await?;
        Ok(json_text(&result))
    }
}

pub struct InstallPluginTool;

#[async_trait::async_trait]
impl McpTool for InstallPluginTool {
    fn name(&self) -> &str {
        "madhyamas_install_plugin"
    }
    fn description(&self) -> &str {
        "Install a plugin from a URL or registry id."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "source": {
                    "type": "string",
                    "description": "Install source: \"url\" or \"registry\"",
                    "default": "url"
                },
                "target": {
                    "type": "string",
                    "description": "Plugin URL (source=url) or registry id (source=registry)"
                },
                "checksum": {
                    "type": "string",
                    "description": "Expected SHA-256 checksum (optional for URL source)"
                }
            },
            "required": ["target"]
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let source = arguments
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("url");
        let target = arguments
            .get("target")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidParams("target is required".to_string()))?;
        let checksum = arguments.get("checksum").and_then(|v| v.as_str());
        let result = install_plugin(client, api_url, source, target, checksum).await?;
        Ok(json_text(&result))
    }
}

pub struct UninstallPluginTool;

#[async_trait::async_trait]
impl McpTool for UninstallPluginTool {
    fn name(&self) -> &str {
        "madhyamas_uninstall_plugin"
    }
    fn description(&self) -> &str {
        "Uninstall a plugin (removes from disk and persistence)."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the plugin to uninstall"
                }
            },
            "required": ["id"]
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let id = get_id(arguments)?;
        let result = uninstall_plugin(client, api_url, &id).await?;
        Ok(json_text(&result))
    }
}

pub struct SearchRegistryTool;

#[async_trait::async_trait]
impl McpTool for SearchRegistryTool {
    fn name(&self) -> &str {
        "madhyamas_search_registry"
    }
    fn description(&self) -> &str {
        "Search the plugin registry by name, description, or tags."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query"
                }
            },
            "required": ["query"]
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let query = arguments
            .get("query")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::InvalidParams("query is required".to_string()))?;
        let result = search_registry(client, api_url, query).await?;
        Ok(json_text(&result))
    }
}

pub struct ListRegistryTool;

#[async_trait::async_trait]
impl McpTool for ListRegistryTool {
    fn name(&self) -> &str {
        "madhyamas_list_registry"
    }
    fn description(&self) -> &str {
        "List all available plugins in the registry."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        _arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let result = list_registry(client, api_url).await?;
        Ok(json_text(&result))
    }
}

pub struct GetPluginSchemaTool;

#[async_trait::async_trait]
impl McpTool for GetPluginSchemaTool {
    fn name(&self) -> &str {
        "madhyamas_get_plugin_schema"
    }
    fn description(&self) -> &str {
        "Get a plugin's settings schema (for UI generation)."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the plugin"
                }
            },
            "required": ["id"]
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let id = get_id(arguments)?;
        let result = get_plugin_schema(client, api_url, &id).await?;
        Ok(json_text(&result))
    }
}

pub struct GetPluginSettingsTool;

#[async_trait::async_trait]
impl McpTool for GetPluginSettingsTool {
    fn name(&self) -> &str {
        "madhyamas_get_plugin_settings"
    }
    fn description(&self) -> &str {
        "Get a plugin's current settings."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the plugin"
                }
            },
            "required": ["id"]
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let id = get_id(arguments)?;
        let result = get_plugin_settings(client, api_url, &id).await?;
        Ok(json_text(&result))
    }
}

pub struct UpdatePluginSettingsTool;

#[async_trait::async_trait]
impl McpTool for UpdatePluginSettingsTool {
    fn name(&self) -> &str {
        "madhyamas_update_plugin_settings"
    }
    fn description(&self) -> &str {
        "Update a plugin's settings."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the plugin"
                },
                "settings": {
                    "type": "object",
                    "description": "Settings as a JSON object"
                }
            },
            "required": ["id", "settings"]
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let id = get_id(arguments)?;
        let settings = arguments
            .get("settings")
            .ok_or_else(|| McpError::InvalidParams("settings is required".to_string()))?
            .clone();
        let result = update_plugin_settings(client, api_url, &id, settings).await?;
        Ok(json_text(&result))
    }
}

pub struct GetPluginLogsTool;

#[async_trait::async_trait]
impl McpTool for GetPluginLogsTool {
    fn name(&self) -> &str {
        "madhyamas_get_plugin_logs"
    }
    fn description(&self) -> &str {
        "Get a plugin's recent invocation logs."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "description": "The ID of the plugin"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of log entries (default 50)"
                }
            },
            "required": ["id"]
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let id = get_id(arguments)?;
        let limit = arguments
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .unwrap_or(50);
        let result = get_plugin_logs(client, api_url, &id, limit).await?;
        Ok(json_text(&result))
    }
}

pub struct GetPluginPanelsTool;

#[async_trait::async_trait]
impl McpTool for GetPluginPanelsTool {
    fn name(&self) -> &str {
        "madhyamas_get_plugin_panels"
    }
    fn description(&self) -> &str {
        "Get a plugin's declarative UI panels (custom UI components \
         defined by the plugin manifest)."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "id": { "type": "string", "description": "Plugin ID" }
            },
            "required": ["id"]
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let id = get_id(arguments)?;
        let resp = client
            .get(format!("{}/api/plugins/{}/panels", api_url, id))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

pub struct GetPluginTemplatesTool;

#[async_trait::async_trait]
impl McpTool for GetPluginTemplatesTool {
    fn name(&self) -> &str {
        "madhyamas_get_plugin_templates"
    }
    fn description(&self) -> &str {
        "List available plugin scaffolding templates (basic, cors, \
         request-logger, domain-blocker, response-modifier)."
    }
    fn input_schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        _arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let resp = client
            .get(format!("{}/api/plugins/templates", api_url))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

pub struct GetRegistryConfigTool;

#[async_trait::async_trait]
impl McpTool for GetRegistryConfigTool {
    fn name(&self) -> &str {
        "madhyamas_get_registry_config"
    }
    fn description(&self) -> &str {
        "Get the current plugin registry configuration (GitHub repo \
         and cache settings)."
    }
    fn input_schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        _arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let resp = client
            .get(format!("{}/api/plugins/registry/config", api_url))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

pub struct UpdateRegistryConfigTool;

#[async_trait::async_trait]
impl McpTool for UpdateRegistryConfigTool {
    fn name(&self) -> &str {
        "madhyamas_update_registry_config"
    }
    fn description(&self) -> &str {
        "Update the plugin registry repository configuration."
    }
    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "repo": { "type": "string", "description": "GitHub repo (owner/repo format)" }
            },
            "required": ["repo"]
        })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let resp = client
            .put(format!("{}/api/plugins/registry/config", api_url))
            .json(arguments)
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

pub struct RefreshRegistryTool;

#[async_trait::async_trait]
impl McpTool for RefreshRegistryTool {
    fn name(&self) -> &str {
        "madhyamas_refresh_registry"
    }
    fn description(&self) -> &str {
        "Force-refresh the plugin registry cache from the configured \
         GitHub repository."
    }
    fn input_schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }
    async fn execute(
        &self,
        client: &Client,
        api_url: &str,
        _arguments: &Value,
    ) -> Result<Vec<ContentBlock>, McpError> {
        let resp = client
            .post(format!("{}/api/plugins/registry/refresh", api_url))
            .json(&json!({}))
            .send()
            .await
            .map_err(|e| McpError::Http(e.to_string()))?;
        api_result(resp).await
    }
}

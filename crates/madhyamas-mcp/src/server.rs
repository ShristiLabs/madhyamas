//! MCP Server implementation for Madhyamas
//!
//! This module implements the Model Context Protocol (MCP) server
//! that exposes Madhyamas capabilities as tools for AI agents.

use std::io::{self, BufRead, Write};

use reqwest::Client;
use serde_json::{json, Value};
use tokio::runtime::Runtime;
use tracing::{debug, error, info};

use crate::tools::{default_registry, enterprise_registry, DynToolRegistry};
use crate::types::*;

/// MCP Server for Madhyamas
pub struct McpServer {
    /// Trait-based tool registry (all tools self-register here).
    dyn_registry: DynToolRegistry,
    tokio_runtime: Runtime,
    /// HTTP client used for resource reads and passed to tool executions.
    http_client: Client,
    /// Madhyamas REST API base URL.
    api_url: String,
    /// Detected API server tier ("enterprise" or "community").
    tier: String,
    /// Configured transport mode.
    transport: McpTransport,
}

impl McpServer {
    /// Create a new MCP server
    pub fn new(config: McpConfig) -> Result<Self, McpError> {
        // Build default auth headers from the configured credentials so
        // every outbound request (tools + resource reads) carries them
        // automatically. In OSS mode / when auth is disabled, no headers
        // are added and the client behaves exactly as before.
        let mut default_headers = reqwest::header::HeaderMap::new();
        for (name, value) in config.auth_headers() {
            if let (Ok(header_name), Ok(header_value)) = (
                reqwest::header::HeaderName::from_bytes(name.as_bytes()),
                reqwest::header::HeaderValue::from_str(&value),
            ) {
                default_headers.insert(header_name, header_value);
            }
        }

        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .default_headers(default_headers)
            .build()
            .map_err(|e| McpError::Http(e.to_string()))?;

        let tokio_runtime = Runtime::new().map_err(|e| McpError::ToolExecution(e.to_string()))?;

        // Detect the API server tier by calling /api/health/detailed.
        // On enterprise servers this returns JSON with a "tier" field;
        // on OSS servers the endpoint may not exist (404) or the call
        // may fail entirely — in both cases we default to "community".
        //
        // The detection runs in a dedicated OS thread with its own
        // tokio runtime so it works even when the caller is already
        // inside a tokio runtime context (e.g. `#[tokio::test]`).
        let tier = Self::detect_tier(&http_client, &config.api_url);
        info!("Detected API tier: {}", tier);

        let mut dyn_registry = default_registry();
        if tier == "enterprise" {
            info!("Registering enterprise MCP tools");
            dyn_registry.merge(enterprise_registry());
        }

        Ok(Self {
            dyn_registry,
            tokio_runtime,
            http_client,
            api_url: config.api_url,
            tier,
            transport: config.transport,
        })
    }

    /// Probe the API server to determine the tier.
    ///
    /// Calls `GET /api/health/detailed` and reads the `tier` field from
    /// the JSON response. Returns `"community"` on any failure (network
    /// error, non-200 status, missing field, parse error) so OSS mode
    /// works without enterprise endpoints.
    fn detect_tier(client: &Client, api_url: &str) -> String {
        let url = format!("{}/api/health/detailed", api_url);
        let client = client.clone();
        let result = std::thread::spawn(move || {
            let runtime = match Runtime::new() {
                Ok(r) => r,
                Err(e) => {
                    debug!("Tier detection: failed to create runtime: {}", e);
                    return None;
                }
            };
            runtime.block_on(async {
                let resp = match client.get(&url).send().await {
                    Ok(r) => r,
                    Err(e) => {
                        debug!("Tier detection request failed: {}", e);
                        return None;
                    }
                };
                if !resp.status().is_success() {
                    debug!("Tier detection non-200 status: {}", resp.status());
                    return None;
                }
                let body: Value = match resp.json().await {
                    Ok(v) => v,
                    Err(e) => {
                        debug!("Tier detection parse error: {}", e);
                        return None;
                    }
                };
                body.get("tier")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
        })
        .join()
        .unwrap_or(None);
        result.unwrap_or_else(|| "community".to_string())
    }

    /// Returns the detected API server tier.
    pub fn tier(&self) -> &str {
        &self.tier
    }

    /// Run the MCP server using stdio transport
    pub fn run(&self) -> Result<(), McpError> {
        info!("Starting Madhyamas MCP server...");
        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut stdout = stdout.lock();

        for line in stdin.lock().lines() {
            let line = line.map_err(|e| McpError::JsonRpc(e.to_string()))?;
            debug!("Received: {}", line);

            let request: JsonRpcRequest = match serde_json::from_str(&line) {
                Ok(req) => req,
                Err(e) => {
                    error!("Failed to parse request: {}", e);
                    let response = Self::error_response(
                        Value::Null,
                        -32700,
                        "Parse error",
                        Some(json!({ "details": e.to_string() })),
                    );
                    self.write_response(&mut stdout, &response)?;
                    continue;
                }
            };

            let response = self.handle_request(request);
            self.write_response(&mut stdout, &response)?;
        }

        Ok(())
    }

    /// Returns the configured transport mode.
    pub fn transport(&self) -> McpTransport {
        self.transport.clone()
    }

    /// Run the MCP server using HTTP transport on the given port.
    ///
    /// Accepts JSON-RPC POST requests on the root endpoint (`/`) and
    /// returns JSON-RPC responses. The server runs until the process is
    /// terminated.
    ///
    /// This method consumes `self` so that the internal tokio runtime
    /// can be dropped safely in a dedicated thread (dropping a runtime
    /// inside an async context panics). The axum server itself runs in
    /// the same dedicated thread with its own runtime.
    pub fn run_http(self, port: u16) -> Result<(), McpError> {
        info!(
            "Starting Madhyamas MCP server (HTTP transport, port {})",
            port
        );
        // Build a fresh registry for the HTTP transport (the stdio
        // registry is borrowed by &self and can't be moved into an
        // owned axum state). This is deterministic — same tier, same
        // tools.
        let mut registry = default_registry();
        if self.tier == "enterprise" {
            registry.merge(enterprise_registry());
        }
        let state = HttpMcpState {
            registry: std::sync::Arc::new(registry),
            http_client: self.http_client.clone(),
            api_url: self.api_url.clone(),
        };
        let app = axum::Router::new()
            .route("/", axum::routing::post(handle_http_request))
            .route("/mcp", axum::routing::post(handle_http_request))
            .with_state(state);
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));

        // Move the inner runtime into a dedicated thread so it can be
        // dropped without panicking ("Cannot drop a runtime in a
        // context where blocking is not allowed"). The axum server
        // runs in the same thread with a fresh runtime.
        let result = std::thread::spawn(move || {
            // Drop the stdio runtime first — it's not needed for HTTP.
            drop(self.tokio_runtime);

            let runtime = match Runtime::new() {
                Ok(r) => r,
                Err(e) => return Err(McpError::ToolExecution(e.to_string())),
            };
            runtime.block_on(async {
                let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
                    McpError::Http(format!("Failed to bind MCP HTTP port {}: {}", port, e))
                })?;
                info!("MCP HTTP server listening on http://{}", addr);
                axum::serve(listener, app)
                    .await
                    .map_err(|e| McpError::Http(format!("HTTP server error: {}", e)))
            })
        })
        .join()
        .map_err(|e| McpError::ToolExecution(format!("HTTP server thread panicked: {:?}", e)))?;

        result.map_err(|e| {
            error!("MCP HTTP server error: {}", e);
            e
        })?;
        Ok(())
    }

    /// Handle an incoming JSON-RPC request
    fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        debug!("Handling method: {}", request.method);

        // Handle notifications (no response expected)
        if request.id.is_null() {
            debug!("Received notification: {}", request.method);
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: None,
                result: None,
                error: None,
            };
        }

        match request.method.as_str() {
            "initialize" => self.handle_initialize(request),
            "tools/list" => self.handle_list_tools(request),
            "tools/call" => self.handle_call_tool(request),
            "resources/list" => self.handle_list_resources(request),
            "resources/templates/list" => self.handle_list_resource_templates(request),
            "resources/read" => self.handle_read_resource(request),
            "prompts/list" => self.handle_list_prompts(request),
            "prompts/get" => self.handle_get_prompt(request),
            _ => Self::error_response(
                request.id,
                -32601,
                "Method not found",
                Some(json!({ "method": request.method })),
            ),
        }
    }

    /// Handle initialize request
    fn handle_initialize(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let result = InitializeResult {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(false),
                }),
                resources: Some(ResourcesCapability {
                    subscribe: Some(false),
                    list_changed: Some(false),
                }),
                prompts: Some(PromptsCapability {
                    list_changed: Some(false),
                }),
            },
            server_info: ServerInfo {
                name: "madhyamas".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };

        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: Some(request.id),
            result: Some(serde_json::to_value(result).unwrap_or_default()),
            error: None,
        }
    }

    /// Handle tools/list request
    fn handle_list_tools(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let tools = self.dyn_registry.list_tools();
        let result = ListToolsResult { tools };

        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: Some(request.id),
            result: Some(serde_json::to_value(result).unwrap_or_default()),
            error: None,
        }
    }

    /// Handle tools/call request
    fn handle_call_tool(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let params = match request.params {
            Some(p) => p,
            None => {
                return Self::error_response(
                    request.id,
                    -32602,
                    "Invalid params: missing parameters",
                    None,
                );
            }
        };

        let tool_name = match params.get("name").and_then(|v| v.as_str()) {
            Some(name) => name.to_string(),
            None => {
                return Self::error_response(
                    request.id,
                    -32602,
                    "Invalid params: missing tool name",
                    None,
                );
            }
        };

        let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

        debug!("Calling tool: {} with args: {:?}", tool_name, arguments);

        let result = self.tokio_runtime.block_on(async {
            self.dyn_registry
                .execute(&tool_name, &self.http_client, &self.api_url, &arguments)
                .await
        });

        let result = match result {
            Some(r) => r,
            None => Err(McpError::NotFound(format!("Unknown tool: {}", tool_name))),
        };

        match result {
            Ok(content) => {
                let tool_result = ToolResult {
                    content,
                    is_error: Some(false),
                };
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: Some(request.id),
                    result: Some(serde_json::to_value(tool_result).unwrap_or_default()),
                    error: None,
                }
            }
            Err(e) => {
                let tool_result = ToolResult {
                    content: vec![ContentBlock::Text {
                        text: e.to_string(),
                    }],
                    is_error: Some(true),
                };
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: Some(request.id),
                    result: Some(serde_json::to_value(tool_result).unwrap_or_default()),
                    error: None,
                }
            }
        }
    }

    /// Handle resources/list request
    fn handle_list_resources(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let resources = vec![
            Resource {
                uri: "madhyamas://traffic".to_string(),
                name: "Traffic".to_string(),
                description: Some("Captured HTTP traffic".to_string()),
                mime_type: Some("application/json".to_string()),
            },
            Resource {
                uri: "madhyamas://sessions".to_string(),
                name: "Sessions".to_string(),
                description: Some("Debugging sessions".to_string()),
                mime_type: Some("application/json".to_string()),
            },
            Resource {
                uri: "madhyamas://config".to_string(),
                name: "Configuration".to_string(),
                description: Some("Madhyamas configuration".to_string()),
                mime_type: Some("application/json".to_string()),
            },
        ];

        let result = ListResourcesResult { resources };

        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: Some(request.id),
            result: Some(serde_json::to_value(result).unwrap_or_default()),
            error: None,
        }
    }

    /// Handle resources/templates/list request
    fn handle_list_resource_templates(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let templates = vec![
            ResourceTemplate {
                uri_template: "madhyamas://session/{id}".to_string(),
                name: "Session Details".to_string(),
                description: Some("Details of a specific debugging session".to_string()),
                mime_type: Some("application/json".to_string()),
            },
            ResourceTemplate {
                uri_template: "madhyamas://traffic/{id}".to_string(),
                name: "Traffic Entry Details".to_string(),
                description: Some("Details of a specific captured traffic entry".to_string()),
                mime_type: Some("application/json".to_string()),
            },
            ResourceTemplate {
                uri_template: "madhyamas://mock/{id}".to_string(),
                name: "Mock Rule Details".to_string(),
                description: Some("Details of a specific mock rule".to_string()),
                mime_type: Some("application/json".to_string()),
            },
        ];

        let result = ListResourceTemplatesResult {
            resource_templates: templates,
        };

        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: Some(request.id),
            result: Some(serde_json::to_value(result).unwrap_or_default()),
            error: None,
        }
    }

    /// Handle resources/read request
    fn handle_read_resource(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let params = match request.params {
            Some(p) => p,
            None => {
                return Self::error_response(
                    request.id,
                    -32602,
                    "Invalid params: missing parameters",
                    None,
                );
            }
        };

        let uri = match params.get("uri").and_then(|v| v.as_str()) {
            Some(u) => u,
            None => {
                return Self::error_response(
                    request.id,
                    -32602,
                    "Invalid params: missing uri",
                    None,
                );
            }
        };

        let client = &self.http_client;
        let api_url = &self.api_url;

        let contents = self.tokio_runtime.block_on(async {
            // Dynamic resource URIs: madhyamas://session/{id},
            // madhyamas://traffic/{id}, madhyamas://mock/{id}
            if let Some(id) = uri.strip_prefix("madhyamas://session/") {
                let id = crate::tools::sanitize_id(id)?;
                let resp = client
                    .get(format!("{}/api/sessions/{}", api_url, id))
                    .send()
                    .await
                    .map_err(|e| McpError::Http(e.to_string()))?;
                let session: Value = resp
                    .json()
                    .await
                    .map_err(|e| McpError::Parse(e.to_string()))?;
                return Ok(vec![ResourceContents {
                    uri: uri.to_string(),
                    mime_type: Some("application/json".to_string()),
                    text: serde_json::to_string_pretty(&session)
                        .unwrap_or_else(|_| "{}".to_string()),
                }]);
            }
            if let Some(id) = uri.strip_prefix("madhyamas://traffic/") {
                let id = crate::tools::sanitize_id(id)?;
                let resp = client
                    .get(format!("{}/api/traffic/{}", api_url, id))
                    .send()
                    .await
                    .map_err(|e| McpError::Http(e.to_string()))?;
                let entry: Value = resp
                    .json()
                    .await
                    .map_err(|e| McpError::Parse(e.to_string()))?;
                return Ok(vec![ResourceContents {
                    uri: uri.to_string(),
                    mime_type: Some("application/json".to_string()),
                    text: serde_json::to_string_pretty(&entry).unwrap_or_else(|_| "{}".to_string()),
                }]);
            }
            if let Some(id) = uri.strip_prefix("madhyamas://mock/") {
                let id = crate::tools::sanitize_id(id)?;
                let resp = client
                    .get(format!("{}/api/mocks/{}", api_url, id))
                    .send()
                    .await
                    .map_err(|e| McpError::Http(e.to_string()))?;
                let mock: Value = resp
                    .json()
                    .await
                    .map_err(|e| McpError::Parse(e.to_string()))?;
                return Ok(vec![ResourceContents {
                    uri: uri.to_string(),
                    mime_type: Some("application/json".to_string()),
                    text: serde_json::to_string_pretty(&mock).unwrap_or_else(|_| "{}".to_string()),
                }]);
            }
            match uri {
                "madhyamas://traffic" => {
                    let resp = client
                        .get(format!("{}/api/traffic", api_url))
                        .send()
                        .await
                        .map_err(|e| McpError::Http(e.to_string()))?;
                    let traffic: Value = resp
                        .json()
                        .await
                        .map_err(|e| McpError::Parse(e.to_string()))?;
                    Ok(vec![ResourceContents {
                        uri: uri.to_string(),
                        mime_type: Some("application/json".to_string()),
                        text: serde_json::to_string_pretty(&traffic)
                            .unwrap_or_else(|_| "[]".to_string()),
                    }])
                }
                "madhyamas://sessions" => {
                    let resp = client
                        .get(format!("{}/api/sessions", api_url))
                        .send()
                        .await
                        .map_err(|e| McpError::Http(e.to_string()))?;
                    let sessions: Value = resp
                        .json()
                        .await
                        .map_err(|e| McpError::Parse(e.to_string()))?;
                    Ok(vec![ResourceContents {
                        uri: uri.to_string(),
                        mime_type: Some("application/json".to_string()),
                        text: serde_json::to_string_pretty(&sessions)
                            .unwrap_or_else(|_| "[]".to_string()),
                    }])
                }
                "madhyamas://config" => {
                    let resp = client
                        .get(format!("{}/api/config", api_url))
                        .send()
                        .await
                        .map_err(|e| McpError::Http(e.to_string()))?;
                    let config: Value = resp
                        .json()
                        .await
                        .map_err(|e| McpError::Parse(e.to_string()))?;
                    Ok(vec![ResourceContents {
                        uri: uri.to_string(),
                        mime_type: Some("application/json".to_string()),
                        text: serde_json::to_string_pretty(&config)
                            .unwrap_or_else(|_| "{}".to_string()),
                    }])
                }
                _ => Err(McpError::NotFound(format!("Unknown resource: {}", uri))),
            }
        });

        match contents {
            Ok(contents) => {
                let result = ReadResourceResult { contents };
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: Some(request.id),
                    result: Some(serde_json::to_value(result).unwrap_or_default()),
                    error: None,
                }
            }
            Err(e) => Self::error_response(
                request.id,
                -32603,
                "Internal error",
                Some(json!({ "details": e.to_string() })),
            ),
        }
    }

    /// Handle prompts/list request
    fn handle_list_prompts(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let prompts = vec![
            Prompt {
                name: "debug-4xx".to_string(),
                description: "Analyze recent 4xx responses and suggest fixes".to_string(),
                arguments: vec![],
            },
            Prompt {
                name: "debug-5xx".to_string(),
                description: "Analyze recent 5xx responses and identify root causes".to_string(),
                arguments: vec![],
            },
            Prompt {
                name: "find-auth-issues".to_string(),
                description: "Check for authentication-related issues in recent traffic"
                    .to_string(),
                arguments: vec![],
            },
            Prompt {
                name: "mock-missing-endpoint".to_string(),
                description: "Create a mock for a missing endpoint found in 404 responses"
                    .to_string(),
                arguments: vec![],
            },
            Prompt {
                name: "compare-staging-prod".to_string(),
                description: "Compare traffic between two sessions".to_string(),
                arguments: vec![
                    PromptArgument {
                        name: "session1".to_string(),
                        description: Some("First session ID".to_string()),
                        required: Some(true),
                    },
                    PromptArgument {
                        name: "session2".to_string(),
                        description: Some("Second session ID".to_string()),
                        required: Some(true),
                    },
                ],
            },
            Prompt {
                name: "audit-trail".to_string(),
                description: "Show audit trail for a specific user or time period".to_string(),
                arguments: vec![PromptArgument {
                    name: "user_id".to_string(),
                    description: Some("Filter by user ID".to_string()),
                    required: Some(false),
                }],
            },
        ];

        let result = ListPromptsResult { prompts };

        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: Some(request.id),
            result: Some(serde_json::to_value(result).unwrap_or_default()),
            error: None,
        }
    }

    /// Handle prompts/get request
    fn handle_get_prompt(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        let params = match request.params {
            Some(p) => p,
            None => {
                return Self::error_response(
                    request.id,
                    -32602,
                    "Invalid params: missing parameters",
                    None,
                );
            }
        };

        let name = match params.get("name").and_then(|v| v.as_str()) {
            Some(n) => n,
            None => {
                return Self::error_response(
                    request.id,
                    -32602,
                    "Invalid params: missing prompt name",
                    None,
                );
            }
        };

        let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
        let client = &self.http_client;
        let api_url = &self.api_url;

        let result = self.tokio_runtime.block_on(async {
            match name {
                "debug-4xx" => {
                    let resp = client
                        .get(format!("{}/api/traffic?status=4", api_url))
                        .send()
                        .await;
                    let context = match resp {
                        Ok(r) if r.status().is_success() => {
                            r.text().await.unwrap_or_else(|_| "[]".to_string())
                        }
                        _ => "[]".to_string(),
                    };
                    let text = format!(
                        "Analyze the following recent 4xx HTTP responses captured by Madhyamas \
                         and suggest fixes for each. Focus on the status code, URL, and any \
                         error patterns.\n\nCaptured 4xx responses:\n{}",
                        context
                    );
                    Ok(GetPromptResult {
                        description: Some("4xx response analysis".to_string()),
                        messages: vec![PromptMessage {
                            role: "user".to_string(),
                            content: ContentBlock::Text { text },
                        }],
                    })
                }
                "debug-5xx" => {
                    let resp = client
                        .get(format!("{}/api/traffic?status=5", api_url))
                        .send()
                        .await;
                    let context = match resp {
                        Ok(r) if r.status().is_success() => {
                            r.text().await.unwrap_or_else(|_| "[]".to_string())
                        }
                        _ => "[]".to_string(),
                    };
                    let text = format!(
                        "Analyze the following recent 5xx HTTP responses captured by Madhyamas \
                         and identify root causes. Look for backend errors, timeouts, and \
                         infrastructure issues.\n\nCaptured 5xx responses:\n{}",
                        context
                    );
                    Ok(GetPromptResult {
                        description: Some("5xx response analysis".to_string()),
                        messages: vec![PromptMessage {
                            role: "user".to_string(),
                            content: ContentBlock::Text { text },
                        }],
                    })
                }
                "find-auth-issues" => {
                    let resp = client
                        .get(format!("{}/api/traffic?status=401", api_url))
                        .send()
                        .await;
                    let context = match resp {
                        Ok(r) if r.status().is_success() => {
                            r.text().await.unwrap_or_else(|_| "[]".to_string())
                        }
                        _ => "[]".to_string(),
                    };
                    let text = format!(
                        "Check for authentication-related issues in the following recent \
                         traffic. Look for 401/403 responses, missing or expired tokens, and \
                         incorrect auth headers.\n\nAuth-related traffic:\n{}",
                        context
                    );
                    Ok(GetPromptResult {
                        description: Some("Authentication issue analysis".to_string()),
                        messages: vec![PromptMessage {
                            role: "user".to_string(),
                            content: ContentBlock::Text { text },
                        }],
                    })
                }
                "mock-missing-endpoint" => {
                    let resp = client
                        .get(format!("{}/api/traffic?status=404", api_url))
                        .send()
                        .await;
                    let context = match resp {
                        Ok(r) if r.status().is_success() => {
                            r.text().await.unwrap_or_else(|_| "[]".to_string())
                        }
                        _ => "[]".to_string(),
                    };
                    let text = format!(
                        "Create mock responses for the missing endpoints found in the following \
                         404 responses. For each unique URL path, suggest a mock response with \
                         a realistic status code and body.\n\n404 responses:\n{}",
                        context
                    );
                    Ok(GetPromptResult {
                        description: Some("Mock missing endpoints".to_string()),
                        messages: vec![PromptMessage {
                            role: "user".to_string(),
                            content: ContentBlock::Text { text },
                        }],
                    })
                }
                "compare-staging-prod" => {
                    let s1 = arguments
                        .get("session1")
                        .and_then(|v| v.as_str())
                        .unwrap_or("session-1");
                    let s2 = arguments
                        .get("session2")
                        .and_then(|v| v.as_str())
                        .unwrap_or("session-2");
                    let text = format!(
                        "Compare the traffic between two debugging sessions:\n\
                         - Session 1: {}\n\
                         - Session 2: {}\n\n\
                         Use the madhyamas_get_traffic and madhyamas_list_sessions tools to \
                         fetch traffic from both sessions, then compare response times, status \
                         codes, and any differences in the API responses.",
                        s1, s2
                    );
                    Ok(GetPromptResult {
                        description: Some("Compare two sessions".to_string()),
                        messages: vec![PromptMessage {
                            role: "user".to_string(),
                            content: ContentBlock::Text { text },
                        }],
                    })
                }
                "audit-trail" => {
                    let user_id = arguments.get("user_id").and_then(|v| v.as_str());
                    let text = if let Some(uid) = user_id {
                        format!(
                            "Show the audit trail for user '{}'. Use the \
                             madhyamas_get_audit_events tool with user_id='{}' to fetch the \
                             events, then summarize the user's actions, timestamps, and any \
                             suspicious activity.",
                            uid, uid
                        )
                    } else {
                        "Show the recent audit trail. Use the madhyamas_get_audit_events tool \
                         to fetch recent events, then summarize the actions, users involved, \
                         and any suspicious activity."
                            .to_string()
                    };
                    Ok(GetPromptResult {
                        description: Some("Audit trail analysis".to_string()),
                        messages: vec![PromptMessage {
                            role: "user".to_string(),
                            content: ContentBlock::Text { text },
                        }],
                    })
                }
                _ => Err(McpError::NotFound(format!("Unknown prompt: {}", name))),
            }
        });

        match result {
            Ok(prompt_result) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: Some(request.id),
                result: Some(serde_json::to_value(prompt_result).unwrap_or_default()),
                error: None,
            },
            Err(e) => Self::error_response(
                request.id,
                -32603,
                "Internal error",
                Some(json!({ "details": e.to_string() })),
            ),
        }
    }

    /// Create an error response
    fn error_response(id: Value, code: i64, message: &str, data: Option<Value>) -> JsonRpcResponse {
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: if id.is_null() { None } else { Some(id) },
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.to_string(),
                data,
            }),
        }
    }

    /// Write response to stdout
    fn write_response(
        &self,
        stdout: &mut io::StdoutLock,
        response: &JsonRpcResponse,
    ) -> Result<(), McpError> {
        let json = serde_json::to_string(response).map_err(|e| McpError::JsonRpc(e.to_string()))?;
        debug!("Sending: {}", json);
        stdout
            .write_all(json.as_bytes())
            .map_err(|e| McpError::JsonRpc(e.to_string()))?;
        stdout
            .write_all(b"\n")
            .map_err(|e| McpError::JsonRpc(e.to_string()))?;
        stdout
            .flush()
            .map_err(|e| McpError::JsonRpc(e.to_string()))?;
        Ok(())
    }
}

// ============ HTTP Transport ============

/// Owned state for the HTTP transport's axum router.
///
/// Unlike the stdio transport (which borrows from `McpServer`), the HTTP
/// transport needs an owned, `Clone`-able state because axum requires
/// `S: Clone + Send + Sync + 'static`. The tool registry is wrapped in
/// `Arc` so clones share the same tool instances.
#[derive(Clone)]
struct HttpMcpState {
    registry: std::sync::Arc<DynToolRegistry>,
    http_client: Client,
    api_url: String,
}

/// Axum handler for MCP HTTP transport: accepts a JSON-RPC request body
/// (single object or batch array) and returns the JSON-RPC response.
async fn handle_http_request(
    axum::extract::State(state): axum::extract::State<HttpMcpState>,
    axum::Json(body): axum::Json<Value>,
) -> axum::Json<Value> {
    // Support both single requests and batch arrays.
    if let Some(arr) = body.as_array() {
        let mut responses = Vec::new();
        for req_value in arr {
            if let Ok(request) = serde_json::from_value::<JsonRpcRequest>(req_value.clone()) {
                if request.id.is_null() {
                    continue;
                }
                let resp = handle_http_request_async(request, &state).await;
                responses.push(serde_json::to_value(resp).unwrap_or_default());
            } else {
                let resp = McpServer::error_response(
                    Value::Null,
                    -32700,
                    "Parse error",
                    Some(json!({ "details": "invalid JSON-RPC request" })),
                );
                responses.push(serde_json::to_value(resp).unwrap_or_default());
            }
        }
        return axum::Json(Value::Array(responses));
    }
    match serde_json::from_value::<JsonRpcRequest>(body.clone()) {
        Ok(request) => {
            if request.id.is_null() {
                return axum::Json(Value::Null);
            }
            let response = handle_http_request_async(request, &state).await;
            axum::Json(serde_json::to_value(response).unwrap_or_default())
        }
        Err(e) => {
            let response = McpServer::error_response(
                Value::Null,
                -32700,
                "Parse error",
                Some(json!({ "details": e.to_string() })),
            );
            axum::Json(serde_json::to_value(response).unwrap_or_default())
        }
    }
}

/// Handle a JSON-RPC request in an async context (HTTP transport).
///
/// This mirrors [`McpServer::handle_request`] but uses `.await` directly
/// instead of `block_on` since the HTTP handler already runs inside a
/// tokio runtime.
async fn handle_http_request_async(
    request: JsonRpcRequest,
    state: &HttpMcpState,
) -> JsonRpcResponse {
    debug!("HTTP handling method: {}", request.method);

    match request.method.as_str() {
        "initialize" => {
            let result = InitializeResult {
                protocol_version: "2024-11-05".to_string(),
                capabilities: ServerCapabilities {
                    tools: Some(ToolsCapability {
                        list_changed: Some(false),
                    }),
                    resources: Some(ResourcesCapability {
                        subscribe: Some(false),
                        list_changed: Some(false),
                    }),
                    prompts: Some(PromptsCapability {
                        list_changed: Some(false),
                    }),
                },
                server_info: ServerInfo {
                    name: "madhyamas".to_string(),
                    version: env!("CARGO_PKG_VERSION").to_string(),
                },
            };
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: Some(request.id),
                result: Some(serde_json::to_value(result).unwrap_or_default()),
                error: None,
            }
        }
        "tools/list" => {
            let tools = state.registry.list_tools();
            let result = ListToolsResult { tools };
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: Some(request.id),
                result: Some(serde_json::to_value(result).unwrap_or_default()),
                error: None,
            }
        }
        "tools/call" => {
            let params = match request.params {
                Some(p) => p,
                None => {
                    return McpServer::error_response(
                        request.id,
                        -32602,
                        "Invalid params: missing parameters",
                        None,
                    );
                }
            };
            let tool_name = match params.get("name").and_then(|v| v.as_str()) {
                Some(name) => name.to_string(),
                None => {
                    return McpServer::error_response(
                        request.id,
                        -32602,
                        "Invalid params: missing tool name",
                        None,
                    );
                }
            };
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
            let result = state
                .registry
                .execute(&tool_name, &state.http_client, &state.api_url, &arguments)
                .await;
            let result = match result {
                Some(r) => r,
                None => Err(McpError::NotFound(format!("Unknown tool: {}", tool_name))),
            };
            match result {
                Ok(content) => {
                    let tool_result = ToolResult {
                        content,
                        is_error: Some(false),
                    };
                    JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: Some(request.id),
                        result: Some(serde_json::to_value(tool_result).unwrap_or_default()),
                        error: None,
                    }
                }
                Err(e) => {
                    let tool_result = ToolResult {
                        content: vec![ContentBlock::Text {
                            text: e.to_string(),
                        }],
                        is_error: Some(true),
                    };
                    JsonRpcResponse {
                        jsonrpc: "2.0".to_string(),
                        id: Some(request.id),
                        result: Some(serde_json::to_value(tool_result).unwrap_or_default()),
                        error: None,
                    }
                }
            }
        }
        "resources/list" => {
            let resources = vec![
                Resource {
                    uri: "madhyamas://traffic".to_string(),
                    name: "Traffic".to_string(),
                    description: Some("Captured HTTP traffic".to_string()),
                    mime_type: Some("application/json".to_string()),
                },
                Resource {
                    uri: "madhyamas://sessions".to_string(),
                    name: "Sessions".to_string(),
                    description: Some("Debugging sessions".to_string()),
                    mime_type: Some("application/json".to_string()),
                },
                Resource {
                    uri: "madhyamas://config".to_string(),
                    name: "Configuration".to_string(),
                    description: Some("Madhyamas configuration".to_string()),
                    mime_type: Some("application/json".to_string()),
                },
            ];
            let result = ListResourcesResult { resources };
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: Some(request.id),
                result: Some(serde_json::to_value(result).unwrap_or_default()),
                error: None,
            }
        }
        "resources/templates/list" => {
            let templates = vec![
                ResourceTemplate {
                    uri_template: "madhyamas://session/{id}".to_string(),
                    name: "Session by ID".to_string(),
                    description: Some("Get details of a specific debugging session".to_string()),
                    mime_type: Some("application/json".to_string()),
                },
                ResourceTemplate {
                    uri_template: "madhyamas://traffic/{id}".to_string(),
                    name: "Traffic entry by ID".to_string(),
                    description: Some("Get details of a specific traffic entry".to_string()),
                    mime_type: Some("application/json".to_string()),
                },
                ResourceTemplate {
                    uri_template: "madhyamas://mock/{id}".to_string(),
                    name: "Mock rule by ID".to_string(),
                    description: Some("Get details of a specific mock rule".to_string()),
                    mime_type: Some("application/json".to_string()),
                },
            ];
            let result = ListResourceTemplatesResult {
                resource_templates: templates,
            };
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: Some(request.id),
                result: Some(serde_json::to_value(result).unwrap_or_default()),
                error: None,
            }
        }
        "prompts/list" => {
            let prompts = vec![
                Prompt {
                    name: "debug-4xx".to_string(),
                    description: "Analyze recent 4xx responses and suggest fixes".to_string(),
                    arguments: vec![],
                },
                Prompt {
                    name: "debug-5xx".to_string(),
                    description: "Analyze recent 5xx responses and identify root causes"
                        .to_string(),
                    arguments: vec![],
                },
                Prompt {
                    name: "find-auth-issues".to_string(),
                    description: "Check for authentication-related issues in recent traffic"
                        .to_string(),
                    arguments: vec![],
                },
                Prompt {
                    name: "mock-missing-endpoint".to_string(),
                    description: "Create a mock for a missing endpoint found in 404 responses"
                        .to_string(),
                    arguments: vec![],
                },
                Prompt {
                    name: "compare-staging-prod".to_string(),
                    description: "Compare traffic between two sessions".to_string(),
                    arguments: vec![
                        PromptArgument {
                            name: "session1".to_string(),
                            description: Some("First session ID".to_string()),
                            required: Some(true),
                        },
                        PromptArgument {
                            name: "session2".to_string(),
                            description: Some("Second session ID".to_string()),
                            required: Some(true),
                        },
                    ],
                },
                Prompt {
                    name: "audit-trail".to_string(),
                    description: "Show audit trail for a specific user or time period".to_string(),
                    arguments: vec![PromptArgument {
                        name: "user_id".to_string(),
                        description: Some("Filter by user ID".to_string()),
                        required: Some(false),
                    }],
                },
            ];
            let result = ListPromptsResult { prompts };
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id: Some(request.id),
                result: Some(serde_json::to_value(result).unwrap_or_default()),
                error: None,
            }
        }
        _ => McpServer::error_response(
            request.id,
            -32601,
            "Method not found",
            Some(json!({ "method": request.method })),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spawn a minimal mock HTTP server that captures the raw request
    /// text from every connection and returns a JSON `[]` body. Returns
    /// the bound address and an mpsc receiver of raw request texts.
    /// Handles multiple connections (tier detection + test request).
    async fn spawn_mock_server() -> (String, tokio::sync::mpsc::Receiver<String>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = tokio::sync::mpsc::channel::<String>(16);
        let url = format!("http://{}", addr);

        tokio::spawn(async move {
            loop {
                let (mut socket, _) = match listener.accept().await {
                    Ok(s) => s,
                    Err(_) => break,
                };
                let tx = tx.clone();
                tokio::spawn(async move {
                    use tokio::io::{AsyncReadExt, AsyncWriteExt};
                    let mut buf = vec![0u8; 8192];
                    let n = socket.read(&mut buf).await.unwrap_or(0);
                    let request_text = String::from_utf8_lossy(&buf[..n]).to_string();
                    let _ = tx.try_send(request_text);
                    let body =
                        b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\r\n[]";
                    let _ = socket.write_all(body).await;
                    let _ = socket.flush().await;
                });
            }
        });

        (url, rx)
    }

    /// Collect all request texts received by the mock server, waiting
    /// briefly for requests to arrive.
    async fn collect_requests(rx: &mut tokio::sync::mpsc::Receiver<String>) -> Vec<String> {
        let mut requests = Vec::new();
        let deadline = tokio::time::Instant::now() + tokio::time::Duration::from_millis(500);
        loop {
            let Ok(msg) = tokio::time::timeout_at(deadline, rx.recv()).await else {
                break;
            };
            if let Some(req) = msg {
                requests.push(req);
            } else {
                break;
            }
        }
        requests
    }

    #[tokio::test]
    async fn test_mcp_server_sends_api_key_header() {
        let (url, mut rx) = spawn_mock_server().await;
        let config = McpConfig {
            api_url: url,
            timeout_secs: 5,
            auth: McpAuth::ApiKey("test-api-key-abc".to_string()),
            transport: McpTransport::Stdio,
        };
        let server = McpServer::new(config).unwrap();
        // Use the server's internal client (private field, accessible within
        // this module) to make a request — this verifies that McpServer::new
        // applied the auth headers as default headers on the client.
        let _ = server.http_client.get(&server.api_url).send().await;
        // McpServer owns a tokio Runtime; dropping it inside an async context
        // panics, so forget it to avoid the drop-time panic in tests.
        std::mem::forget(server);
        let requests = collect_requests(&mut rx).await;
        let found = requests.iter().any(|r| {
            r.to_ascii_lowercase()
                .contains("x-api-key: test-api-key-abc")
        });
        assert!(
            found,
            "no request contained X-API-Key header; requests: {:?}",
            requests
        );
    }

    #[tokio::test]
    async fn test_mcp_server_sends_jwt_header() {
        let (url, mut rx) = spawn_mock_server().await;
        let config = McpConfig {
            api_url: url,
            timeout_secs: 5,
            auth: McpAuth::Jwt("my-jwt-token".to_string()),
            transport: McpTransport::Stdio,
        };
        let server = McpServer::new(config).unwrap();
        let _ = server.http_client.get(&server.api_url).send().await;
        std::mem::forget(server);
        let requests = collect_requests(&mut rx).await;
        let found = requests.iter().any(|r| {
            r.to_ascii_lowercase()
                .contains("authorization: bearer my-jwt-token")
        });
        assert!(
            found,
            "no request contained Authorization header; requests: {:?}",
            requests
        );
    }

    #[tokio::test]
    async fn test_mcp_server_without_auth_sends_no_auth_headers() {
        let (url, mut rx) = spawn_mock_server().await;
        let config = McpConfig {
            api_url: url,
            timeout_secs: 5,
            auth: McpAuth::None,
            transport: McpTransport::Stdio,
        };
        let server = McpServer::new(config).unwrap();
        let _ = server.http_client.get(&server.api_url).send().await;
        std::mem::forget(server);
        let requests = collect_requests(&mut rx).await;
        for request in &requests {
            let lower = request.to_ascii_lowercase();
            assert!(
                !lower.contains("x-api-key"),
                "unexpected X-API-Key header: {}",
                request
            );
            assert!(
                !lower.contains("authorization:"),
                "unexpected Authorization header: {}",
                request
            );
        }
    }

    #[tokio::test]
    async fn test_tier_detection_defaults_to_community() {
        let (url, _rx) = spawn_mock_server().await;
        let config = McpConfig {
            api_url: url,
            timeout_secs: 5,
            auth: McpAuth::None,
            transport: McpTransport::Stdio,
        };
        let server = McpServer::new(config).unwrap();
        assert_eq!(server.tier(), "community");
        std::mem::forget(server);
    }

    #[tokio::test]
    async fn test_tier_detection_unreachable_server() {
        // Bind to a port then immediately drop the listener so nothing is
        // listening — the tier detection should gracefully default to
        // "community".
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let url = format!("http://127.0.0.1:{}", port);
        let config = McpConfig {
            api_url: url,
            timeout_secs: 2,
            auth: McpAuth::None,
            transport: McpTransport::Stdio,
        };
        let server = McpServer::new(config).unwrap();
        assert_eq!(server.tier(), "community");
        std::mem::forget(server);
    }
}

//! MCP Server implementation for Madhyamas
//!
//! This module implements the Model Context Protocol (MCP) server
//! that exposes Madhyamas capabilities as tools for AI agents.

use std::io::{self, BufRead, Write};

use reqwest::Client;
use serde_json::{json, Value};
use tokio::runtime::Runtime;
use tracing::{debug, error, info};

use crate::tools::{default_registry, DynToolRegistry};
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
}

impl McpServer {
    /// Create a new MCP server
    pub fn new(config: McpConfig) -> Result<Self, McpError> {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| McpError::Http(e.to_string()))?;

        let tokio_runtime = Runtime::new().map_err(|e| McpError::ToolExecution(e.to_string()))?;

        Ok(Self {
            dyn_registry: default_registry(),
            tokio_runtime,
            http_client,
            api_url: config.api_url,
        })
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
            "resources/read" => self.handle_read_resource(request),
            "prompts/list" => self.handle_list_prompts(request),
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
        // No prompts supported yet
        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: Some(request.id),
            result: Some(json!({ "prompts": [] })),
            error: None,
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

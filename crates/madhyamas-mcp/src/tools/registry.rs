//! Tool registry

use crate::types::Tool;
use serde_json::json;

/// Tool registry that defines all available MCP tools
pub struct ToolRegistry {
    tools: Vec<Tool>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    /// Create a new tool registry with all Madhyamas tools
    pub fn new() -> Self {
        let tools = vec![
            // Traffic inspection tools
            Tool {
                name: "madhyamas_get_traffic".to_string(),
                description: "List captured HTTP traffic with advanced filtering. Returns a summary of requests including method, URL, status code, and timing.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "filter": {
                            "type": "string",
                            "description": "Filter expression to match URLs (supports wildcards)"
                        },
                        "method": {
                            "type": "string",
                            "enum": ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"],
                            "description": "Filter by HTTP method"
                        },
                        "status": {
                            "type": "integer",
                            "description": "Filter by HTTP status code (e.g., 200, 404, 500)"
                        },
                        "file_type": {
                            "type": "string",
                            "description": "Filter by file type/extension (e.g., json, html, css, js, png)"
                        },
                        "header": {
                            "type": "string",
                            "description": "Filter by header (format: 'key:value' or just 'key')"
                        },
                        "cookie": {
                            "type": "string",
                            "description": "Filter by cookie (format: 'name=value' or just 'name')"
                        },
                        "search": {
                            "type": "string",
                            "description": "Search in request/response bodies"
                        },
                        "min_size": {
                            "type": "integer",
                            "description": "Filter by minimum response size in bytes"
                        },
                        "max_size": {
                            "type": "integer",
                            "description": "Filter by maximum response size in bytes"
                        },
                        "min_time": {
                            "type": "integer",
                            "description": "Filter by minimum response time in milliseconds"
                        },
                        "max_time": {
                            "type": "integer",
                            "description": "Filter by maximum response time in milliseconds"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of results to return (default: 100)"
                        },
                        "offset": {
                            "type": "integer",
                            "description": "Offset for pagination"
                        }
                    }
                }),
            },
            Tool {
                name: "madhyamas_get_traffic_entry".to_string(),
                description: "Get detailed information about a specific traffic entry, including full request/response headers and bodies.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The ID of the traffic entry to retrieve"
                        }
                    },
                    "required": ["id"]
                }),
            },
            Tool {
                name: "madhyamas_search_traffic".to_string(),
                description: "Search captured traffic by content (headers, bodies, URLs). Useful for finding specific API calls or patterns.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query string"
                        }
                    },
                    "required": ["query"]
                }),
            },
            Tool {
                name: "madhyamas_get_traffic_count".to_string(),
                description: "Get the total count of captured traffic entries.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "madhyamas_clear_traffic".to_string(),
                description: "Clear all captured traffic. This action cannot be undone.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },

            // Mock tools
            Tool {
                name: "madhyamas_list_mocks".to_string(),
                description: "List all mock rules currently configured.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "madhyamas_create_mock".to_string(),
                description: "Create a mock rule to intercept and replace responses. Useful for testing error handling, edge cases, or offline development.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "url_pattern": {
                            "type": "string",
                            "description": "URL pattern to match (supports wildcards and regex)"
                        },
                        "method": {
                            "type": "string",
                            "enum": ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"],
                            "description": "HTTP method to match"
                        },
                        "status_code": {
                            "type": "integer",
                            "description": "HTTP status code to return (default: 200)"
                        },
                        "headers": {
                            "type": "object",
                            "description": "Response headers to return"
                        },
                        "body": {
                            "description": "Response body to return"
                        },
                        "delay_ms": {
                            "type": "integer",
                            "description": "Optional delay before responding (for testing slow connections)"
                        },
                        "enabled": {
                            "type": "boolean",
                            "description": "Whether the mock is enabled immediately (default: true)"
                        }
                    },
                    "required": ["url_pattern"]
                }),
            },
            Tool {
                name: "madhyamas_delete_mock".to_string(),
                description: "Delete a mock rule.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The ID of the mock rule to delete"
                        }
                    },
                    "required": ["id"]
                }),
            },
            Tool {
                name: "madhyamas_toggle_mock".to_string(),
                description: "Enable or disable a mock rule.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The ID of the mock rule to toggle"
                        },
                        "enabled": {
                            "type": "boolean",
                            "description": "true to enable, false to disable"
                        }
                    },
                    "required": ["id", "enabled"]
                }),
            },

            // Breakpoint tools
            Tool {
                name: "madhyamas_list_breakpoints".to_string(),
                description: "List all breakpoint rules currently configured.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "madhyamas_create_breakpoint".to_string(),
                description: "Create a breakpoint rule to pause traffic matching a pattern. Paused traffic can be inspected and modified before proceeding.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "url_pattern": {
                            "type": "string",
                            "description": "URL pattern to match"
                        },
                        "method": {
                            "type": "string",
                            "enum": ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"],
                            "description": "HTTP method to match"
                        },
                        "direction": {
                            "type": "string",
                            "enum": ["request", "response", "both"],
                            "description": "Which direction to intercept (default: request)"
                        },
                        "enabled": {
                            "type": "boolean",
                            "description": "Whether breakpoint is enabled immediately (default: true)"
                        }
                    },
                    "required": ["url_pattern"]
                }),
            },
            Tool {
                name: "madhyamas_delete_breakpoint".to_string(),
                description: "Delete a breakpoint rule.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The ID of the breakpoint rule to delete"
                        }
                    },
                    "required": ["id"]
                }),
            },

            // Replay tools
            Tool {
                name: "madhyamas_replay_request".to_string(),
                description: "Replay a captured request. Optionally modify headers/body before replaying. Useful for debugging or testing different scenarios.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The ID of the traffic entry to replay"
                        },
                        "modifications": {
                            "type": "object",
                            "properties": {
                                "headers": {
                                    "type": "object",
                                    "description": "Headers to add/modify"
                                },
                                "body": {
                                    "description": "New request body"
                                },
                                "url": {
                                    "type": "string",
                                    "description": "Override URL"
                                }
                            },
                            "description": "Optional modifications to apply before replaying"
                        }
                    },
                    "required": ["id"]
                }),
            },
            Tool {
                name: "madhyamas_save_request".to_string(),
                description: "Save a request for later replay. Useful for creating a library of test requests.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "traffic_id": {
                            "type": "string",
                            "description": "The ID of the traffic entry to save"
                        },
                        "name": {
                            "type": "string",
                            "description": "Optional name for the saved request"
                        }
                    },
                    "required": ["traffic_id"]
                }),
            },
            Tool {
                name: "madhyamas_list_saved_requests".to_string(),
                description: "List all saved requests available for replay.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },

            // Session tools
            Tool {
                name: "madhyamas_list_sessions".to_string(),
                description: "List all debugging sessions.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "madhyamas_create_session".to_string(),
                description: "Create a new debugging session. Sessions help organize captured traffic.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Name for the session"
                        },
                        "description": {
                            "type": "string",
                            "description": "Description of the session"
                        }
                    }
                }),
            },
            Tool {
                name: "madhyamas_export_session".to_string(),
                description: "Export a session in HAR or cURL format for sharing or backup.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The ID of the session to export"
                        },
                        "format": {
                            "type": "string",
                            "enum": ["har", "curl"],
                            "description": "Export format (default: har)"
                        }
                    },
                    "required": ["id"]
                }),
            },
            Tool {
                name: "madhyamas_import_session".to_string(),
                description: "Import a session from HAR format or previously exported data.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "session_data": {
                            "type": "object",
                            "description": "The session data to import (HAR format or Madhyamas export)"
                        }
                    },
                    "required": ["session_data"]
                }),
            },
            Tool {
                name: "madhyamas_switch_session".to_string(),
                description: "Switch the active debugging session.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The ID of the session to switch to"
                        }
                    },
                    "required": ["id"]
                }),
            },

            // Export tools
            Tool {
                name: "madhyamas_export_curl".to_string(),
                description: "Export a specific request as a cURL command. Useful for reproducing API calls in a terminal.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The ID of the traffic entry to export as cURL"
                        }
                    },
                    "required": ["id"]
                }),
            },

            // Configuration tools
            Tool {
                name: "madhyamas_get_config".to_string(),
                description: "Get current Madhyamas configuration including proxy port, API port, host, HTTPS interception status, and max requests.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "madhyamas_update_config".to_string(),
                description: "Update runtime Madhyamas configuration. Only specified fields will be updated.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "intercept_https": {
                            "type": "boolean",
                            "description": "Enable or disable HTTPS interception"
                        },
                        "max_requests": {
                            "type": "integer",
                            "description": "Maximum number of requests to keep in memory"
                        },
                        "verbose": {
                            "type": "boolean",
                            "description": "Enable or disable verbose logging"
                        },
                        "public_ip": {
                            "type": ["string", "null"],
                            "description": "Public IP address to display (null to auto-detect)"
                        }
                    }
                }),
            },

            // Capture mode tools
            Tool {
                name: "madhyamas_get_capture_status".to_string(),
                description: "Get current capture mode status. Returns whether traffic is being recorded (recording mode) or just forwarded without recording (passthrough mode).".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "madhyamas_toggle_capture".to_string(),
                description: "Toggle capture mode between recording and passthrough. In passthrough mode, the proxy forwards traffic but does not record it to the database.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        ];

        Self { tools }
    }

    /// List all available tools
    pub fn list_tools(&self) -> Vec<Tool> {
        self.tools.clone()
    }

    /// Get a tool by name
    pub fn get_tool(&self, name: &str) -> Option<&Tool> {
        self.tools.iter().find(|t| t.name == name)
    }
}

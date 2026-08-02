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
            Tool {
                name: "madhyamas_import_har".to_string(),
                description: "Import traffic from a HAR (HTTP Archive) JSON document into a new session. Each log.entries[] entry is converted into a traffic entry. Invalid entries are skipped. Useful for loading traffic captured by other tools (Chrome DevTools, Charles, Fiddler).".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "har": {
                            "type": "object",
                            "description": "The full HAR JSON document (must contain a 'log' object with an 'entries' array)"
                        },
                        "session_name": {
                            "type": "string",
                            "description": "Optional name for the newly created session (default: 'Imported HAR')"
                        },
                        "switch_session": {
                            "type": "boolean",
                            "description": "Switch the active session to the newly created one after import (default: false)"
                        }
                    },
                    "required": ["har"]
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
            Tool {
                name: "madhyamas_create_advanced_mock".to_string(),
                description: "Create an advanced mock rule with full configuration including response sequences, conditional responses, or probabilistic responses. Use this for complex mocking scenarios.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Name for the mock rule"
                        },
                        "condition": {
                            "type": "object",
                            "description": "Match condition (e.g., {\"type\": \"url_pattern\", \"pattern\": \"https://api.example.com/.*\"})"
                        },
                        "response_config": {
                            "type": "object",
                            "description": "Response configuration. Can be: Single {\"type\": \"single\", \"response\": {...}}, Sequence {\"type\": \"sequence\", \"responses\": [...]}, Conditional {\"type\": \"conditional\", \"conditions\": [...]}, or Probabilistic {\"type\": \"probabilistic\", \"responses\": [...]}"
                        },
                        "description": {
                            "type": "string",
                            "description": "Optional description/documentation"
                        },
                        "tags": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Tags for organization"
                        },
                        "collection_id": {
                            "type": "string",
                            "description": "Collection to add this mock to"
                        },
                        "enabled": {
                            "type": "boolean",
                            "description": "Whether the mock is enabled (default: true)"
                        },
                        "priority": {
                            "type": "integer",
                            "description": "Priority (lower = higher priority, default: 100)"
                        }
                    },
                    "required": ["name", "condition", "response_config"]
                }),
            },
            Tool {
                name: "madhyamas_update_mock".to_string(),
                description: "Update an existing mock rule with new configuration.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The ID of the mock rule to update"
                        },
                        "mock": {
                            "type": "object",
                            "description": "The full mock rule object to update"
                        }
                    },
                    "required": ["id", "mock"]
                }),
            },
            Tool {
                name: "madhyamas_get_mock".to_string(),
                description: "Get details of a specific mock rule.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The ID of the mock rule to retrieve"
                        }
                    },
                    "required": ["id"]
                }),
            },
            Tool {
                name: "madhyamas_duplicate_mock".to_string(),
                description: "Duplicate an existing mock rule with a new name.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The ID of the mock rule to duplicate"
                        },
                        "new_name": {
                            "type": "string",
                            "description": "Optional new name for the duplicate"
                        }
                    },
                    "required": ["id"]
                }),
            },
            Tool {
                name: "madhyamas_rollback_mock".to_string(),
                description: "Rollback a mock rule to a previous version.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The ID of the mock rule to rollback"
                        },
                        "version": {
                            "type": "integer",
                            "description": "The version number to rollback to"
                        }
                    },
                    "required": ["id", "version"]
                }),
            },
            Tool {
                name: "madhyamas_get_mock_versions".to_string(),
                description: "Get version history for a mock rule.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The ID of the mock rule"
                        }
                    },
                    "required": ["id"]
                }),
            },
            // Mock Collections
            Tool {
                name: "madhyamas_list_mock_collections".to_string(),
                description: "List all mock collections. Collections help organize related mock rules.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "madhyamas_create_mock_collection".to_string(),
                description: "Create a new mock collection for organizing related mock rules.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Name for the collection"
                        },
                        "description": {
                            "type": "string",
                            "description": "Optional description"
                        },
                        "tags": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Tags for the collection"
                        }
                    },
                    "required": ["name"]
                }),
            },
            Tool {
                name: "madhyamas_delete_mock_collection".to_string(),
                description: "Delete a mock collection. Optionally delete all rules in the collection.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The ID of the collection to delete"
                        },
                        "delete_rules": {
                            "type": "boolean",
                            "description": "Whether to also delete all rules in this collection (default: false)"
                        }
                    },
                    "required": ["id"]
                }),
            },
            Tool {
                name: "madhyamas_toggle_mock_collection".to_string(),
                description: "Enable or disable all mock rules in a collection.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The ID of the collection to toggle"
                        },
                        "enabled": {
                            "type": "boolean",
                            "description": "true to enable all, false to disable all"
                        }
                    },
                    "required": ["id", "enabled"]
                }),
            },
            // Mock Analytics
            Tool {
                name: "madhyamas_get_mock_analytics".to_string(),
                description: "Get hit analytics for all mock rules, including hit counts and history.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "madhyamas_get_mock_hit_history".to_string(),
                description: "Get detailed hit history for a specific mock rule.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The ID of the mock rule"
                        }
                    },
                    "required": ["id"]
                }),
            },
            // Mock Testing & Preview
            Tool {
                name: "madhyamas_test_mock".to_string(),
                description: "Test a mock rule against a sample request to see if it matches and what response would be returned.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The ID of the mock rule to test"
                        },
                        "request": {
                            "type": "object",
                            "description": "Sample request data with url, method, headers, body"
                        }
                    },
                    "required": ["id", "request"]
                }),
            },
            Tool {
                name: "madhyamas_preview_mock_match".to_string(),
                description: "Preview which mock rule would match a given request without actually intercepting traffic.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "request": {
                            "type": "object",
                            "description": "Request data to test against all mocks"
                        }
                    },
                    "required": ["request"]
                }),
            },
            // Mock Import/Export
            Tool {
                name: "madhyamas_export_mocks".to_string(),
                description: "Export all mock rules as JSON for backup or sharing.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "madhyamas_import_mocks".to_string(),
                description: "Import mock rules from HAR, OpenAPI, or Postman format.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "format": {
                            "type": "string",
                            "enum": ["har", "openapi", "postman"],
                            "description": "Import format"
                        },
                        "data": {
                            "type": "string",
                            "description": "The data to import (JSON string)"
                        }
                    },
                    "required": ["format", "data"]
                }),
            },
            // Mock Recording
            Tool {
                name: "madhyamas_set_mock_recording".to_string(),
                description: "Enable or disable mock recording mode. When enabled, responses are captured as potential mock rules.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "enabled": {
                            "type": "boolean",
                            "description": "true to enable recording, false to disable"
                        }
                    },
                    "required": ["enabled"]
                }),
            },
            Tool {
                name: "madhyamas_get_mock_recording_status".to_string(),
                description: "Get current mock recording status.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "madhyamas_get_recorded_mocks".to_string(),
                description: "Get all mock rules that have been recorded from live traffic.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "madhyamas_promote_recorded_mocks".to_string(),
                description: "Promote all recorded mocks to active mock rules.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
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
                description: "Replay a saved request with optional edit-then-repeat. Supports modifying the URL, method, headers, body, and redirect behavior before replaying. Useful for debugging, testing different scenarios, or re-running requests with modified payloads.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The ID of the saved request to replay"
                        },
                        "modifications": {
                            "type": "object",
                            "properties": {
                                "url": {
                                    "type": "string",
                                    "description": "Override the request URL"
                                },
                                "method": {
                                    "type": "string",
                                    "description": "Override the HTTP method (GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS)"
                                },
                                "headers": {
                                    "type": "object",
                                    "description": "Headers to add or replace (key-value pairs)"
                                },
                                "remove_headers": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "Header names to remove from the request"
                                },
                                "body": {
                                    "type": "string",
                                    "description": "New request body (raw text)"
                                },
                                "follow_redirects": {
                                    "type": "boolean",
                                    "description": "Whether to follow 3xx redirect responses (default: false)"
                                }
                            },
                            "description": "Optional modifications to apply before replaying (edit-then-repeat)"
                        }
                    },
                    "required": ["id"]
                }),
            },
            Tool {
                name: "madhyamas_replay_advanced".to_string(),
                description: "Replay a saved request multiple times with concurrency, iterations, and inter-request delay (batch/advanced replay). Returns aggregate statistics including success/failure counts and latency percentiles (min/avg/max/p95). Useful for basic load testing and performance benchmarking. Safety limits: iterations capped at 10,000 and concurrency at 100.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The ID of the saved request to replay"
                        },
                        "iterations": {
                            "type": "integer",
                            "description": "Total number of requests to send (max 10,000, default: 1)",
                            "minimum": 1
                        },
                        "concurrency": {
                            "type": "integer",
                            "description": "Number of simultaneous in-flight requests (max 100, default: 1)",
                            "minimum": 1
                        },
                        "delay_ms": {
                            "type": "integer",
                            "description": "Optional delay between requests in milliseconds",
                            "minimum": 0
                        },
                        "modifications": {
                            "type": "object",
                            "properties": {
                                "url": {
                                    "type": "string",
                                    "description": "Override the request URL"
                                },
                                "method": {
                                    "type": "string",
                                    "description": "Override the HTTP method (GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS)"
                                },
                                "headers": {
                                    "type": "object",
                                    "description": "Headers to add or replace (key-value pairs)"
                                },
                                "remove_headers": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "Header names to remove from the request"
                                },
                                "body": {
                                    "type": "string",
                                    "description": "New request body (raw text)"
                                },
                                "follow_redirects": {
                                    "type": "boolean",
                                    "description": "Whether to follow 3xx redirect responses (default: false)"
                                }
                            },
                            "description": "Optional modifications to apply before replaying (applied to all iterations)"
                        }
                    },
                    "required": ["id", "iterations", "concurrency"]
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

            // Throttle tools
            Tool {
                name: "madhyamas_get_throttle".to_string(),
                description: "Get the current network throttle profile, including download/upload bandwidth limits, latency, jitter, and packet loss.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "madhyamas_set_throttle".to_string(),
                description: "Set a custom network throttle profile to simulate slow or unreliable network conditions. Optionally enable/disable throttling at the same time.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "download_bps": {
                            "type": "integer",
                            "description": "Download bandwidth in bytes per second (0 = unlimited)"
                        },
                        "upload_bps": {
                            "type": "integer",
                            "description": "Upload bandwidth in bytes per second (0 = unlimited)"
                        },
                        "delay_ms": {
                            "type": "integer",
                            "description": "Latency in milliseconds"
                        },
                        "jitter_ms": {
                            "type": "integer",
                            "description": "Jitter in milliseconds"
                        },
                        "packet_loss_percent": {
                            "type": "integer",
                            "description": "Packet loss percentage (0-100)"
                        },
                        "name": {
                            "type": "string",
                            "description": "Profile name"
                        },
                        "enabled": {
                            "type": "boolean",
                            "description": "Whether to enable throttling immediately"
                        }
                    }
                }),
            },
            Tool {
                name: "madhyamas_toggle_throttle".to_string(),
                description: "Enable or disable network throttling without changing the active profile.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "enabled": {
                            "type": "boolean",
                            "description": "true to enable throttling, false to disable"
                        }
                    },
                    "required": ["enabled"]
                }),
            },
            Tool {
                name: "madhyamas_get_throttle_presets".to_string(),
                description: "List available predefined throttle profiles (e.g., GPRS, EDGE, 3G, 4G LTE) for quick network simulation.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },

            // Rewrite tools
            Tool {
                name: "madhyamas_list_rewrites".to_string(),
                description: "List all URL/header rewrite rules currently configured.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "madhyamas_create_rewrite".to_string(),
                description: "Create a rewrite rule to modify URLs, headers, or bodies of matching requests/responses.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Name for the rewrite rule"
                        },
                        "condition": {
                            "type": "object",
                            "description": "Match condition (e.g., {\"type\": \"url_pattern\", \"pattern\": \"https://api.example.com/.*\"})"
                        },
                        "direction": {
                            "type": "string",
                            "enum": ["request", "response", "both"],
                            "description": "Which direction to apply rewrites (default: request)"
                        },
                        "rewrites": {
                            "type": "array",
                            "items": {"type": "object"},
                            "description": "List of rewrite actions to apply (e.g., {\"type\": \"set_header\", \"name\": \"X-Custom\", \"value\": \"test\"})"
                        },
                        "enabled": {
                            "type": "boolean",
                            "description": "Whether the rule is enabled (default: true)"
                        },
                        "priority": {
                            "type": "integer",
                            "description": "Priority (lower = higher priority, default: 100)"
                        }
                    },
                    "required": ["name", "condition", "direction", "rewrites"]
                }),
            },
            Tool {
                name: "madhyamas_delete_rewrite".to_string(),
                description: "Delete a rewrite rule.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The ID of the rewrite rule to delete"
                        }
                    },
                    "required": ["id"]
                }),
            },
            Tool {
                name: "madhyamas_toggle_rewrite".to_string(),
                description: "Enable or disable a rewrite rule.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The ID of the rewrite rule to toggle"
                        },
                        "enabled": {
                            "type": "boolean",
                            "description": "true to enable, false to disable"
                        }
                    },
                    "required": ["id", "enabled"]
                }),
            },
            Tool {
                name: "madhyamas_get_rewrite_templates".to_string(),
                description: "Get predefined rewrite rule templates for common scenarios.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },

            // gRPC tools
            Tool {
                name: "madhyamas_get_grpc_connections".to_string(),
                description: "List all captured gRPC connections.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "madhyamas_get_grpc_streams".to_string(),
                description: "List all gRPC streams observed by the proxy.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "madhyamas_get_grpc_frames".to_string(),
                description: "Get captured gRPC frames, optionally filtered.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "filter": {
                            "type": "string",
                            "description": "Optional filter expression for frames"
                        }
                    }
                }),
            },
            Tool {
                name: "madhyamas_get_grpc_stats".to_string(),
                description: "Get aggregated gRPC traffic statistics.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "madhyamas_clear_grpc".to_string(),
                description: "Clear all captured gRPC frames and reset statistics.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },

            // Script tools
            Tool {
                name: "madhyamas_list_scripts".to_string(),
                description: "List all registered scripts.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "madhyamas_create_script".to_string(),
                description: "Create a new script that runs on specified request/response hooks.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Name for the script"
                        },
                        "source": {
                            "type": "string",
                            "description": "The script source code"
                        },
                        "hook": {
                            "type": "string",
                            "description": "Hook to attach the script to (e.g., on_request, on_response)"
                        },
                        "enabled": {
                            "type": "boolean",
                            "description": "Whether the script is enabled immediately (default: true)"
                        }
                    },
                    "required": ["name", "source"]
                }),
            },
            Tool {
                name: "madhyamas_get_script".to_string(),
                description: "Get details of a specific script.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The ID of the script to retrieve"
                        }
                    },
                    "required": ["id"]
                }),
            },
            Tool {
                name: "madhyamas_update_script".to_string(),
                description: "Update an existing script with new source/configuration.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The ID of the script to update"
                        },
                        "script": {
                            "type": "object",
                            "description": "The full script object to update"
                        }
                    },
                    "required": ["id", "script"]
                }),
            },
            Tool {
                name: "madhyamas_delete_script".to_string(),
                description: "Delete a script.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The ID of the script to delete"
                        }
                    },
                    "required": ["id"]
                }),
            },
            Tool {
                name: "madhyamas_toggle_script".to_string(),
                description: "Enable or disable a script.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The ID of the script to toggle"
                        },
                        "enabled": {
                            "type": "boolean",
                            "description": "true to enable, false to disable"
                        }
                    },
                    "required": ["id", "enabled"]
                }),
            },
            Tool {
                name: "madhyamas_get_script_templates".to_string(),
                description: "Get predefined script templates for common scenarios.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "madhyamas_test_script".to_string(),
                description: "Test (dry-run) a script against a sample request/response context without affecting live traffic or recording history.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "source": {
                            "type": "string",
                            "description": "The script source code to test"
                        },
                        "hook": {
                            "type": "string",
                            "description": "Hook to test against (e.g. on_request, on_response)",
                            "enum": ["on_request", "on_response", "on_websocket_message", "on_grpc_message", "on_traffic_store", "on_session_start", "on_session_end"]
                        }
                    },
                    "required": ["source", "hook"]
                }),
            },
            Tool {
                name: "madhyamas_validate_script".to_string(),
                description: "Validate a script's syntax without executing it. Returns whether the source is valid and any parse errors.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "source": {
                            "type": "string",
                            "description": "The script source code to validate"
                        }
                    },
                    "required": ["source"]
                }),
            },
            Tool {
                name: "madhyamas_get_script_history".to_string(),
                description: "Get execution history for a specific script, showing recent runs with success/failure status, duration, console output, and errors.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The ID of the script to get history for"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of history entries to return (default: 50)"
                        }
                    },
                    "required": ["id"]
                }),
            },

            // Plugin tools
            Tool {
                name: "madhyamas_list_plugins".to_string(),
                description: "List all loaded plugins.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
            Tool {
                name: "madhyamas_get_plugin".to_string(),
                description: "Get details of a specific plugin.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The ID of the plugin to retrieve"
                        }
                    },
                    "required": ["id"]
                }),
            },
            Tool {
                name: "madhyamas_enable_plugin".to_string(),
                description: "Enable a plugin.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The ID of the plugin to enable"
                        }
                    },
                    "required": ["id"]
                }),
            },
            Tool {
                name: "madhyamas_disable_plugin".to_string(),
                description: "Disable a plugin.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The ID of the plugin to disable"
                        }
                    },
                    "required": ["id"]
                }),
            },
            Tool {
                name: "madhyamas_get_plugin_stats".to_string(),
                description: "Get runtime statistics for a specific plugin.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "string",
                            "description": "The ID of the plugin"
                        }
                    },
                    "required": ["id"]
                }),
            },
            Tool {
                name: "madhyamas_reload_plugins".to_string(),
                description: "Reload all plugins from disk.".to_string(),
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

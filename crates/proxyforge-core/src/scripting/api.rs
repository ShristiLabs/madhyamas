//! Script API - functions available to scripts

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// API available to scripts
pub struct ScriptApi;

impl ScriptApi {
    /// Get API documentation
    pub fn documentation() -> String {
        r#"
# ProxyForge Script API

## Request Object
```javascript
request = {
    method: string,       // HTTP method (GET, POST, etc.)
    url: string,          // Full URL
    host: string,         // Hostname
    path: string,         // Path with query string
    headers: object,      // Request headers
    body: string | null,  // Request body (if text)
    contentType: string,  // Content-Type header value
    query: object         // Parsed query parameters
}
```

## Response Object
```javascript
response = {
    statusCode: number,   // HTTP status code
    statusMessage: string,// Status message
    headers: object,      // Response headers
    body: string | null,  // Response body (if text)
    contentType: string,  // Content-Type header value
    durationMs: number    // Response time in milliseconds
}
```

## Hook Functions

### onRequest(request, context)
Called before a request is forwarded to the server.

Returns:
```javascript
{
    continue: boolean,    // false to stop and return response
    modified: boolean,    // true if request was modified
    response: {           // only if continue is false
        statusCode: number,
        headers: object,
        body: string
    }
}
```

### onResponse(request, response, context)
Called after a response is received from the server.

Returns:
```javascript
{
    continue: boolean,
    modified: boolean
}
```

## Utility Functions

### console.log(message)
Log a message to the console.

### context.data
Shared data storage between hooks for the same request.

### context.requestId
Unique ID for the current request.

### context.sessionId
Current session ID.

## Examples

### Add Custom Header
```javascript
function onRequest(request, context) {
    request.headers['X-Custom-Header'] = 'value';
    return { continue: true, modified: true };
}
```

### Block Request
```javascript
function onRequest(request, context) {
    if (request.url.includes('ads.')) {
        return {
            continue: false,
            response: {
                statusCode: 403,
                headers: {},
                body: 'Blocked'
            }
        };
    }
    return { continue: true };
}
```

### Modify Response
```javascript
function onResponse(request, response, context) {
    if (response.headers['Content-Type']?.includes('json')) {
        const body = JSON.parse(response.body);
        body.modified = true;
        response.body = JSON.stringify(body);
        return { continue: true, modified: true };
    }
    return { continue: true };
}
```

### Pass Data Between Hooks
```javascript
function onRequest(request, context) {
    context.data.startTime = Date.now();
    return { continue: true };
}

function onResponse(request, response, context) {
    const elapsed = Date.now() - context.data.startTime;
    console.log(`Request took ${elapsed}ms`);
    return { continue: true };
}
```
"#
        .to_string()
    }

    /// Get built-in functions available to scripts
    pub fn builtin_functions() -> Vec<BuiltinFunction> {
        vec![
            BuiltinFunction {
                name: "console.log".to_string(),
                description: "Log a message to the console".to_string(),
                signature: "console.log(message: string): void".to_string(),
            },
            BuiltinFunction {
                name: "JSON.parse".to_string(),
                description: "Parse a JSON string".to_string(),
                signature: "JSON.parse(text: string): any".to_string(),
            },
            BuiltinFunction {
                name: "JSON.stringify".to_string(),
                description: "Convert a value to JSON string".to_string(),
                signature: "JSON.stringify(value: any): string".to_string(),
            },
            BuiltinFunction {
                name: "base64.encode".to_string(),
                description: "Encode a string to base64".to_string(),
                signature: "base64.encode(text: string): string".to_string(),
            },
            BuiltinFunction {
                name: "base64.decode".to_string(),
                description: "Decode a base64 string".to_string(),
                signature: "base64.decode(encoded: string): string".to_string(),
            },
            BuiltinFunction {
                name: "crypto.hash".to_string(),
                description: "Hash a string using SHA-256".to_string(),
                signature: "crypto.hash(text: string): string".to_string(),
            },
            BuiltinFunction {
                name: "url.parse".to_string(),
                description: "Parse a URL into components".to_string(),
                signature: "url.parse(url: string): URLComponents".to_string(),
            },
            BuiltinFunction {
                name: "url.build".to_string(),
                description: "Build a URL from components".to_string(),
                signature: "url.build(components: URLComponents): string".to_string(),
            },
        ]
    }
}

/// Built-in function documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltinFunction {
    pub name: String,
    pub description: String,
    pub signature: String,
}

/// URL components for script API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct URLComponents {
    pub scheme: String,
    pub host: String,
    pub port: Option<u16>,
    pub path: String,
    pub query: HashMap<String, String>,
    pub fragment: Option<String>,
}

impl URLComponents {
    pub fn parse(url: &str) -> Option<Self> {
        let parsed = url.parse::<hyper::Uri>().ok()?;

        let query = parsed
            .query()
            .map(|q| {
                url::form_urlencoded::parse(q.as_bytes())
                    .into_owned()
                    .collect()
            })
            .unwrap_or_default();

        Some(Self {
            scheme: parsed.scheme_str()?.to_string(),
            host: parsed.host()?.to_string(),
            port: parsed.port_u16(),
            path: parsed.path().to_string(),
            query,
            fragment: None, // hyper doesn't expose fragment
        })
    }

    pub fn build(&self) -> String {
        let mut url = format!("{}://{}", self.scheme, self.host);

        if let Some(port) = self.port {
            url.push_str(&format!(":{}", port));
        }

        url.push_str(&self.path);

        if !self.query.is_empty() {
            url.push('?');
            let query_str: Vec<String> = self
                .query
                .iter()
                .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
                .collect();
            url.push_str(&query_str.join("&"));
        }

        if let Some(ref fragment) = self.fragment {
            url.push_str(&format!("#{}", fragment));
        }

        url
    }
}

# Mock Responses - Enhanced Features

This document describes the enhanced mock response capabilities in Madhyamas, including dynamic responses, sequencing, conditional responses, collections, recording, and import/export features.

## Table of Contents

- [Overview](#overview)
- [Response Configuration Types](#response-configuration-types)
- [Template Variables](#template-variables)
- [Mock Collections](#mock-collections)
- [Hit Analytics](#hit-analytics)
- [Recording from Live Traffic](#recording-from-live-traffic)
- [Import/Export](#importexport)
- [API Reference](#api-reference)

## Overview

Mock responses allow you to intercept HTTP requests and return predefined responses without hitting the actual server. The enhanced mock system supports:

- **Single responses** - Return a static response
- **Sequence responses** - Cycle through multiple responses
- **Conditional responses** - Return different responses based on request conditions
- **Probabilistic responses** - Return responses based on weighted probabilities
- **Template variables** - Dynamic response content based on request data
- **Delay variance** - Add jitter to response delays for realistic simulation
- **Collections** - Organize mocks into groups
- **Recording** - Capture live traffic as mock rules
- **Import/Export** - HAR, OpenAPI, and Postman format support

## Response Configuration Types

### Single Response

The simplest configuration - returns the same response every time.

```json
{
  "type": "single",
  "response": {
    "status_code": 200,
    "headers": { "Content-Type": "application/json" },
    "body": "{\"message\": \"Hello, World!\"}"
  }
}
```

### Sequence Response

Cycles through a list of responses in order. Useful for testing pagination, retry logic, or state changes.

```json
{
  "type": "sequence",
  "responses": [
    { "status_code": 200, "body": "{\"page\": 1}" },
    { "status_code": 200, "body": "{\"page\": 2}" },
    { "status_code": 200, "body": "{\"page\": 3}" }
  ],
  "loop": true
}
```

Options:
- `loop` (boolean): When true, cycles back to the first response after reaching the end. When false, repeats the last response.

### Conditional Response

Returns different responses based on request conditions. Conditions are evaluated in order, and the first matching condition's response is returned.

```json
{
  "type": "conditional",
  "conditions": [
    {
      "condition": {
        "type": "header_equals",
        "name": "X-API-Version",
        "value": "v2"
      },
      "response": { "status_code": 200, "body": "{\"version\": 2}" }
    },
    {
      "condition": {
        "type": "query_param",
        "name": "debug",
        "value": "true"
      },
      "response": { "status_code": 200, "body": "{\"debug\": true}" }
    }
  ],
  "default_response": { "status_code": 200, "body": "{\"version\": 1}" }
}
```

#### Condition Types

| Type | Description | Parameters |
|------|-------------|------------|
| `header_equals` | Match exact header value | `name`, `value` |
| `header_regex` | Match header with regex | `name`, `pattern` |
| `query_param` | Match query parameter | `name`, `value` |
| `body_contains` | Check if body contains string | `value` |
| `body_json_path` | Match JSONPath expression | `path`, `expected` |
| `time_range` | Match time of day (UTC) | `start_hour`, `end_hour` |
| `hit_count_range` | Match based on hit count | `min_hits`, `max_hits` |

### Probabilistic Response

Returns responses based on weighted probabilities. Useful for testing error handling, A/B testing scenarios, or chaos engineering.

```json
{
  "type": "probabilistic",
  "responses": [
    { "weight": 80, "response": { "status_code": 200, "body": "{\"success\": true}" } },
    { "weight": 15, "response": { "status_code": 500, "body": "{\"error\": \"Server Error\"}" } },
    { "weight": 5, "response": { "status_code": 503, "body": "{\"error\": \"Service Unavailable\"}" } }
  ]
}
```

## Template Variables

Enable dynamic response content by setting `template_enabled: true` on a response. Template variables are replaced with values from the request.

### Built-in Variables

| Variable | Description |
|----------|-------------|
| `{{url}}` | Full request URL |
| `{{method}}` | HTTP method |
| `{{path}}` | URL path |
| `{{host}}` | Request host |
| `{{timestamp}}` | Current ISO timestamp |
| `{{timestamp_unix}}` | Unix timestamp |
| `{{uuid}}` | Random UUID |
| `{{random_int}}` | Random integer (0-999999) |
| `{{random_float}}` | Random float (0.00-1.00) |

### Request Data Variables

| Variable | Description |
|----------|-------------|
| `{{header:Name}}` | Value of header "Name" |
| `{{query:param}}` | Value of query parameter "param" |
| `{{path:0}}` | Path segment at index 0 |

### Example

```json
{
  "status_code": 200,
  "headers": { "Content-Type": "application/json" },
  "body": "{\"id\": \"{{uuid}}\", \"timestamp\": \"{{timestamp}}\", \"user\": \"{{header:X-User-Id}}\"}",
  "template_enabled": true
}
```

## Delay and Jitter

Add realistic latency to mock responses:

```json
{
  "status_code": 200,
  "body": "{}",
  "delay_ms": 500,
  "delay_variance_ms": 100
}
```

This returns a response with a delay between 400ms and 600ms (500ms ± 100ms).

## Mock Collections

Organize mocks into collections for better management.

### Create a Collection

```bash
POST /api/mocks/collections
{
  "name": "User API Mocks",
  "description": "Mocks for user-related endpoints",
  "tags": ["users", "auth"]
}
```

### Assign Mocks to Collections

When creating or updating a mock, set the `collection_id` field:

```json
{
  "name": "Get User",
  "collection_id": "collection-uuid-here",
  ...
}
```

### Toggle All Mocks in a Collection

```bash
POST /api/mocks/collections/{id}/toggle
{ "enabled": true }
```

## Hit Analytics

Track mock usage with hit analytics.

### Get All Hit History

```bash
GET /api/mocks/analytics
```

### Get Statistics for a Mock

```bash
GET /api/mocks/{id}/analytics
```

Returns:
```json
{
  "total_hits": 150,
  "avg_response_time_ms": 45,
  "min_response_time_ms": 12,
  "max_response_time_ms": 234,
  "first_hit": "2024-01-15T10:30:00Z",
  "last_hit": "2024-01-15T14:22:00Z"
}
```

### Get Hit History for a Mock

```bash
GET /api/mocks/{id}/history
```

### Clear Hit History

```bash
POST /api/mocks/analytics/clear
```

## Recording from Live Traffic

Capture live traffic and automatically create mock rules.

### Start Recording

```bash
POST /api/mocks/recording
{ "enabled": true }
```

### Stop Recording

```bash
POST /api/mocks/recording
{ "enabled": false }
```

### Get Recording Status

```bash
GET /api/mocks/recording/status
```

### View Recorded Mocks

```bash
GET /api/mocks/recording/recorded
```

### Promote Recorded Mocks to Active Rules

```bash
POST /api/mocks/recording/promote
```

### Clear Recorded Mocks

```bash
POST /api/mocks/recording/clear
```

## Import/Export

### Export Mocks

```bash
GET /api/mocks/export
```

Returns all mock rules as JSON.

### Import from HAR

```bash
POST /api/mocks/import
{
  "format": "har",
  "data": "{ ... HAR JSON ... }"
}
```

### Import from OpenAPI/Swagger

```bash
POST /api/mocks/import
{
  "format": "openapi",
  "data": "{ ... OpenAPI JSON ... }"
}
```

### Import from Postman Collection

```bash
POST /api/mocks/import
{
  "format": "postman",
  "data": "{ ... Postman Collection JSON ... }"
}
```

## Mock Expiration

Set mocks to automatically expire:

### Expire at Date/Time

```json
{
  "expiration": {
    "type": "date_time",
    "expires_at": "2024-12-31T23:59:59Z"
  }
}
```

### Expire After N Hits

```json
{
  "expiration": {
    "type": "hit_count",
    "max_hits": 100
  }
}
```

### Expire After Duration

```json
{
  "expiration": {
    "type": "duration",
    "duration_seconds": 3600
  }
}
```

## Version History

Mock rules maintain version history for rollback support.

### Get Version History

```bash
GET /api/mocks/{id}/versions
```

### Rollback to Previous Version

```bash
POST /api/mocks/{id}/rollback
{ "version": 2 }
```

## API Reference

### Mock Rules

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/mocks` | List all mock rules |
| POST | `/api/mocks` | Create a mock rule |
| GET | `/api/mocks/{id}` | Get a mock rule |
| PUT | `/api/mocks/{id}` | Update a mock rule |
| DELETE | `/api/mocks/{id}` | Delete a mock rule |
| POST | `/api/mocks/{id}/toggle` | Enable/disable a mock |
| POST | `/api/mocks/{id}/duplicate` | Duplicate a mock |
| POST | `/api/mocks/{id}/test` | Test a mock against a request |
| GET | `/api/mocks/{id}/versions` | Get version history |
| POST | `/api/mocks/{id}/rollback` | Rollback to a version |

### Collections

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/mocks/collections` | List all collections |
| POST | `/api/mocks/collections` | Create a collection |
| GET | `/api/mocks/collections/{id}` | Get a collection |
| DELETE | `/api/mocks/collections/{id}` | Delete a collection |
| POST | `/api/mocks/collections/{id}/toggle` | Toggle all mocks in collection |

### Analytics

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/mocks/analytics` | Get all hit history |
| GET | `/api/mocks/{id}/analytics` | Get stats for a mock |
| GET | `/api/mocks/{id}/history` | Get hit history for a mock |
| POST | `/api/mocks/analytics/clear` | Clear all hit history |

### Recording

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/mocks/recording/status` | Get recording status |
| POST | `/api/mocks/recording` | Set recording mode |
| GET | `/api/mocks/recording/recorded` | Get recorded mocks |
| POST | `/api/mocks/recording/promote` | Promote recorded to active |
| POST | `/api/mocks/recording/clear` | Clear recorded mocks |

### Import/Export

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/mocks/export` | Export all mocks as JSON |
| POST | `/api/mocks/import` | Import mocks from HAR/OpenAPI/Postman |

### Preview

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/mocks/preview` | Preview which mock matches a request |

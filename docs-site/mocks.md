---
title: Mocks
description: Intercept requests and return fake API responses without hitting the real server — match criteria, collections, recording, import/export, and common use cases.
---

# Mocks

Mocks let you intercept requests and return **fake responses** without hitting the real server. This is perfect for testing how your app handles different API responses, developing against APIs that don't exist yet, or testing edge cases that are hard to reproduce with real services.

![Mocks View](/screenshots/mocks-view.png)

## How Mocks Work

When a request matches a mock rule:

1. Madhyamas **intercepts** the request before it reaches the server
2. It returns the **mock response** you defined (status code, headers, body)
3. The real server is never contacted
4. The traffic appears in the traffic list with a "mocked" indicator

Mocks are evaluated after rewrites but before breakpoints in the interception pipeline.

## Creating a Mock

1. Navigate to the **Mocks** view using the left navigation rail
2. Click **Add Mock**
3. Configure the mock:

### Match Criteria

| Field | Description |
|-------|-------------|
| **URL Pattern** | Wildcard pattern to match the request URL (e.g., `*/api/users/*`) |
| **Method** | HTTP method to match (GET, POST, etc.) — leave empty for any |
| **Request Headers** | Optional header match conditions |
| **Request Body** | Optional body match condition |

### Response Configuration

| Field | Description |
|-------|-------------|
| **Status Code** | HTTP status code to return (e.g., 200, 404, 500) |
| **Response Headers** | Custom headers to include in the response |
| **Response Body** | The response body (text, JSON, XML, etc.) |
| **Content Type** | The Content-Type header value |
| **Delay** | Optional delay in milliseconds before responding |

4. Click **Save** to activate the mock

## Mock Collections

Organize related mocks into **collections** for easy management. For example, you might have:

- A "User API" collection with mocks for login, profile, and logout endpoints
- An "Error Scenarios" collection with 500, 502, and 503 responses
- A "Development" collection with mock data for local development

### Creating a Collection

1. In the Mocks view, click **New Collection**
2. Give it a name and optional description
3. Add mocks to the collection by selecting it when creating or editing a mock

### Toggling Collections

Enable or disable an entire collection with a single toggle. This lets you quickly switch between different mock scenarios.

## Recording Mocks

Instead of creating mocks from scratch, you can **record** mocks from real traffic:

1. Enable mock recording mode
2. Make requests through the proxy as usual
3. Madhyamas captures the real request/response pairs
4. Stop recording
5. Review and save the captured interactions as mocks

This is the fastest way to create realistic mock data from actual API behavior.

## Importing and Exporting Mocks

### Export

Export your mocks to a JSON file for backup or sharing:

1. In the Mocks view, click the menu (⋯) → **Export**
2. Choose to export all mocks or a specific collection
3. Save the `.json` file

### Import

Import mocks from a previously exported file:

1. Click the menu (⋯) → **Import**
2. Select the `.json` file
3. Choose whether to merge with existing mocks or replace them

## Common Use Cases

### Developing Against an Unfinished API

Mock the API endpoints your frontend needs, with realistic response data, so you can build the UI before the backend is ready.

### Testing Error Scenarios

Create mocks that return 500 errors, timeouts, or malformed responses to verify your app handles them gracefully.

### Reproducing Production Issues

Record traffic from a production environment, export it as mocks, and replay the exact responses locally to reproduce and debug issues.

### Demo Environments

Create a collection of mocks that provides consistent, predictable responses for demos and screenshots — no dependency on live APIs.

## See also

- [Rewrites](./rewrites) — modify live traffic instead of replacing it
- [Breakpoints](./breakpoints) — interactive, one-off modifications
- [Replay](./replay) — re-execute captured requests
- [Scripting](./scripting) — dynamic, programmatic mocks
- [REST API reference](./rest-api) — `/api/mocks` endpoints

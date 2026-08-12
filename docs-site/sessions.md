---
title: Sessions
description: Organize captured traffic into named sessions in Madhyamas to keep different debugging contexts separate — create, switch, export, and delete sessions.
---

# Sessions

Sessions let you **organize traffic into named groups** so you can keep different debugging contexts separate. Instead of one long list of all traffic, you can create sessions for different tasks, bugs, or environments and switch between them easily.

![Sessions View](/screenshots/sessions-view.png)

## Why Use Sessions?

Without sessions, all captured traffic goes into a single default session. Over time, this becomes a mix of unrelated requests that's hard to navigate. Sessions solve this by letting you:

- Keep traffic from different debugging tasks separate
- Save a specific set of traffic for later analysis
- Share a captured session with a colleague
- Compare traffic from different environments

## Creating a Session

1. Navigate to the **Sessions** view using the left navigation rail
2. Click **New Session**
3. Give it a name (e.g., "Login Bug Investigation") and optional description
4. Click **Create**

The new session starts empty. All new traffic will be captured into whichever session is currently active.

## Switching Sessions

Click any session in the list to switch to it. The traffic view updates to show only the traffic in that session. New traffic is captured into the active session.

The currently active session is highlighted with a colored indicator. A badge shows the number of traffic entries in each session.

## Exporting Sessions

Export a session to share it or archive it:

1. Select the session you want to export
2. Click **Export** → choose **HAR** format
3. Save the `.har` file

HAR files can be opened in browser DevTools, Charles Proxy, Fiddler, or any HAR-compatible tool.

## Importing Sessions

Import a previously exported session:

1. Click **Import**
2. Select a `.har` file
3. The session is created with the imported traffic

This is useful for sharing debugging context with teammates or analyzing traffic captured by someone else.

## Deleting Sessions

Delete a session to remove it and all its traffic:

1. Select the session
2. Click **Delete**
3. Confirm the deletion

::: warning
Deleting a session permanently removes all traffic in it. Export first if you want to preserve the data.
:::

## The Default Session

Madhyamas always has a default session. If you don't create any sessions, all traffic goes here. You can't delete the default session, but you can clear its traffic.

## Common Use Cases

### Bug Investigation

Create a session named after the bug you're investigating. Capture only the relevant traffic. When you're done, export it and attach it to the bug report.

### Environment Comparison

Create separate sessions for "Production", "Staging", and "Local". Switch between them to compare how the same app behaves in different environments.

### Feature Development

Create a session for the feature you're working on. Capture traffic as you develop and test. Keep it for reference or export it as documentation of the API behavior.

## See also

- [Traffic Inspection](./traffic-inspection) — viewing and filtering traffic within a session
- [Importing HAR Files](./har-import) — import external captures as sessions
- [Auto Save](./auto-save) — periodic session backups
- [REST API reference](./rest-api) — `/api/sessions` endpoints

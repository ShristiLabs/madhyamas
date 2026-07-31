# Replay

Replay lets you **re-execute previously captured requests** — either one at a time or as a saved sequence. This is invaluable for reproducing issues, testing the same request against different servers, or automating repetitive API testing.

![Replay View](/screenshots/replay-view.png)

## How Replay Works

When you replay a request, Madhyamas sends it to the server again using the original method, URL, headers, and body. The response is captured as a new traffic entry, so you can compare it with the original.

## Replaying a Single Request

### From the Traffic View

1. Right-click any traffic entry in the traffic list
2. Select **Replay** from the context menu
3. The request is sent immediately
4. The new response appears as a new traffic entry

### From the Replay View

1. Navigate to the **Replay** view
2. Select a previously saved request from the list
3. Click **Replay** to execute it
4. View the response in the detail panel

## Saving Requests for Replay

Instead of searching through traffic history, you can save specific requests for easy replay:

1. Right-click a traffic entry → **Save for Replay**
2. Give it a name and optional description
3. It appears in the Replay view's saved list

Saved requests persist across restarts, so you can build a library of commonly tested API calls.

## Modifying Before Replay

Before replaying, you can modify the request:

1. Select a saved request in the Replay view
2. Click **Edit** to open the request editor
3. Change the URL, method, headers, or body
4. Click **Replay** to send the modified request

This is useful for:
- Testing different parameter values
- Adding or removing headers
- Changing the request body
- Pointing to a different server

## Replay History

Every replay execution is recorded in the **Replay History** tab. Each entry shows:

- The original request that was replayed
- The timestamp of the replay
- The response status code and timing
- The full response details

This lets you compare results across multiple replays — for example, to see if a server's response has changed over time.

## Common Use Cases

### Reproducing a Bug

Capture the exact request that caused a bug, save it, and replay it after each code change to verify the fix works.

### API Regression Testing

Save a set of key API requests and replay them after deploying a new version to verify the responses haven't changed.

### Performance Comparison

Replay the same request multiple times and compare response times to track performance trends.

### Testing Different Environments

Save a request, then modify the URL to point to staging or production, and replay to compare responses across environments.

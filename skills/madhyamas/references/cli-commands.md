# CLI Commands Reference

All 58 CLI subcommands for the `madhyamas` binary. The CLI communicates with a running Madhyamas proxy instance via REST API.

## Global Flags

```
madhyamas [GLOBAL FLAGS] <COMMAND> [SUBCOMMAND] [OPTIONS]

Global Flags:
  --api-url <URL>     API server URL [default: http://127.0.0.1:3001]
                       [env: MADHYAMAS_API_URL]
  -v, --verbose        Enable verbose logging
  -h, --help           Print help
  -V, --version        Print version
```

Most subcommands support `--json` for machine-readable JSON output.

## traffic — Traffic Inspection

### traffic list

List captured traffic with optional filters.

```bash
madhyamas traffic list [OPTIONS]

Options:
  -f, --filter <FILTER>    Filter by URL pattern
  -m, --method <METHOD>    Filter by HTTP method
  -s, --status <STATUS>    Filter by HTTP status code
  -l, --limit <LIMIT>      Max results [default: 100]
      --json               Output as JSON
```

Example: `madhyamas traffic list --method POST --status 500 --json`

### traffic get

Get a specific traffic entry by ID.

```bash
madhyamas traffic get <ID> [OPTIONS]

Arguments:
  <ID>    Traffic entry ID

Options:
      --json    Output as JSON
```

### traffic search

Search traffic by content (headers, bodies, URLs).

```bash
madhyamas traffic search <QUERY> [OPTIONS]

Arguments:
  <QUERY>    Search query

Options:
      --json    Output as JSON
```

Example: `madhyamas traffic search "authorization"`

### traffic count

Get total count of captured traffic entries.

```bash
madhyamas traffic count
```

### traffic clear

Clear all captured traffic.

```bash
madhyamas traffic clear
```

## mocks — Mock Responses

### mocks list

List all mock rules.

```bash
madhyamas mocks list
```

### mocks create

Create a mock rule.

```bash
madhyamas mocks create --url-pattern <PATTERN> [OPTIONS]

Options:
  -u, --url-pattern <PATTERN>    URL pattern (supports wildcards)
  -m, --method <METHOD>          HTTP method to match
  -s, --status-code <CODE>       Response status code
  -b, --body <BODY>              Response body
  -d, --delay-ms <MS>            Response delay in milliseconds
  -e, --enabled <BOOL>           Enable or disable
      --json                     Output as JSON
```

Example: `madhyamas mocks create --url-pattern "*/api/auth*" --status-code 200 --body '{"token":"fake"}'`

### mocks delete

Delete a mock rule.

```bash
madhyamas mocks delete <ID>

Arguments:
  <ID>    Mock ID
```

### mocks toggle

Toggle a mock rule on/off.

```bash
madhyamas mocks toggle <ID> <ENABLED>

Arguments:
  <ID>        Mock ID
  <ENABLED>   Enable or disable (true/false)
```

## breakpoints — Breakpoints

### breakpoints list

List all breakpoint rules.

```bash
madhyamas breakpoints list
```

### breakpoints create

Create a breakpoint rule.

```bash
madhyamas breakpoints create --url-pattern <PATTERN> [OPTIONS]

Options:
  -u, --url-pattern <PATTERN>    URL pattern to match
  -m, --method <METHOD>          HTTP method to match
  -d, --direction <DIR>          Direction (request/response)
  -e, --enabled <BOOL>           Enable or disable
      --json                     Output as JSON
```

Example: `madhyamas breakpoints create --url-pattern "*/auth*" --direction request`

### breakpoints delete

Delete a breakpoint rule.

```bash
madhyamas breakpoints delete <ID>

Arguments:
  <ID>    Breakpoint ID
```

## sessions — Session Management

### sessions list

List all sessions.

```bash
madhyamas sessions list
```

### sessions create

Create a new session.

```bash
madhyamas sessions create [OPTIONS]

Options:
  -n, --name <NAME>              Session name
  -d, --description <DESC>       Session description
      --json                     Output as JSON
```

### sessions delete

Delete a session.

```bash
madhyamas sessions delete <ID>

Arguments:
  <ID>    Session ID
```

### sessions switch

Switch active session.

```bash
madhyamas sessions switch <ID> [OPTIONS]

Arguments:
  <ID>    Session ID

Options:
      --json    Output as JSON
```

### sessions export

Export a session.

```bash
madhyamas sessions export <ID> [OPTIONS]

Arguments:
  <ID>    Session ID

Options:
  -f, --format <FORMAT>    Export format (har, curl) [default: har]
      --json               Output as JSON
```

## replay — Request Replay

### replay run

Replay a captured request.

```bash
madhyamas replay run <ID> [OPTIONS]

Arguments:
  <ID>    Traffic entry ID to replay

Options:
      --json    Output as JSON
```

### replay save

Save a request for later replay.

```bash
madhyamas replay save <TRAFFIC_ID> [OPTIONS]

Arguments:
  <TRAFFIC_ID>    Traffic entry ID

Options:
  -n, --name <NAME>           Optional name for the saved request
  -d, --description <DESC>    Optional description
      --json                  Output as JSON
```

### replay list

List saved requests.

```bash
madhyamas replay list [OPTIONS]

Options:
      --json    Output as JSON
```

### replay delete

Delete a saved request.

```bash
madhyamas replay delete <ID>

Arguments:
  <ID>    Saved request ID
```

### replay export

Export a request as cURL or HAR.

```bash
madhyamas replay export <ID> [OPTIONS]

Arguments:
  <ID>    Traffic entry ID

Options:
  -f, --format <FORMAT>    Export format (curl, har) [default: curl]
```

### replay history

Show replay execution history.

```bash
madhyamas replay history [OPTIONS]

Options:
      --json    Output as JSON
```

## config — Configuration

### config get

Get current proxy configuration.

```bash
madhyamas config get [OPTIONS]

Options:
      --json    Output as JSON
```

### config update

Update runtime configuration.

```bash
madhyamas config update [OPTIONS]

Options:
      --intercept-https <BOOL>    Enable/disable HTTPS interception
      --max-requests <NUM>        Max requests in memory
      --verbose <BOOL>            Enable/disable verbose logging
      --public-ip <IP>            Public IP (use "null" to clear)
      --json                      Output as JSON
```

Example: `madhyamas config update --intercept-https false`

## capture — Capture Mode

### capture status

Get capture mode status (recording or passthrough).

```bash
madhyamas capture status [OPTIONS]

Options:
      --json    Output as JSON
```

### capture toggle

Toggle capture mode (recording <-> passthrough).

```bash
madhyamas capture toggle
```

### capture enable

Enable traffic recording.

```bash
madhyamas capture enable
```

### capture disable

Disable traffic recording (passthrough mode).

```bash
madhyamas capture disable
```

## throttle — Network Throttling

### throttle get

Get current throttle profile.

```bash
madhyamas throttle get
```

### throttle set

Set throttle parameters.

```bash
madhyamas throttle set [OPTIONS]

Options:
      --download-bps <BPS>    Download bandwidth (bytes/sec)
      --upload-bps <BPS>      Upload bandwidth (bytes/sec)
      --delay-ms <MS>         Added latency in milliseconds
  -n, --name <NAME>           Optional preset name
```

Example: `madhyamas throttle set --download-bps 50000 --delay-ms 200`

### throttle enable

Enable throttling.

```bash
madhyamas throttle enable
```

### throttle disable

Disable throttling.

```bash
madhyamas throttle disable
```

### throttle presets

List available throttle presets.

```bash
madhyamas throttle presets
```

## rewrites — Rewrite Rules

### rewrites list

List all rewrite rules.

```bash
madhyamas rewrites list
```

### rewrites create

Create a rewrite rule.

```bash
madhyamas rewrites create --name <NAME> --pattern <PATTERN> --action <ACTION>

Options:
  -n, --name <NAME>        Rule name
  -p, --pattern <PATTERN>  URL pattern to match
  -a, --action <ACTION>    Replacement action (new URL or body)
```

### rewrites delete

Delete a rewrite rule.

```bash
madhyamas rewrites delete <ID>

Arguments:
  <ID>    Rewrite rule ID
```

### rewrites toggle

Toggle a rewrite rule on/off.

```bash
madhyamas rewrites toggle <ID>

Arguments:
  <ID>    Rewrite rule ID
```

### rewrites templates

List available rewrite templates.

```bash
madhyamas rewrites templates
```

## grpc — gRPC Inspection

### grpc connections

List gRPC connections.

```bash
madhyamas grpc connections
```

### grpc streams

List gRPC streams.

```bash
madhyamas grpc streams
```

### grpc frames

List gRPC frames with optional filters.

```bash
madhyamas grpc frames [OPTIONS]

Options:
      --connection-id <ID>    Filter by connection ID
      --stream-id <ID>        Filter by stream ID
  -l, --limit <LIMIT>         Max results [default: 100]
```

### grpc stats

Get gRPC statistics.

```bash
madhyamas grpc stats
```

### grpc clear

Clear all gRPC frames.

```bash
madhyamas grpc clear
```

## scripts — Script Management

### scripts list

List all scripts.

```bash
madhyamas scripts list
```

### scripts create

Create a script.

```bash
madhyamas scripts create --name <NAME> --hook <HOOK> [OPTIONS]

Options:
  -n, --name <NAME>     Script name
  -h, --hook <HOOK>     Hook/event (e.g., request, response)
      --file <PATH>     Path to script file (conflicts with --inline)
  -i, --inline <CODE>   Inline script source (conflicts with --file)
```

Example: `madhyamas scripts create --name "log-requests" --hook request --inline "console.log(request.url)"`

### scripts get

Get a specific script.

```bash
madhyamas scripts get <ID>

Arguments:
  <ID>    Script ID
```

### scripts delete

Delete a script.

```bash
madhyamas scripts delete <ID>

Arguments:
  <ID>    Script ID
```

### scripts toggle

Toggle a script on/off.

```bash
madhyamas scripts toggle <ID>

Arguments:
  <ID>    Script ID
```

### scripts templates

List available script templates.

```bash
madhyamas scripts templates
```

## plugins — Plugin Management

### plugins list

List all plugins.

```bash
madhyamas plugins list
```

### plugins get

Get a specific plugin.

```bash
madhyamas plugins get <ID>

Arguments:
  <ID>    Plugin ID
```

### plugins enable

Enable a plugin.

```bash
madhyamas plugins enable <ID>

Arguments:
  <ID>    Plugin ID
```

### plugins disable

Disable a plugin.

```bash
madhyamas plugins disable <ID>

Arguments:
  <ID>    Plugin ID
```

### plugins stats

Get statistics for a plugin.

```bash
madhyamas plugins stats <ID>

Arguments:
  <ID>    Plugin ID
```

### plugins reload

Reload all plugins from disk.

```bash
madhyamas plugins reload
```

## export — Export Traffic

### export har

Export captured traffic as HAR format.

```bash
madhyamas export har [OPTIONS]

Options:
  -o, --output <FILE>    Write to file instead of stdout
```

Example: `madhyamas export har --output traffic.har`

### export curl

Export a traffic entry as a cURL command.

```bash
madhyamas export curl <ID>

Arguments:
  <ID>    Traffic entry ID
```

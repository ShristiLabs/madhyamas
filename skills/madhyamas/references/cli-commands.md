# CLI Commands Reference

All 128 CLI subcommands for the `madhyamas` binary. The CLI communicates with a running Madhyamas proxy instance via REST API.

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

### mocks update

Update an existing mock rule.

```bash
madhyamas mocks update <ID> [OPTIONS]

Arguments:
  <ID>    Mock ID

Options:
  -u, --url-pattern <PATTERN>    URL pattern
  -m, --method <METHOD>          HTTP method to match
  -s, --status-code <CODE>       Response status code
  -b, --body <BODY>              Response body
  -d, --delay-ms <MS>            Response delay in milliseconds
  -e, --enabled <BOOL>           Enable or disable
      --json                     Output as JSON
```

### mocks duplicate

Duplicate an existing mock rule.

```bash
madhyamas mocks duplicate <ID> [OPTIONS]

Arguments:
  <ID>    Mock ID to duplicate

Options:
  -n, --new-name <NAME>    Optional new name for the duplicate
```

### mocks rollback

Rollback a mock rule to a previous version.

```bash
madhyamas mocks rollback <ID> [OPTIONS]

Arguments:
  <ID>    Mock ID

Options:
  -v, --version <VERSION>    Version number to rollback to
```

### mocks versions

Get version history for a mock rule.

```bash
madhyamas mocks versions <ID>

Arguments:
  <ID>    Mock ID
```

### mocks create-advanced

Create an advanced mock rule from a JSON config.

```bash
madhyamas mocks create-advanced [OPTIONS]

Options:
  -c, --config <JSON>          Advanced mock config as a JSON string
      --config-file <PATH>     Read config from a JSON file
```

### mocks analytics

Get mock hit analytics (global or per-rule).

```bash
madhyamas mocks analytics [ID]

Arguments:
  [ID]    Optional mock ID (if omitted, returns global analytics)
```

### mocks history

Get mock hit history for a specific rule.

```bash
madhyamas mocks history <ID>

Arguments:
  <ID>    Mock ID
```

### mocks preview

Preview which mock rule would match a given request.

```bash
madhyamas mocks preview [OPTIONS]

Options:
  -m, --method <METHOD>    HTTP method
  -u, --url <URL>          Request URL
  -h, --headers <JSON>     Request headers as JSON
  -b, --body <BODY>        Request body
```

### mocks test

Test a mock rule against a sample request.

```bash
madhyamas mocks test <ID> [OPTIONS]

Arguments:
  <ID>    Mock ID to test

Options:
  -m, --method <METHOD>    HTTP method
  -u, --url <URL>          Request URL
  -h, --headers <JSON>     Request headers as JSON
  -b, --body <BODY>        Request body
```

### mocks export

Export all mock rules as JSON.

```bash
madhyamas mocks export [OPTIONS]

Options:
  -o, --output <FILE>    Write to file instead of stdout
```

### mocks import

Import mock rules from a file.

```bash
madhyamas mocks import --input <FILE> [OPTIONS]

Options:
  -i, --input <FILE>      Input JSON file
  -f, --format <FORMAT>   Import format (har, openapi, postman) [default: har]
```

### mocks recording

Mock recording subcommands.

```bash
madhyamas mocks recording <SUBCOMMAND>

Subcommands:
  set --enabled <BOOL>    Enable or disable recording mode
  status                  Get current recording status
  list                    List all recorded mock candidates
  promote                 Promote recorded mocks to active rules
```

### mocks collections

Mock collection subcommands.

```bash
madhyamas mocks collections <SUBCOMMAND>

Subcommands:
  list                                    List all mock collections
  create --name <NAME> [--description D]  Create a new collection
  get <ID>                                Get a specific collection
  delete <ID> [--delete-rules]            Delete a collection
  toggle <ID> <ENABLED>                   Toggle all mocks in a collection
  update <ID> [--name N] [--description D] [--enabled BOOL]  Update collection metadata
```

### mocks get

Get a specific mock rule by ID.

```bash
madhyamas mocks get <ID>

Arguments:
  <ID>    Mock ID
```

### mocks batch-toggle

Batch toggle multiple mock rules.

```bash
madhyamas mocks batch-toggle --ids <IDS> --enabled <BOOL>

Options:
  -i, --ids <IDS>        Comma-separated list of mock rule IDs
  -e, --enabled <BOOL>   Enable or disable
```

### mocks templates

List available predefined mock templates.

```bash
madhyamas mocks templates
```

### mocks clear-recording

Clear all recorded mock candidates.

```bash
madhyamas mocks clear-recording
```

### mocks clear-analytics

Clear all mock hit history and analytics data.

```bash
madhyamas mocks clear-analytics
```

## breakpoints — Breakpoints

### breakpoints list

List all breakpoint rules.

```bash
madhyamas breakpoints list
```

### breakpoints get

Get a specific breakpoint rule by ID.

```bash
madhyamas breakpoints get <ID>
```

Arguments:
  <ID>    Breakpoint rule ID

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

### breakpoints paused

Paused traffic subcommands.

```bash
madhyamas breakpoints paused <SUBCOMMAND>

Subcommands:
  list              List all traffic paused by breakpoints
  get <ID>          Get a specific paused traffic item
  resume <ID> <ACTION>   Resume a paused item (action: continue or abort)
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

### rewrites update

Update a rewrite rule.

```bash
madhyamas rewrites update <ID> [OPTIONS]

Arguments:
  <ID>    Rewrite rule ID

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

### rewrites batch-toggle

Batch toggle multiple rewrite rules.

```bash
madhyamas rewrites batch-toggle --ids <IDS> --enabled <BOOL>

Options:
  -i, --ids <IDS>        Comma-separated list of rewrite rule IDs
  -e, --enabled <BOOL>   Enable or disable
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

### scripts test

Test (dry-run) a script against a sample context without affecting live traffic.

```bash
madhyamas scripts test [OPTIONS]

Options:
  -i, --inline <CODE>   Inline script source to test
      --file <PATH>     Path to a file containing the script source to test
  -H, --hook <HOOK>     Hook to test against (e.g. on_request, on_response)
```

Example: `madhyamas scripts test --inline "console.log(request.url)" --hook on_request`

### scripts validate

Validate a script's syntax without executing it.

```bash
madhyamas scripts validate [OPTIONS]

Options:
  -i, --inline <CODE>   Inline script source to validate
      --file <PATH>     Path to a file containing the script source to validate
```

### scripts history

Show execution history for a specific script.

```bash
madhyamas scripts history <ID> [OPTIONS]

Arguments:
  <ID>    Script ID

Options:
  -l, --limit <LIMIT>   Max number of history entries [default: 20]
```

### scripts history-all

Show execution history across all scripts.

```bash
madhyamas scripts history-all
```

### scripts history-clear

Clear execution history for a specific script.

```bash
madhyamas scripts history-clear <ID>

Arguments:
  <ID>    Script ID
```

### scripts reorder

Reorder a script by changing its priority (lower = earlier in the chain).

```bash
madhyamas scripts reorder <ID> --priority <PRIORITY>

Arguments:
  <ID>    Script ID

Options:
  -p, --priority <PRIORITY>   New priority position
```

### scripts match-preview

Preview which scripts would match a given request without executing them.

```bash
madhyamas scripts match-preview [OPTIONS]

Options:
      --url <URL>       URL to test (required)
      --method <METHOD> HTTP method [default: GET]
```

### scripts config

Get global script runtime configuration.

```bash
madhyamas scripts config
```

### scripts config-update

Update global script runtime configuration.

```bash
madhyamas scripts config-update [OPTIONS]

Options:
      --timeout-ms <MS>          Execution timeout in milliseconds
      --memory-limit-mb <MB>     Memory limit in MB
      --capture-console <BOOL>   Enable console output capture
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

### plugins install

Install a plugin from a URL or registry id.

```bash
madhyamas plugins install [OPTIONS] <TARGET>

Arguments:
  <TARGET>    Plugin URL (when source=url) or registry id (when source=registry)

Options:
      --source <SOURCE>   Install source: "url" or "registry" [default: url]
      --checksum <HASH>   Expected SHA-256 checksum (optional for URL source)
```

Example: `madhyamas plugins install --source registry cors-helper`

### plugins uninstall

Uninstall a plugin (removes from disk and persistence).

```bash
madhyamas plugins uninstall <ID>

Arguments:
  <ID>    Plugin ID
```

### plugins search

Search the plugin registry by name, description, or tags.

```bash
madhyamas plugins search <QUERY>

Arguments:
  <QUERY>    Search query
```

### plugins registry

List all available plugins in the registry.

```bash
madhyamas plugins registry
```

### plugins registry-config

Show or set the registry GitHub repo (e.g. "owner/repo" or "owner/repo@branch").

```bash
madhyamas plugins registry-config [REPO]

Arguments:
  [REPO]    Set the registry repo (omit to just show current config)
```

### plugins registry-refresh

Force-refresh the registry cache from the configured GitHub repository.

```bash
madhyamas plugins registry-refresh
```

### plugins schema

Get a plugin's settings schema (for UI generation).

```bash
madhyamas plugins schema <ID>

Arguments:
  <ID>    Plugin ID
```

### plugins get-settings

Get a plugin's current settings.

```bash
madhyamas plugins get-settings <ID>

Arguments:
  <ID>    Plugin ID
```

### plugins set-settings

Update a plugin's settings (pass JSON via --settings).

```bash
madhyamas plugins set-settings <ID> --settings <JSON>

Arguments:
  <ID>    Plugin ID

Options:
  -s, --settings <JSON>   Settings as a JSON string
```

### plugins logs

Get a plugin's recent invocation logs.

```bash
madhyamas plugins logs <ID> [OPTIONS]

Arguments:
  <ID>    Plugin ID

Options:
  -l, --limit <LIMIT>   Maximum number of log entries [default: 50]
```

### plugins gen-key

Generate a new Ed25519 keypair for signing plugins.

```bash
madhyamas plugins gen-key [OPTIONS]

Options:
      --format <FORMAT>   Output format: "hex" (default) or "json" [default: hex]
```

### plugins sign

Sign a plugin zip package with a publisher secret key.

```bash
madhyamas plugins sign <ZIP_PATH> --secret-key <KEY> [OPTIONS]

Arguments:
  <ZIP_PATH>    Path to the plugin zip package to sign

Options:
  -s, --secret-key <KEY>   Publisher secret key as hex (64 hex chars = 32 bytes)
  -o, --output <PATH>      Output path for the signature file (default: <zip_path>.sig)
```

### plugins new

Scaffold a new plugin project from a template.

```bash
madhyamas plugins new <TEMPLATE> <NAME> [OPTIONS]

Arguments:
  <TEMPLATE>    Template id: basic, cors, request-logger, domain-blocker, response-modifier
  <NAME>        Plugin name (kebab-case, e.g. "my-cors-plugin")

Options:
  -o, --output <DIR>   Output directory [default: .]
```

### plugins templates

List available plugin scaffolding templates.

```bash
madhyamas plugins templates
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

## autosave — Auto Save

### autosave get

Get current Auto Save configuration.

```bash
madhyamas autosave get [OPTIONS]

Options:
      --json    Output as JSON
```

### autosave update

Update Auto Save configuration. Only provided fields are updated.

```bash
madhyamas autosave update [OPTIONS]

Options:
      --enabled <BOOL>                  Enable or disable Auto Save
      --interval-seconds <SECONDS>      Seconds between snapshots
      --export-format <FORMAT>          Export format: "har" or "session"
      --output-dir <DIR>                Backup directory path
      --max-backups <NUM>               Number of backups to keep
      --rotate-after-requests <NUM>     Rotate session after N requests (0 to disable)
      --rotate-after-minutes <NUM>      Rotate session after N minutes (0 to disable)
      --json                            Output as JSON
```

Example: `madhyamas autosave update --enabled true --interval-seconds 300`

### autosave snapshot

Trigger an immediate Auto Save snapshot (save now).

```bash
madhyamas autosave snapshot [OPTIONS]

Options:
      --json    Output as JSON
```

## blocklist — Block List

### blocklist list

List all block list entries.

```bash
madhyamas blocklist list
```

### blocklist stats

View block list summary statistics (total entries, enabled count, total hits).

```bash
madhyamas blocklist stats
```

### blocklist get

Get a specific block list entry by ID.

```bash
madhyamas blocklist get <ID>

Arguments:
  <ID>    Block list entry ID
```

### blocklist create

Create a block list entry.

```bash
madhyamas blocklist create --pattern <PATTERN> [OPTIONS]

Options:
  -p, --pattern <PATTERN>       Domain or wildcard pattern to block
  -n, --note <NOTE>             Optional note describing why this entry exists
  -e, --enabled <BOOL>          Whether the entry is enabled (default: true)
      --status-code <CODE>      HTTP status code to return (default: 403)
      --response-body <BODY>    Response body to return when blocked
      --content-type <TYPE>     Content-Type header for the block response
      --json                    Output as JSON
```

Example: `madhyamas blocklist create --pattern "*.ads.example.com" --status-code 403`

### blocklist update

Update a block list entry.

```bash
madhyamas blocklist update <ID> [OPTIONS]

Arguments:
  <ID>    Block list entry ID

Options:
  -p, --pattern <PATTERN>       Domain or wildcard pattern
  -n, --note <NOTE>             Optional note
  -e, --enabled <BOOL>          Enable or disable
      --status-code <CODE>      HTTP status code
      --response-body <BODY>    Response body
      --content-type <TYPE>     Content-Type header
```

### blocklist delete

Delete a block list entry.

```bash
madhyamas blocklist delete <ID>

Arguments:
  <ID>    Block list entry ID
```

### blocklist toggle

Enable or disable a block list entry.

```bash
madhyamas blocklist toggle <ID> <ENABLED>

Arguments:
  <ID>        Block list entry ID
  <ENABLED>   Enable (true) or disable (false)
```

## focus — Focus Hosts

### focus list

List all focus host patterns.

```bash
madhyamas focus list
```

### focus add

Add a focus host pattern to highlight matching traffic.

```bash
madhyamas focus add <PATTERN>

Arguments:
  <PATTERN>    Host pattern (e.g. `api.example.com`, `*.example.com`, `*api*`)
```

### focus remove

Remove a focus host by ID.

```bash
madhyamas focus remove <ID>

Arguments:
  <ID>    Focus host ID
```

### focus clear

Clear all focus hosts.

```bash
madhyamas focus clear
```

## logs — Log Rotation

### logs status

Show current log rotation status and archived files.

```bash
madhyamas logs status [OPTIONS]

Options:
      --json    Output as JSON
```

### logs rotate

Rotate the current log file immediately (on-demand).

```bash
madhyamas logs rotate
```

### logs config

Update log rotation configuration. Only provided fields are updated.

```bash
madhyamas logs config [OPTIONS]

Options:
      --enabled <BOOL>           Enable or disable file logging
      --rotation <MODE>          Rotation mode: never, hourly, daily, or size
      --size-mb <MB>             Size in MB (only used with --rotation size)
      --max-files <NUM>          Maximum number of archived log files to keep
      --max-file-size-mb <MB>    Hard per-file size cap in MB
      --json-format <BOOL>       Use structured JSON log format
      --json                     Output as JSON
```

Example: `madhyamas logs config --rotation daily --max-files 10`

## mirror — Mirror Tool

### mirror status

Show current mirror status and statistics.

```bash
madhyamas mirror status [OPTIONS]

Options:
      --json    Output as JSON
```

### mirror start

Start mirroring (enable the mirror tool).

```bash
madhyamas mirror start
```

### mirror stop

Stop mirroring (disable the mirror tool).

```bash
madhyamas mirror stop
```

### mirror config

Update mirror configuration. Only provided fields are updated.

```bash
madhyamas mirror config [OPTIONS]

Options:
      --enabled <BOOL>             Enable or disable mirroring
      --output-dir <DIR>           Output directory for mirrored files
      --host-filter <PATTERNS>     Comma-separated host filter patterns (use "none" to clear)
      --save-request-bodies <BOOL> Whether to also save request bodies
      --json                       Output as JSON
```

## wstraffic — WebSocket Traffic

### wstraffic connections

List all WebSocket connections.

```bash
madhyamas wstraffic connections
```

### wstraffic connection

Get details of a specific WebSocket connection.

```bash
madhyamas wstraffic connection <ID>

Arguments:
  <ID>    WebSocket connection ID
```

### wstraffic messages

List WebSocket messages with optional filtering.

```bash
madhyamas wstraffic messages [OPTIONS]

Options:
      --connection-id <ID>     Filter by connection ID
      --direction <DIR>        Filter by direction (send, receive)
      --message-type <TYPE>    Filter by message type (text, binary, ping, pong, close)
      --search <QUERY>         Search in message payloads
      --limit <LIMIT>          Maximum number of results
      --offset <OFFSET>        Offset for pagination
```

### wstraffic clear

Clear all WebSocket traffic (messages and closed connections).

```bash
madhyamas wstraffic clear
```

## users — User Management (Enterprise)

### users list

List all registered users (enterprise tier).

```bash
madhyamas users list [OPTIONS]

Options:
      --json    Output as JSON
```

### users create

Create a new user account (enterprise tier).

```bash
madhyamas users create [OPTIONS]

Options:
      --username <USERNAME>    Username for the new user
      --email <EMAIL>          Email address
      --password <PASSWORD>    Initial password
      --role <ROLE>            User role (admin, user, viewer)
      --json                   Output as JSON
```

Example: `madhyamas users create --username alice --email alice@example.com --password secret --role user`

### users delete

Delete a user by ID (enterprise tier).

```bash
madhyamas users delete [OPTIONS]

Options:
      --id <ID>    User ID to delete
```

### users update-role

Update a user's role (enterprise tier).

```bash
madhyamas users update-role [OPTIONS]

Options:
      --id <ID>        User ID to update
      --role <ROLE>    New role (admin, user, viewer)
      --json           Output as JSON
```

## audit — Audit Logging (Enterprise)

### audit list

List audit events with optional filters (enterprise tier).

```bash
madhyamas audit list [OPTIONS]

Options:
      --user-id <USER_ID>       Filter by user ID
      --event-type <TYPE>       Filter by event type
      --limit <LIMIT>           Max results [default: 100]
      --json                    Output as JSON
```

### audit export

Export all audit events as JSON (enterprise tier).

```bash
madhyamas audit export
```

### audit stats

Show audit statistics (enterprise tier).

```bash
madhyamas audit stats
```

## license — License Management (Enterprise)

### license info

Show license information (enterprise tier).

```bash
madhyamas license info
```

## auth — Authentication (Enterprise)

### auth login

Login and obtain a JWT token (enterprise tier).

```bash
madhyamas auth login [OPTIONS]

Options:
      --username <USERNAME>    Username
      --password <PASSWORD>    Password
```

Example: `madhyamas auth login --username admin --password mypass`

### auth logout

Logout and invalidate the current session (enterprise tier).

```bash
madhyamas auth logout
```

### auth api-keys list

List all API keys (enterprise tier).

```bash
madhyamas auth api-keys list
```

### auth api-keys create

Create a new API key (enterprise tier).

```bash
madhyamas auth api-keys create [OPTIONS]

Options:
      --name <NAME>      Name for the API key
      --scopes <SCOPES>  Comma-separated scopes [default: *:*]
```

Example: `madhyamas auth api-keys create --name my-key --scopes "*:*"`

### auth api-keys revoke

Revoke an API key by ID (enterprise tier).

```bash
madhyamas auth api-keys revoke [OPTIONS]

Options:
      --id <ID>    API key ID to revoke
```

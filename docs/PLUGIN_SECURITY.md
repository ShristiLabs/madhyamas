# Plugin Security

## Overview

Madhyamas plugins run in a **sandboxed WASM runtime** (`wasmtime`). This
document describes the security model, what plugins can and cannot do, and
the mitigations in place.

## Sandboxing

WASM is sandboxed by design:
- **No filesystem access** — plugins cannot read or write host files.
- **No network access** — plugins cannot make network connections (the
  `http_fetch` host function is **not** linked in v1).
- **No host memory access** — plugins can only access their own linear
  memory, which is allocated by the host.
- **No process spawning** — plugins cannot execute commands or spawn
  processes.

The only host function linked is `log(level, ptr, len)`, which appends a
log line to the invocation's log buffer.

## CPU Limits (Fuel Metering)

Each hook invocation gets a **fuel budget** — a number of WASM instructions
the plugin is allowed to execute. When the budget is exhausted, the
invocation traps and is reported as an error.

- **Default**: 10,000,000 instructions (configurable via `fuel_limit` in
  the manifest)
- **Enforcement**: `wasmtime`'s fuel metering is set per-invocation via
  `Store::set_fuel()`
- **Trap on exhaustion**: the call returns a trap error, which is logged

This prevents infinite loops and CPU-exhaustion attacks.

## Memory Limits

- **Host ceiling**: 256 MiB static linear memory maximum (set via
  `Config::static_memory_maximum_size`)
- **Per-plugin**: `max_memory_pages` in the manifest (default 64 pages =
  4 MiB)
- The bump allocator in the SDK grows memory as needed via
  `memory_grow`, but is capped by the host ceiling.

## Package Integrity

### Checksum Verification

Plugin packages (`.zip`) are verified with **SHA-256** checksums during
installation:

```bash
madhyamas plugins install https://example.com/plugin.zip --checksum abc123...
```

If the checksum doesn't match, installation is aborted. If no checksum is
provided, a warning is logged and the plugin is installed without
verification (the user should always provide a checksum for untrusted
sources).

### Signature Verification (Ed25519)

When a plugin manifest declares a `publisher_public_key` (hex-encoded 32-byte
Ed25519 public key), the installer verifies a `signature.sig` file found
alongside the manifest in the extracted package. The signature is a detached
Ed25519 signature over the **raw zip bytes** (not the extracted files).

- If the signature is valid, `InstallResult.signature_verified` is `true`.
- If a key is declared but no `signature.sig` file is found, the plugin is
  installed with `signature_verified = false` (logged as a warning).
- If a key is declared and a signature file is found but verification fails,
  installation is **aborted** with an error.

To sign a plugin package:

```bash
# 1. Generate a keypair (do this once and store the secret key securely)
madhyamas plugins gen-key

# 2. Add the public_key to your manifest
# publisher_public_key = "<hex public key>"

# 3. Sign the zip package
madhyamas plugins sign my-plugin.zip --secret-key <hex secret key>
# This writes signature.sig alongside the zip

# 4. Include signature.sig in the zip package
```

## Zip-Slip Protection

The installer rejects zip entries that contain `..` components or absolute
paths, preventing the zip-slip directory traversal attack.

## What Plugins Can Do

- Read and modify request/response headers, body, URL, method, status code
- Read plugin settings (configured by the user)
- Maintain per-plugin state (persisted in SQLite)
- Emit log lines (visible in the invocation log)
- Short-circuit requests (return a custom response without forwarding)
- Run periodic timer tasks (when `timer_interval_seconds` is set)

## What Plugins Cannot Do

- Access the filesystem
- Make network connections
- Access host memory outside their linear memory
- Spawn processes or execute commands
- Access other plugins' state (no inter-plugin communication in v1)
- Run for unbounded time (fuel metering caps CPU)

## Recommendations for Plugin Authors

1. **Declare minimal capabilities** — only declare the capabilities your
   plugin actually uses.
2. **Set a low fuel_limit** — if your plugin does simple work, set a low
   fuel limit to bound the worst-case CPU usage.
3. **Set a low max_memory_pages** — default is 64 pages (4 MiB); reduce if
   your plugin uses less.
4. **Publish a checksum** — always provide a SHA-256 checksum when
   distributing your plugin.
5. **Sign your plugin** — when Phase 3 signing is available, sign your
   plugin with an Ed25519 key and publish your public key.

## Recommendations for Users

1. **Only install plugins from trusted sources** — the sandbox prevents
   filesystem/network access, but a malicious plugin could still modify
   your traffic in unwanted ways.
2. **Always provide a checksum** — `madhyamas plugins install <url>
   --checksum <sha256>`.
3. **Review the manifest** — check `hooks`, `capabilities`, and
   `fuel_limit` before enabling a plugin.
4. **Monitor invocation logs** — `madhyamas plugins logs <id>` to see what
   the plugin is doing.
5. **Disable plugins when not needed** — `madhyamas plugins disable <id>`.

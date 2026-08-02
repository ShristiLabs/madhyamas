# Scripting Security Model

This document describes the security model for Madhyamas's scripting system,
the threat model, and the mitigations in place.

## Overview

Madhyamas scripts are written in JavaScript and executed by an embedded
[`boa_engine`] runtime — a pure-Rust ECMAScript engine.  The security model
is based on **sandboxing by construction**: the runtime has no access to the
filesystem, network, or process APIs, and we do not register any host
functions that would expose those capabilities.

## Trust Model

**Scripts are trusted code.**  They are created by the proxy operator (the
person running Madhyamas), not by untrusted third parties.  The operator has
full control over what scripts are loaded, and scripts run with the same
privileges as the proxy process itself — except that the JS sandbox prevents
direct filesystem/network/process access.

This is the same trust model as browser extensions: the user installs them,
and they run in a sandboxed context with limited capabilities.

## Threat Model

| Threat | Mitigation |
|--------|------------|
| Script accesses filesystem | boa has no filesystem APIs; no fs functions are registered |
| Script makes network requests | boa has no network APIs; no network functions are registered |
| Script executes shell commands | boa has no process APIs; no exec functions are registered |
| Script runs forever (infinite loop) | Soft timeout via `timeout_ms` (default 5s) |
| Script consumes excessive memory | `max_memory_bytes` reserved for future enforcement |
| Script accesses other scripts' state | Fresh `Context` per execution — no shared globals |
| Script escapes sandbox | boa is a pure-Rust interpreter with no FFI by default |
| Malicious script in template | Templates are hardcoded in the binary, not user-supplied |
| Script injection via API | API validates script source before storing |

## Sandbox Architecture

### boa_engine

[`boa_engine`](https://github.com/boa-dev/boa) is a pure-Rust ECMAScript
engine.  Key security properties:

1. **No native APIs:** boa does not implement `require()`, `import()`,
   `process`, `fs`, `http`, `net`, or any Node.js/Bun-style APIs.
2. **No FFI:** boa does not expose any foreign function interface by default.
3. **Pure interpreter:** boa is a tree-walking interpreter — there is no JIT
   compilation, so there is no risk of JIT-based code injection.
4. **Memory-safe Rust:** boa is written in safe Rust, so there is no risk of
   memory corruption bugs in the engine itself.

### Host Functions Registered

The following host functions are registered on the JS global object:

| Function | Capabilities | Risk |
|----------|-------------|------|
| `console.log(...)` | Appends to an in-memory array | None — no I/O |
| `base64.encode(str)` | Pure computation | None |
| `base64.decode(str)` | Pure computation | None |
| `crypto.hash(input)` | SHA-256 via `sha2` crate | None — no secret access |
| `url.parse(urlString)` | Pure string parsing | None |
| `url.build(components)` | Pure string construction | None |

**No functions are registered that provide:**
- Filesystem access (read, write, list, delete)
- Network access (HTTP, TCP, UDP, DNS)
- Process access (exec, spawn, kill)
- Environment variable access
- Clipboard access
- Inter-process communication

### Execution Isolation

A fresh `boa_engine::Context` is created for **every** script execution.
This means:

- No shared global state between scripts
- No shared global state between executions of the same script
- No prototype pollution attacks (modifying `Object.prototype` in one script
  does not affect another)
- No memory leaks across executions (the entire context is dropped after each
  execution)

### Timeout Enforcement

The `timeout_ms` configuration (default 5000ms) is enforced as a **soft**
limit.  boa does not support mid-execution preemption (it is a synchronous
interpreter), so the script always runs to completion.  However, if the
execution time exceeds `timeout_ms`, the result is replaced with a timeout
error and the script is marked as failed in the execution history.

This means a malicious script *could* hang the proxy pipeline for up to the
duration of its execution (there is no hard preemption).  This is acceptable
under the trust model (scripts are operator-authored), but operators should
be aware of this when loading third-party scripts.

**Future improvement:** A hard timeout could be implemented by running scripts
in a separate thread with a watchdog timer, but this would add complexity and
is not currently planned.

### Memory Limits

The `max_memory_bytes` configuration (default 10MB) is **reserved for future
enforcement.**  boa does not currently expose a memory limit API, so this
setting is not yet enforced.  In practice, the fresh-context-per-execution
design limits memory usage to a single execution's allocations, which are
dropped immediately after the execution completes.

## Input Validation

### Script Source Validation

Before a script is stored, the API validates:

1. **Non-empty:** The source must not be empty.
2. **Size limit:** The source must not exceed 100KB.
3. **Structural check:** Balanced braces, parentheses, and brackets (fast
   pre-check).
4. **Syntax parse:** The source is parsed by `boa_engine` to verify it is
   valid ECMAScript.  This catches syntax errors before the script is stored.

### API Input Validation

All API endpoints use the `validator` crate to validate input:

- `name`: 1-255 characters
- `source`: minimum 1 character
- `hooks`: array of strings

## Persistence Security

Scripts and execution history are stored in the SQLite database at
`~/.madhyamas/traffic.db`.  The database file has the same permissions as the
Madhyamas process (typically owned by the operator).  No sensitive data is
stored in scripts — but script source code may contain logic that reveals
API keys or tokens if the operator hardcodes them.  **Best practice: do not
hardcode secrets in scripts.**  Use environment variables or external
configuration instead.

## Recommendations for Operators

1. **Review scripts before enabling:** Read the source code of any script
   before enabling it, especially if it came from a third party.
2. **Use the test dialog:** Test scripts in the dry-run dialog before
   enabling them on live traffic.
3. **Set a reasonable timeout:** The default 5s timeout is sufficient for
   most scripts.  Reduce it if you need faster failure detection.
4. **Don't hardcode secrets:** Never put API keys, passwords, or tokens in
   script source code.  They are stored in plaintext in the database.
5. **Disable unused scripts:** Disabled scripts do not execute, reducing the
   attack surface.
6. **Monitor execution history:** Check the history tab for scripts that are
   failing or taking unusually long.

## Recommendations for Developers

1. **Do not register dangerous host functions:** If you add new host
   functions to `engine.rs`, ensure they do not provide filesystem, network,
   or process access.
2. **Keep the fresh-context-per-execution design:** Do not share
   `boa_engine::Context` between executions — this is a critical security
   property.
3. **Test with malicious inputs:** When adding new features, test with
   scripts that attempt to access restricted APIs (they should fail with
   `ReferenceError` or `TypeError`).
4. **Audit dependencies:** `boa_engine` and `sha2` are the only
   scripting-related dependencies.  Audit them regularly for security
   advisories.

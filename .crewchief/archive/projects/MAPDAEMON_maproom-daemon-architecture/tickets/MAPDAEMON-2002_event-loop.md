# MAPDAEMON-2002: The Event Loop & Ping

**Status:** Open
**Phase:** 2 (Core)
**Estimated Effort:** 90 minutes
**Priority:** High

---

## Summary
Implement the core event loop that reads from `stdin`, parses JSON-RPC requests, dispatches them, and writes responses to `stdout`. Implement the `ping` method to verify connectivity.

---

## Background
With the scaffolding in place, we need the actual "engine" of the daemon. This is an infinite loop that processes lines of text from standard input. It must be asynchronous (`tokio`) to handle I/O efficiently without blocking.

---

## Acceptance Criteria
1.  ✅ `crewchief-maproom serve` runs indefinitely until `stdin` is closed (EOF).
2.  ✅ Sending `{"jsonrpc": "2.0", "method": "ping", "id": 1}` to stdin results in `{"jsonrpc": "2.0", "result": "pong", "id": 1}` on stdout.
3.  ✅ Invalid JSON returns a standard JSON-RPC parse error.
4.  ✅ Unknown methods return a "Method not found" error.
5.  ✅ **Crucial:** No logs are printed to stdout (only stderr).

---

## Technical Requirements

### 1. The Loop
Use `tokio::io::BufReader` on `tokio::io::stdin()`.
```rust
let stdin = tokio::io::stdin();
let reader = BufReader::new(stdin);
let mut lines = reader.lines();

while let Ok(Some(line)) = lines.next_line().await {
    // Process line
}
```

### 2. Dispatcher
A simple match statement on `request.method`.
*   `"ping"` -> return "pong".
*   `_` -> return Error -32601 (Method not found).

### 3. Output
Use `tokio::io::stdout()`. Ensure we append a newline `\n` after every response so the client knows the message is complete. Flush after write.

### 4. Error Handling
If `serde_json::from_str` fails, return Error -32700 (Parse error).

---

## Implementation Steps
1.  Implement `daemon::run()` to start the tokio loop.
2.  Implement the line reading logic.
3.  Implement the request parsing and error handling.
4.  Implement the `handle_request` function with the `ping` case.
5.  Implement the response serialization and writing to stdout.

---

## Verification
*   Run `cargo run -- serve`.
*   Type `{"jsonrpc": "2.0", "method": "ping", "id": 1}` and hit Enter.
*   Verify response.
*   Hit Ctrl+D (EOF) and verify process exits.

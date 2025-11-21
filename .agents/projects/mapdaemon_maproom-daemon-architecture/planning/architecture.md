# Architecture: Maproom Daemon

## Architectural Goals
1.  **Persistence:** Maintain a single process lifecycle to enable resource reuse (DB connections, caches).
2.  **Performance:** Minimize per-request overhead.
3.  **Standardization:** Use a standard protocol (JSON-RPC 2.0) for communication.
4.  **Simplicity:** Avoid complex networking (ports, sockets) for local-only IPC.

## System Design

### 1. The `serve` Command
A new subcommand `serve` will be added to the `crewchief-maproom` binary.
```bash
crewchief-maproom serve
```
When executed, this command enters a long-running loop, listening on `stdin` and writing to `stdout`. Logs should be directed to `stderr` to avoid corrupting the JSON output stream.

### 2. Communication Protocol: JSON-RPC 2.0 over Stdio
We will use the JSON-RPC 2.0 specification.

**Request:**
```json
{
  "jsonrpc": "2.0",
  "method": "search",
  "params": {
    "query": "vector search term",
    "limit": 10
  },
  "id": 1
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": [ ... results ... ],
  "id": 1
}
```

**Error:**
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32600,
    "message": "Invalid Request"
  },
  "id": null
}
```

### 3. Internal Components

#### `DaemonState`
A shared state container wrapped in `Arc`.
```rust
struct DaemonState {
    pool: PgPool, // Connection pool
    // Future: embedding_cache, etc.
}
```

#### `RpcLoop`
The main event loop:
1.  Initialize `DaemonState` (connect to DB).
2.  Loop:
    *   Read line from `stdin`.
    *   Parse as JSON-RPC request.
    *   Dispatch to handler based on `method`.
    *   Serialize response.
    *   Write line to `stdout`.

### 4. Concurrency Model
*   **Runtime:** `tokio`
*   **Handling:** Requests can be processed concurrently.
    *   The main loop reads stdin.
    *   On valid request, `tokio::spawn` a handler task.
    *   Handler task processes request (DB query) and writes response to a shared output channel (MPSC) which the main loop writes to stdout (to ensure thread-safe writing).
    *   *Alternative for MVP:* Process sequentially in the loop if search is fast enough, but async concurrent handling is preferred for scalability.

### 5. Methods (MVP)
*   `ping`: Returns "pong". Used for health checks and keeping the connection alive.
*   `search`: Executes a vector search. Params mirror the existing CLI arguments.

## Technology Choices
*   **Protocol:** JSON-RPC 2.0 (Standard, easy to implement).
*   **Transport:** Stdio (Simple, secure for local IPC, no firewall issues).
*   **Database:** `sqlx` (Already used, supports pooling).
*   **Serialization:** `serde` + `serde_json`.

## Constraints & Trade-offs
*   **Logs:** All logging **MUST** go to `stderr`. Any output to `stdout` will break the JSON-RPC protocol parser on the client side. We must configure the logger (tracing/env_logger) to strictly use stderr.
*   **State:** For now, state is just the DB pool. In the future, we can add in-memory caches.
*   **Client:** This project does *not* update the Node.js client. That is a future integration step. We are building the capability first.

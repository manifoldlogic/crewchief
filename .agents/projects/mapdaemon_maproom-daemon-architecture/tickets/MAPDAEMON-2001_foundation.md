# MAPDAEMON-2001: Foundation & Scaffolding

**Status:** Open
**Phase:** 1 (Foundation)
**Estimated Effort:** 45 minutes
**Priority:** High

---

## Summary
Establish the project structure for the daemon architecture. Create the `daemon` module, define the JSON-RPC data types using `serde`, and register the `serve` subcommand in the CLI entry point.

---

## Background
We are transitioning `crewchief-maproom` to support a persistent daemon mode. This requires a new module to house the server logic and strict type definitions for the JSON-RPC 2.0 protocol we will use for communication.

---

## Acceptance Criteria
1.  ✅ `crates/maproom/src/daemon/mod.rs` exists.
2.  ✅ JSON-RPC types (`Request`, `Response`, `Error`, `ErrorCode`) are defined and serializable/deserializable via `serde`.
3.  ✅ `crewchief-maproom --help` shows the new `serve` subcommand.
4.  ✅ Running `crewchief-maproom serve` prints a placeholder message (e.g., "Daemon mode starting...") and exits (for now).

---

## Technical Requirements

### 1. Module Structure
Create `crates/maproom/src/daemon/` with:
*   `mod.rs`: Module definition.
*   `types.rs`: JSON-RPC struct definitions.

### 2. JSON-RPC Types
Implement the standard JSON-RPC 2.0 envelope:
```rust
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String, // Must be "2.0"
    pub method: String,
    pub params: Option<serde_json::Value>,
    pub id: Option<serde_json::Value>, // ID can be number, string, or null
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<JsonRpcError>,
    pub id: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}
```

### 3. CLI Registration
Update `crates/maproom/src/main.rs` (or `cli.rs` if separated) to add the `Serve` variant to the `Commands` enum using `clap`.

---

## Implementation Steps
1.  Create the directory `crates/maproom/src/daemon`.
2.  Create `types.rs` with the structs above.
3.  Create `mod.rs` exposing the types and a public `run()` function (placeholder).
4.  Update `main.rs` to include `mod daemon;`.
5.  Update `clap` definition to add `serve` command.
6.  Match on the `serve` command in `main` and call `daemon::run()`.

---

## Verification
*   Run `cargo build`.
*   Run `./target/debug/crewchief-maproom serve` and verify output.

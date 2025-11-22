# MAPDAEMON-2003: Vector Search Integration

**Status:** Open
**Phase:** 3 (Feature)
**Estimated Effort:** 90 minutes
**Priority:** High

---

## Summary
Integrate the existing `VectorExecutor` into the daemon to enable actual search functionality. This involves setting up the database connection pool and implementing the `search` JSON-RPC method.

---

## Background
The daemon currently only responds to `ping`. To replace the CLI for search, it needs to expose the vector search logic. We will reuse the existing `VectorExecutor` struct but initialize it once at startup and share it across requests.

---

## Acceptance Criteria
1.  ✅ Daemon connects to PostgreSQL on startup using `DATABASE_URL`.
2.  ✅ `search` method accepts `query`, `limit`, etc.
3.  ✅ `search` returns a list of results matching the CLI output format (but in JSON).
4.  ✅ Database connection is reused across multiple search requests (connection pooling).

---

## Technical Requirements

### 1. Daemon State
Define a struct to hold the shared state:
```rust
struct DaemonState {
    executor: VectorExecutor, // Or whatever holds the PgPool
}
```
Initialize this state *before* entering the request loop.

### 2. Search Method
Implement handler for `method: "search"`.
*   **Params:** Define a struct `SearchParams` that derives `Deserialize`.
    *   `query`: String
    *   `limit`: Option<i64>
    *   `threshold`: Option<f32>
    *   `tags`: Option<Vec<String>>
*   **Logic:** Call `state.executor.search(...)`.
*   **Response:** Serialize the `Vec<SearchResult>` to JSON.

### 3. Concurrency
Since `search` is async and involves DB I/O, we should ideally spawn a task for it so we don't block the reading loop (though for MVP sequential is acceptable if simpler).
*   *Recommendation:* Keep it sequential for this ticket to ensure stability, unless performance testing shows blocking is an issue.

---

## Implementation Steps
1.  Update `daemon::run` to initialize the DB pool/executor.
2.  Pass `Arc<DaemonState>` to the request handler.
3.  Define `SearchParams` struct.
4.  Implement the `search` case in the dispatcher.
5.  Map the `VectorExecutor` results to `serde_json::Value`.

---

## Verification
*   Start daemon with valid `DATABASE_URL`.
*   Send search request: `{"jsonrpc": "2.0", "method": "search", "params": {"query": "test", "limit": 1}, "id": 1}`.
*   Verify valid search results are returned.

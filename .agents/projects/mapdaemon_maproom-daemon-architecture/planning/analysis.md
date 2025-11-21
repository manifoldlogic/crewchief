# Analysis: Maproom Daemon Architecture

## Problem Definition
The current Maproom architecture relies on the MCP server spawning a new Rust process (`crewchief-maproom`) for every single search request. This "ephemeral process" model introduces significant performance bottlenecks and architectural limitations:

1.  **Process Overhead:** Spawning a new OS process for each request adds latency (tens to hundreds of milliseconds), which is perceptible in interactive search scenarios.
2.  **No Connection Pooling:** Each process must establish a new connection to the PostgreSQL database. This is expensive and limits scalability under load.
3.  **No Shared State/Caching:** There is no persistent memory state between requests. Caching frequently accessed data (like embeddings or index metadata) is impossible across requests.
4.  **Resource Churn:** High frequency of process creation/destruction puts unnecessary load on the OS and database.

## Context & Current State
*   **Current Implementation:** The `crewchief-maproom` binary is designed primarily as a CLI tool. It executes a command and exits.
*   **MCP Integration:** The Node.js MCP server uses `child_process.spawn` (or similar) to invoke the CLI.
*   **Database:** Uses `pgvector` for vector search. Establishing a Postgres connection involves TCP handshakes and authentication, which is costly to repeat.

## Proposed Solution: Persistent Daemon
Transition the Rust binary to support a long-running "daemon" or "server" mode.

*   **New Command:** `crewchief-maproom serve` (or similar).
*   **Communication:** The daemon will listen for requests. For the MVP, this could be via:
    *   **Standard IO (stdio):** Keeping the process open and communicating via JSON-RPC or a simple line-based protocol over stdin/stdout. This is easiest to integrate with the existing MCP server structure (which already spawns a process, just needs to keep it alive).
    *   **HTTP/TCP:** Listening on a local port. This is more standard for "services" but introduces port management complexity.
    *   **Unix Domain Sockets:** Good for local IPC, but platform-specific (Windows support varies).

**Decision:** For this phase, **Stdio JSON-RPC** or a simple **Line-based JSON** protocol over a persistent process is likely the simplest and most robust first step. It avoids port conflicts and firewall issues. The MCP server starts the process once and sends multiple requests over stdin.

## Industry Standards
*   **LSP (Language Server Protocol):** Uses JSON-RPC over stdio to communicate between editors and language servers. This is a proven model for local tool integration.
*   **Database Drivers:** Maintain connection pools to avoid handshake overhead.
*   **Search Engines (Elasticsearch, Solr):** Always run as persistent services to manage caches and file handles.

## Research Findings
*   **Rust Async Runtime:** We are already using `tokio`. It is well-suited for handling a long-running server loop.
*   **State Management:** We can use `Arc<RwLock<...>>` or similar patterns to share state (database pool, caches) across the application lifespan.
*   **Connection Pooling:** `sqlx` (or `tokio-postgres` with `deadpool`) supports connection pooling out of the box. We just need to initialize the pool once at startup.

## Conclusion
Moving to a daemon architecture is a critical optimization. It transforms Maproom from a "script runner" into a true "search engine" capable of high-performance, low-latency operations. The primary value is **latency reduction** and **resource efficiency**.

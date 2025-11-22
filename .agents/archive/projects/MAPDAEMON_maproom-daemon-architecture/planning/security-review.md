# Security Review: Maproom Daemon

## Architecture Security Analysis
The proposed architecture moves from a CLI execution model to a long-running daemon communicating via Standard IO (stdin/stdout).

### Attack Surface
*   **Input:** `stdin` stream receiving JSON-RPC messages.
*   **Output:** `stdout` stream sending JSON responses.
*   **State:** Database connection (PostgreSQL).

### Threat Model
1.  **Malicious Input (DoS/Crash):**
    *   **Threat:** Sending extremely large or malformed JSON to crash the daemon.
    *   **Mitigation:** `serde_json` limits recursion depth. We should also enforce a maximum line length (e.g., 1MB) for requests if using line-based reading.
2.  **Injection Attacks:**
    *   **Threat:** SQL injection via search parameters.
    *   **Mitigation:** The existing `VectorExecutor` uses `sqlx` with parameterized queries. This must be maintained. The JSON-RPC params are strongly typed structs.
3.  **Unauthorized Access:**
    *   **Threat:** Another user on the system accessing the daemon.
    *   **Mitigation:** The daemon uses **Stdio** for communication. It does not open any network ports. Only the process that spawned it (the MCP server) has access to its stdin/stdout pipes. This leverages OS-level process isolation.

## Known Gaps & Risks
*   **Logging Leakage:** If sensitive query data is logged to stderr, it might be visible in system logs.
    *   **Mitigation:** Ensure production logging levels are appropriate (INFO/WARN) and do not log full query payloads unless DEBUG is enabled.
*   **Denial of Service:** A heavy search query could lock up the database connection pool.
    *   **Mitigation:** Use database timeouts and connection pool limits.

## MVP Mitigations
*   **Strict Transport:** Stick to Stdio. Do not implement TCP/HTTP listeners in this phase.
*   **Input Limits:** Use `tokio::io::AsyncBufReadExt::lines()` which handles line splitting, but be aware of memory limits for very long lines.
*   **Graceful Error Handling:** Ensure no panic on bad input; return JSON-RPC error objects instead.

## Conclusion
The Stdio-based daemon architecture is inherently secure for local IPC. The primary security focus is ensuring robust input parsing and preventing resource exhaustion.

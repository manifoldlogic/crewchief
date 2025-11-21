# Architecture: Maproom Daemon Architecture

## Architecture Decisions
- **Command**: New `serve` command in CLI.
- **Communication**: JSON-RPC 2.0 over Standard I/O (Stdin/Stdout). This allows easy integration with Node.js `child_process` later.
- **State Management**: The daemon will hold a `PgPool` (SQLx connection pool) and potentially an in-memory cache of recent results or compiled queries.

## Technology Choices
- **Tokio**: Async runtime (already used).
- **Tower-LSP** or similar (optional, or custom JSON-RPC loop): To handle the request/response cycle.
- **SQLx**: For connection pooling.

## Performance Considerations
- **Memory Usage**: Monitor memory usage to ensure the daemon doesn't bloat over time.
- **Concurrency**: Handle multiple requests if we move to sockets later, but for stdio, it's likely serial or pipelined.

## Constraints
- Must be robust against malformed input (don't crash the daemon).

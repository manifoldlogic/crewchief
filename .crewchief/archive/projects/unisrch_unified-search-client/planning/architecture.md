# Architecture: Unified Search Client

## Architecture Decisions
- **Delegation Pattern**: The MCP server will not implement search algorithms. It will construct a CLI command string and execute the Rust binary.
- **Interface**: The MCP tool `search` will accept arguments that map directly to the Rust CLI arguments.
- **Error Handling**: Capture stderr from the Rust process and bubble it up as MCP errors.

## Technology Choices
- **Node.js**: Existing runtime for MCP.
- **Child Process**: Standard library for spawning processes.
- **Zod**: For validation of inputs before passing to CLI.

## Performance Considerations
- **Process Overhead**: Spawning a process per request has overhead. This is a known trade-off for this phase (to be addressed by `MAPDAEMON` later).
- **Latency**: acceptable for human-speed interactions (agent queries).

## Constraints
- Rust binary must be in the path or at a known location relative to the MCP server.

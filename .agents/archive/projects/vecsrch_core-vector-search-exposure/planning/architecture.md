# Architecture: Core Vector Search Exposure

## Architecture Decisions
- **CLI Integration**: Extend the existing `clap` CLI definition in `crates/maproom/src/cli/` to include a `vector-search` (or simply `search` with a flag) command.
- **Direct Invocation**: The CLI handler will instantiate `VectorExecutor` directly. No intermediate layers or daemon processes are required for this phase.
- **Output Format**: Output should be structured (JSON) to be easily consumable by the MCP layer later.

## Technology Choices
- **Rust**: Leveraging the existing codebase.
- **Clap**: For CLI argument parsing (standard in Rust ecosystem).
- **Serde**: For JSON output serialization.

## Performance Considerations
- **Initialization**: Loading the vector index might take time. For this CLI-based approach, we accept the startup cost per command invocation. (The `MAPDAEMON` project will address this later).
- **Memory**: Ensure the index loading doesn't OOM on standard dev machines.

## Constraints
- Must not break existing CLI commands.
- Must work with the existing PostgreSQL setup for `pgvector`.

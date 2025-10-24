# Ticket: MCP_CORE-1003: Upsert Tool Implementation

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- mcp-tools-engineer
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Implement the Upsert tool for the Maproom MCP server, enabling clients to trigger re-indexing of specific files or paths in the semantic search database. This tool provides a critical bridge between file changes and index updates, allowing AI assistants and other clients to keep the search index synchronized with code modifications.

## Background
The Maproom MCP server provides semantic code search capabilities via multiple tools. The Upsert tool is one of the core supporting tools (Phase 1, Week 2, Supporting Tools, Task 1) that enables clients to update the search index when files are modified. This is essential for maintaining index freshness and ensuring search results reflect current code state.

The tool acts as a wrapper around the `crewchief-maproom` Rust binary's upsert command, handling process spawning, progress tracking, error capture, and result formatting in a way that's accessible to MCP clients.

## Acceptance Criteria
- [ ] Process spawning working - successfully spawns crewchief-maproom binary with correct arguments
- [ ] Progress tracking functional - captures and reports indexing progress from binary output
- [ ] Errors captured and formatted - handles process errors and returns formatted error responses
- [ ] Results returned correctly - returns UpsertResult with updated_files, updated_chunks, and duration_ms
- [ ] Input validation - validates paths array, commit string, and worktree string parameters
- [ ] Unit tests passing - comprehensive tests for success cases, error cases, and edge cases

## Technical Requirements

### Input Schema (Zod)
- `paths`: array of strings (file or directory paths to re-index)
- `commit`: string (git commit hash for context)
- `worktree`: string (worktree identifier for isolation)

### Output Schema
```typescript
interface UpsertResult {
  updated_files: number;
  updated_chunks: number;
  duration_ms: number;
}
```

### Process Spawning
- Spawn `crewchief-maproom` binary with `upsert` subcommand
- Pass arguments: `--paths <comma-separated>`, `--commit <hash>`, `--worktree <name>`
- Capture stdout for progress tracking
- Capture stderr for error messages
- Handle process exit codes

### Error Handling
- Process spawn failures
- Binary not found errors
- Invalid path errors from indexer
- Database connection errors from indexer
- Timeout handling for long-running upserts

## Implementation Notes

### Architecture Reference
See `/workspace/crewchief_context/maproom/MCP_CORE/MCP_CORE_ARCHITECTURE.md` (lines 106-127) for the Upsert tool architecture design.

### Process Utilities Module
Create a reusable `process.ts` utility module for spawning and managing child processes. This will be used by multiple tools (upsert, scan, etc.) and should provide:
- Process spawning with argument validation
- Stream handling (stdout/stderr)
- Progress parsing from output
- Timeout support
- Graceful termination

### Progress Tracking
The crewchief-maproom binary outputs progress information to stdout during indexing. Parse this output to provide real-time feedback to clients. Expected output format includes:
- File counts
- Chunk counts
- Completion percentage
- Duration metrics

### Binary Location
The `crewchief-maproom` binary should be located via:
1. Environment variable (e.g., `MAPROOM_BINARY_PATH`)
2. Platform-specific default paths in `packages/cli/bin/<platform>/`
3. System PATH

### Testing Strategy
- Unit tests with mocked process spawning
- Integration tests with actual binary (if available in test environment)
- Error case coverage (missing binary, invalid paths, etc.)
- Performance tests for large file sets

## Dependencies

### External Dependencies
- **crewchief-maproom binary** (Rust indexer) - Required for actual upsert operations
  - Must be built and available in expected location
  - See `crates/maproom/` for source code
  - Build with `cargo build --release --bin crewchief-maproom`

### Internal Dependencies
- Node.js child_process module for process spawning
- Zod for parameter validation
- MCP tool handler base classes/interfaces (from Phase 1, Week 1)

### Prerequisite Tickets
- MCP server base implementation (Phase 1, Week 1)
- Tool handler interface/base classes
- Database connection setup

## Risk Assessment

- **Risk**: Binary not available in expected location on different platforms
  - **Mitigation**: Implement flexible binary discovery with multiple fallback locations; provide clear error messages guiding users to build or install binary

- **Risk**: Long-running upsert operations may timeout or block
  - **Mitigation**: Implement timeout handling; consider async/streaming responses for large operations; provide progress updates

- **Risk**: Process spawning failures or crashes
  - **Mitigation**: Comprehensive error handling; graceful degradation; clear error messages with troubleshooting guidance

- **Risk**: Path validation complexity (relative vs absolute, worktree-relative, etc.)
  - **Mitigation**: Defer detailed path validation to the Rust binary; provide basic sanity checks in TypeScript layer; document expected path formats

## Files/Packages Affected

### Files to Create
- `packages/maproom-mcp/src/tools/upsert.ts` - Main upsert tool handler implementation
- `packages/maproom-mcp/src/tools/upsert_schema.ts` - Zod schema for input/output validation
- `packages/maproom-mcp/src/utils/process.ts` - Reusable process spawning utilities
- `packages/maproom-mcp/tests/tools/upsert_test.ts` - Unit tests for upsert tool

### Files to Modify
- `packages/maproom-mcp/src/tools/index.ts` - Register upsert tool in tool registry
- `packages/maproom-mcp/src/server.ts` - Wire up upsert tool in MCP server (if needed)
- `packages/maproom-mcp/package.json` - Add any new dependencies

### Packages Affected
- `@crewchief/maproom-mcp` - Primary package for this implementation

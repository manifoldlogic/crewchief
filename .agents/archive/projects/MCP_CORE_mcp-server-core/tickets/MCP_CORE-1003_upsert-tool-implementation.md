# Ticket: MCP_CORE-1003: Upsert Tool Implementation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

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
- [x] Process spawning working - successfully spawns crewchief-maproom binary with correct arguments
- [x] Progress tracking functional - captures and reports indexing progress from binary output
- [x] Errors captured and formatted - handles process errors and returns formatted error responses
- [x] Results returned correctly - returns UpsertResult with updated_files, updated_chunks, and duration_ms
- [x] Input validation - validates paths array, commit string, and worktree string parameters
- [x] Unit tests passing - comprehensive tests for success cases, error cases, and edge cases

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

## Implementation Notes

### Completed Implementation

All acceptance criteria have been met. The implementation includes:

1. **Process Utilities Module** (`src/utils/process.ts`):
   - Reusable process spawning infrastructure with timeout support
   - Binary discovery with multiple fallback strategies (env var, platform paths, dev builds, system PATH)
   - Stream handling for stdout/stderr capture
   - Progress parsing from indexer output
   - Comprehensive error handling with ProcessError class

2. **Upsert Tool Handler** (`src/tools/upsert.ts`):
   - Full implementation following MCP best practices
   - Input validation with Zod schemas
   - Path security validation to prevent traversal attacks
   - Process spawning with configurable timeout (default 2 minutes)
   - Result parsing from binary output with fallback defaults
   - Error formatting for MCP protocol compliance

3. **Type Definitions** (`src/types.ts`):
   - UpsertParams interface (paths, commit, repo, worktree, root)
   - UpsertResult interface (updated_files, updated_chunks, duration_ms)
   - UpsertToolConfig interface (timeout, env)

4. **Zod Schemas** (`src/tools/upsert_schema.ts`):
   - UpsertParamsSchema with comprehensive validation
   - UpsertResultSchema for output validation
   - Validation helper functions

5. **Integration** (`src/index.ts`):
   - Updated handleUpsert to use new tool handler
   - Added error handling with formatUpsertError
   - Proper MCP protocol response formatting

6. **Comprehensive Tests** (`tests/tools/upsert.test.ts`):
   - 41 unit tests covering all requirements
   - Parameter validation tests (empty arrays, empty strings, required fields)
   - Path security tests
   - Indexing stats parsing tests (multiple output formats)
   - Binary discovery tests
   - Process error handling tests
   - Edge cases (single path, many paths, special characters, ANSI codes)
   - All tests passing

### Architecture Decisions

- **Process Utilities as Reusable Module**: Created `process.ts` as a separate utility module to support future tools (scan, explain) that will also need process spawning
- **Binary Discovery Strategy**: Implemented multi-level fallback (env var → platform binaries → dev builds → system PATH) for maximum flexibility across environments
- **Progress Parsing**: Used regex patterns that support multiple output formats from the Rust binary for robustness
- **Error Enhancement**: Added specific error handling for common cases (BINARY_NOT_FOUND, TIMEOUT, ENOENT) with helpful troubleshooting messages
- **Path Validation**: Leveraged existing validation.ts utilities for security, consistent with Open tool implementation
- **Timeout Configuration**: Made timeout configurable via UpsertToolConfig, with sensible 2-minute default

### Testing Coverage

All 41 tests pass, covering:
- ✓ Parameter validation (required fields, empty values, array constraints)
- ✓ Path security (traversal attempts, relative paths, directories)
- ✓ Indexing stats parsing (complete output, alternative phrasing, missing stats, edge cases)
- ✓ Binary discovery (env var, fallbacks, candidates)
- ✓ Process error handling (all error codes and messages)
- ✓ Edge cases (single/many paths, long hashes, special characters, ANSI codes)

### Files Created

- `/workspace/packages/maproom-mcp/src/utils/process.ts` - 330 lines
- `/workspace/packages/maproom-mcp/src/tools/upsert.ts` - 190 lines
- `/workspace/packages/maproom-mcp/src/tools/upsert_schema.ts` - 50 lines
- `/workspace/packages/maproom-mcp/tests/tools/upsert.test.ts` - 500+ lines

### Files Modified

- `/workspace/packages/maproom-mcp/src/types.ts` - Added UpsertParams, UpsertResult, UpsertToolConfig interfaces
- `/workspace/packages/maproom-mcp/src/index.ts` - Updated handleUpsert function with proper error handling

### Build & Test Status

- ✓ TypeScript compilation successful (`pnpm build`)
- ✓ All 41 unit tests passing (`pnpm test tests/tools/upsert.test.ts`)
- ✓ No dependencies added (used existing: child_process, pino, zod)

### Ready for Next Steps

The implementation is complete and ready for:
1. Test runner verification (unit-test-runner agent)
2. Ticket verification (verify-ticket agent)
3. Integration testing with actual crewchief-maproom binary
4. Commit and merge (commit-ticket agent)

# Ticket: SEMRANK-0001: Create TypeScript Search MCP Tool

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

## Test Execution Evidence

```bash
$ pnpm vitest run tests/search_tool.test.ts

RUN  v1.6.1 /workspace/packages/maproom-mcp

 ✓ tests/search_tool.test.ts  (41 tests | 4 skipped) 22ms

 Test Files  1 passed (1)
      Tests  37 passed | 4 skipped (41)
   Start at  14:04:02
   Duration  478ms (transform 98ms, setup 0ms, collect 228ms, tests 22ms, environment 0ms, prepare 108ms)
```

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement `/packages/maproom-mcp/src/tools/search.ts` as MCP tool wrapper around Rust FTS implementation to enable semantic ranking enhancements.

## Background
Project review identified that search.ts MCP tool does not exist. The Rust FTS implementation exists at `/crates/maproom/src/search/fts.rs` (lines 77-99), but there is no TypeScript MCP tool to expose this functionality via the MCP protocol. This is a critical blocker - Phase 2 tickets assume this tool exists and will be used to implement semantic ranking enhancements. Without this tool, the entire SEMRANK project cannot proceed.

This ticket implements Phase 0 (MCP Tool Creation & Baseline) from the SEMRANK project plan.

## Acceptance Criteria
- [x] File `/packages/maproom-mcp/src/tools/search.ts` created and functional
- [x] MCP tool accepts parameters: query (string), repo_filter (optional string), worktree (optional string), limit (number, default 20), debug (boolean, default false)
- [x] Tool calls Rust FTS implementation via subprocess/RPC and returns results
- [x] Returns: Array of chunks with { chunk_id, symbol_name, kind, relpath, preview, score }
- [x] MCP protocol integration validated - tool appears in MCP server tool list
- [x] Basic error handling for missing parameters and Rust call failures

## Technical Requirements
- Follow existing MCP tool patterns from `open.ts`, `context.ts`, `status.ts`
- Use Zod for parameter validation
- Call Rust binary at `packages/cli/bin/<platform>/crewchief-maproom`
- Parse JSON-RPC response from Rust FTS search
- Handle subprocess spawning and stdout/stderr parsing
- Return MCP-compliant tool response format

## Implementation Notes

**CRITICAL ARCHITECTURAL REQUIREMENT:**
This tool MUST call the Rust binary via subprocess to avoid code duplication. The current TypeScript implementation in `index.ts` (lines 684-850) has duplicate SQL logic that should be replaced.

**Why Call Rust Binary:**
- Single source of truth for search logic (avoid maintaining FTS queries in two languages)
- Consistency with other tools (upsert, scan all call Rust)
- Phase 2 modifications only need to update Rust, not TypeScript
- CLI, VSCode, and MCP all use same implementation

**Implementation Pattern:**
- Reference `packages/maproom-mcp/src/tools/upsert.ts` for subprocess calling pattern
- Use `getBinaryPath()` from utils/process.ts to locate binary
- Spawn with args: `['search', '--query', query, '--repo', repo, '--limit', limit, ...]`
- Parse NDJSON output from stdout (one JSON object per line)
- Handle stderr for error logging
- Use existing error handling from upsert.ts pattern

**Rust Binary Details:**
- Location: `packages/cli/bin/<platform>/crewchief-maproom`
- Command: `crewchief-maproom search --query "..." --repo "..." --limit N`
- Output format: NDJSON (newline-delimited JSON)
- Current Rust FTS scoring: `ts_rank_cd() + 0.2 bonus if symbol ILIKE '%query%'`
- Debug mode: `--debug` flag returns raw score breakdown

**Do NOT:**
- Call SQL directly from TypeScript (creates duplicate logic)
- Reuse `handleSearch()` from index.ts (this is the duplication we're eliminating)
- Create database connections in search.ts (Rust binary handles this)

**Example Tool Call:**
```typescript
await search({
  query: "validate_provider",
  repo_filter: "crewchief",
  limit: 10,
  debug: true
});
```

**Expected Response Format:**
```typescript
{
  results: [
    {
      chunk_id: "uuid-here",
      symbol_name: "validate_provider",
      kind: "function",
      relpath: "src/auth/provider.ts",
      preview: "function validate_provider(config: Config) { ... }",
      score: 0.85
    },
    // ...more results
  ]
}
```

## Dependencies
None - this is the first ticket in the SEMRANK project and enables all subsequent work.

## Risk Assessment
- **Risk**: Rust binary calling may have platform-specific issues
  - **Mitigation**: Test on Mac/Linux, use conditional platform detection for binary path
- **Risk**: JSON-RPC protocol mismatch between TypeScript and Rust
  - **Mitigation**: Validate against Rust FTS output format, add comprehensive error handling
- **Risk**: MCP protocol changes or integration issues
  - **Mitigation**: Follow existing tool patterns closely (open.ts, context.ts, status.ts)

## Files/Packages Affected
- **Create**: `/packages/maproom-mcp/src/tools/search.ts`
- **Modify**: `/packages/maproom-mcp/src/index.ts` (register new tool)
- **Reference**:
  - `/packages/maproom-mcp/src/tools/open.ts` (pattern reference)
  - `/packages/maproom-mcp/src/tools/context.ts` (pattern reference)
  - `/packages/maproom-mcp/src/indexer.ts` (Rust binary calling)
  - `/crates/maproom/src/search/fts.rs` (Rust implementation)

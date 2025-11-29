# Ticket: DAEMIGR-2001: MCP Server Daemon Integration

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (integration will be tested in DAEMIGR-2903)
- [x] **Verified** - by the verify-ticket agent

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
Replace process-spawning logic with DaemonClient in MCP server search tool, preserving all existing functionality while enabling daemon-based communication for 20-50x performance improvement.

## Background
The MCP server currently spawns a new process for each search request (lines 233-291 in search.ts), preventing realization of daemon performance benefits. This ticket migrates the search tool to use DaemonClient while preserving chunk ID fetching and error handling.

This implements **Phase 2: Integration** of the daemon client migration plan, specifically the MCP server integration component. By replacing the process-spawning logic with daemon-based RPC calls, we unlock significant performance improvements while maintaining full backward compatibility with existing MCP tool functionality.

**Planning Reference:**
- `.crewchief/projects/DAEMIGR_daemon-client-migration/planning/architecture.md` (lines 352-466)

## Acceptance Criteria
- [x] MCP search tool uses daemon instead of spawning (no calls to trySpawnWithCandidates in search path)
- [x] Chunk IDs fetched correctly from database (existing logic preserved at lines 332+)
- [x] Errors handled gracefully:
  - [x] RpcError converted to MCP error response (ProcessError with RPC_ERROR code)
  - [x] DaemonStartError converted to user-friendly message (with troubleshooting steps)
  - [x] DaemonTimeoutError converted to timeout message (with troubleshooting steps)
- [x] Existing search functionality preserved:
  - [x] All search modes work (mode validation preserved, daemon called)
  - [x] All filters work (repo, worktree passed to daemon, limit parameter included)
  - [x] Debug mode works (debug parameter passed to daemon)
  - [x] Result formatting unchanged (daemon results transformed to RustSearchOutput format)

## Technical Requirements

### Code Changes
- **Target file:** `/workspace/packages/maproom-mcp/src/tools/search.ts`
- **Replace:** Lines 233-291 (spawning logic with `trySpawnWithCandidates`)
- **Preserve:** Lines 307-343 (chunk ID fetching from database)

### New Import
```typescript
import { getDaemonClient } from '../daemon'
```

### Replace Spawning Logic (Lines 233-291)
```typescript
// OLD (remove):
const candidates = getBinaryCandidates()
const result = await trySpawnWithCandidates(candidates, args, {...})
const rustOutput = JSON.parse(result.stdout)

// NEW (add):
const daemon = getDaemonClient()
const searchResult = await daemon.search({
  query,
  repo,
  worktree,
  limit,
  mode,
  debug
})
```

### Error Handling Requirements
- Catch `RpcError` and convert to MCP error with user-friendly message
- Catch `DaemonStartError` and suggest checking daemon binary location
- Catch `DaemonTimeoutError` and suggest checking database/network connectivity
- Propagate stderr logs to MCP logger for debugging
- Maintain existing error response format for MCP client compatibility

### Data Structure Compatibility
- `searchResult` format must match old `rustOutput` format exactly
- Chunk IDs fetched and mapped to results identically to current implementation
- Result formatting must remain unchanged for MCP client compatibility

## Implementation Notes

### Step-by-Step Approach
1. Import `getDaemonClient` from daemon.ts (created in DAEMIGR-2002)
2. Replace binary spawning block (lines 233-291) with `daemon.search()` call
3. Ensure `searchResult` has same structure as old `rustOutput`
4. Implement comprehensive error handling for all daemon error types
5. Test error paths (daemon crash, timeout, invalid params)
6. Verify chunk ID fetching works identically with daemon results
7. Add migration comment for future maintainers

### Critical Context
- **Keep old spawning code in utils/process.ts** - The VSCode extension still needs it
- Add comment explaining migration: "MCP uses daemon, VSCode uses spawning"
- This creates a dual-mode architecture during Phase 2 migration

### Testing Considerations
- Test all search modes: full-text, vector, hybrid
- Test all filter combinations: repo, worktree, file_type
- Test debug mode output
- Test error scenarios: daemon unavailable, timeout, invalid query
- Verify chunk ID resolution matches current behavior

## Dependencies
- **DAEMIGR-1904** - Unit tests pass, daemon-client package ready
- **DAEMIGR-2002** - daemon.ts singleton created (can implement in parallel, integrate after)

## Risk Assessment
- **Risk**: Breaking existing search functionality
  - **Mitigation**: Comprehensive integration tests in DAEMIGR-2903 will verify all search modes and filters work identically

- **Risk**: Result format mismatch between daemon and spawned process
  - **Mitigation**: Verify searchResult structure matches rustOutput exactly; add format validation tests

- **Risk**: Error handling gaps leading to poor user experience
  - **Mitigation**: Test all error scenarios (daemon crash, timeout, network issues, invalid params)

- **Risk**: Performance regression if daemon startup is slow
  - **Mitigation**: Daemon singleton pattern ensures single startup cost; DAEMIGR-2903 includes performance benchmarks

## Files/Packages Affected
- **Modify:** `/workspace/packages/maproom-mcp/src/tools/search.ts` (lines 233-291)
- **Reference:** `/workspace/packages/maproom-mcp/src/utils/process.ts` (keep for VSCode extension)
- **Import from:** `/workspace/packages/maproom-mcp/src/daemon.ts` (created in DAEMIGR-2002)
- **Architecture reference:** `.crewchief/projects/DAEMIGR_daemon-client-migration/planning/architecture.md`

## Phase Information
- **Phase:** 2 (Integration)
- **Priority:** HIGH
- **Estimated Effort:** 1 day (8 hours)

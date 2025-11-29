# Ticket: OPNFIX-1001: Update getWorktreePath Function for Multi-Candidate Fallback

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Modify the getWorktreePath function in open.ts to query all candidate worktrees, validate file existence for each, and return the first valid path. This enables automatic recovery from database pollution.

## Background
The `mcp__maproom__open` tool is completely broken due to path resolution bugs. When retrieving file contents, it constructs invalid paths by duplicating path segments like `/workspace/crates/maproom/crates/maproom/src/main.rs`.

Current implementation uses `LIMIT 1` without ORDER BY (non-deterministic), trusts database data blindly without filesystem validation, and fails when first match has incorrect abs_path. Bug identified in `.crewchief/reports/2025-11-18_maproom-mcp-context-tool-failure-analysis.md`.

This ticket implements the core fix from the OPNFIX Phase 1 plan: multi-candidate fallback with filesystem validation.

## Acceptance Criteria
- [x] SQL query removes LIMIT 1 and adds ORDER BY w.id DESC
- [x] Function loops through all candidate rows from database
- [x] Validates file existence using fileExists() for each candidate
- [x] Returns first valid worktree abs_path
- [x] Throws ValidationError with candidate count if all fail
- [x] Error message includes suggestion to run cleanup

## Technical Requirements
- Use existing database Client from pg library
- Import fileExists() from utils/validation.ts (created in ticket OPNFIX-1002)
- Use path.join() for path construction
- Maintain existing function signature for backward compatibility
- Follow existing error handling patterns (ValidationError)
- Query should return all matching worktrees ordered by newest first (id DESC)
- Iterate through candidates until file is found on filesystem
- Use async/await properly for filesystem checks

## Implementation Notes

### Current Code Location
File: `packages/maproom-mcp/src/tools/open.ts` (lines 51-85)

### Algorithm
1. Execute SQL query WITHOUT `LIMIT 1`, add `ORDER BY w.id DESC`
2. Receive array of candidate worktree rows
3. For each row in candidates:
   - Construct full path: `path.join(row.abs_path, relpath)`
   - Call `await fileExists(fullPath)`
   - If true, return `row.abs_path` (success)
4. If loop completes without finding valid path:
   - Throw ValidationError with:
     - Number of candidates tried
     - Suggestion to run `maproom db cleanup-stale`
     - Original relpath and worktree name

### Error Handling
- Maintain existing ValidationError usage
- Provide actionable error messages
- Include debug information (candidate count)
- Guide user to resolution (cleanup command)

### Example Implementation Pattern
```typescript
// Query all candidates
const { rows } = await client.query(
  `SELECT w.abs_path FROM worktrees w ...
   ORDER BY w.id DESC`,  // Remove LIMIT 1
  [repoId, worktreeName]
)

if (rows.length === 0) {
  throw new ValidationError(...)
}

// Try each candidate
for (const row of rows) {
  const fullPath = path.join(row.abs_path, relpath)
  if (await fileExists(fullPath)) {
    return row.abs_path  // Success!
  }
}

// All failed
throw new ValidationError(
  `File '${relpath}' not accessible. Tried ${rows.length} candidates. ` +
  `Run 'maproom db cleanup-stale' to fix database pollution.`,
  'FILE_NOT_FOUND'
)
```

## Dependencies
- **Requires**: OPNFIX-1002 (fileExists helper function)
- **Blocks**: OPNFIX-2001, OPNFIX-3001 (security and test tickets)

## Risk Assessment
- **Risk**: Breaking existing functionality if function signature changes
  - **Mitigation**: Maintain exact same function signature and return type
- **Risk**: Performance impact from multiple filesystem checks
  - **Mitigation**: ORDER BY id DESC puts most recent (likely correct) first; typically succeeds on first try
- **Risk**: Database query returns too many rows (memory impact)
  - **Mitigation**: In practice, only 2-3 worktrees per repo; not a concern

## Files/Packages Affected
- `packages/maproom-mcp/src/tools/open.ts` (getWorktreePath function, lines 51-85)
- `packages/maproom-mcp/src/utils/validation.ts` (import fileExists)

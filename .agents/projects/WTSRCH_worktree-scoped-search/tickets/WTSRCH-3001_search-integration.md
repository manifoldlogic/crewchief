# Ticket: WTSRCH-3001: Integrate Worktree Resolution into Search Tool Handler

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- typescript-mcp-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Wire up the worktree resolution logic to the MCP search tool handler so that searches default to the current worktree. Add metadata to search results to inform users about auto-detection, fallbacks, and helpful hints for common scenarios.

## Background
The search tool handler currently accepts an optional `worktree` parameter but defaults to searching all worktrees (null). This causes massive result duplication when users search without specifying a worktree. With the resolution logic from Phase 2 (WTSRCH-2001), we can now auto-detect the current worktree and scope searches appropriately.

This integration completes Phase 3 of the WTSRCH project, which adds auto-detection of the current git branch to make Maproom MCP search default to the current worktree. Phases 1-2 provided git branch detection and worktree resolution logic. This ticket makes the feature live by integrating it into the actual search tool handler.

This integration must preserve backward compatibility - existing code that passes explicit worktree parameters must continue to work unchanged.

## Acceptance Criteria
- [ ] Search tool handler calls `resolveWorktreeId()` before executing search
- [ ] Resolution happens BEFORE existing worktree parameter handling code (around line 655)
- [ ] When `worktree` parameter is undefined, auto-detection triggers
- [ ] When `worktree` parameter is provided (string or null), it's used directly (no auto-detection)
- [ ] Resolved worktree ID is passed to search executor functions (FTS, vector, hybrid)
- [ ] Search result metadata includes resolution information:
  - `auto_detected: boolean` - Was worktree auto-detected?
  - `worktree: string` - Which worktree was searched
  - `mode: string` - Resolution mode (explicit/auto/fallback/all)
  - `hint: string` - Helpful message when fallback occurs
- [ ] Helpful hints provided for common scenarios (branch not indexed, detection failed)
- [ ] Integration tests pass for all resolution tiers
- [ ] Existing tests continue to pass (backward compatibility verified)
- [ ] No breaking changes to MCP tool schema

## Technical Requirements

### 1. Integration Point
Modify `packages/maproom-mcp/src/index.ts` in the search tool handler function.

Insert resolution logic **before** line 650 (after repo validation, before worktree handling).

### 2. Modified Code Pattern
```typescript
// After repo validation (line ~649)
const repoId = repoRows[0].id

// NEW: Resolve worktree ID using three-tier logic
const resolution = await resolveWorktreeId(repo, worktree, client)
let worktreeId: number | null = resolution.id
const resolutionMetadata = resolution.metadata

// NEW: Generate hint message for fallback scenarios
let hint: string | undefined
if (resolutionMetadata.fallback && resolutionMetadata.detected_branch) {
  hint = `Current branch '${resolutionMetadata.detected_branch}' is not indexed.\n\n` +
    `To search your current code:\n` +
    `1. Run: mcp__maproom__scan({repo: "${repo}", worktree: "${resolutionMetadata.detected_branch}"})\n\n` +
    `Searching '${resolutionMetadata.worktree}' worktree instead.`
}

// Continue with existing search execution...
```

### 3. Remove/Replace Existing Code
Lines 653-663 currently handle explicit worktree parameter. This logic is now in `resolveWorktreeId()`, so:
- Keep the code for now (backward compatibility safety)
- But it won't execute because `resolveWorktreeId()` handles it first
- Can be cleaned up in a future refactor (not in this ticket)

### 4. Result Metadata Addition
```typescript
const result: any = {
  hits: rows.map((r) => { /* existing mapping */ }),

  // NEW: Add resolution metadata
  auto_detected: resolutionMetadata.auto_detected ?? false,
  worktree: resolutionMetadata.worktree,
  mode: resolutionMetadata.mode,
  hint: hint,

  // Existing fields...
  total: rows.length,
  debug: debug ? debugInfo : undefined,
}
```

### 5. Hint Messages
Provide helpful, actionable hints for common scenarios:

**Branch not indexed:**
```
Current branch 'feature-xyz' is not indexed.

To search your current code:
1. Run: mcp__maproom__scan({repo: "myrepo", worktree: "feature-xyz"})

Searching 'main' worktree instead.
```

**Git detection failed:**
```
Failed to detect current branch (not in git repository or detached HEAD).

Searching all indexed worktrees.
```

## Implementation Notes

### File: `packages/maproom-mcp/src/index.ts`

**Integration Sequence:**
1. Call `resolveWorktreeId()` immediately after repo validation
2. Use returned `worktreeId` for search execution
3. Build hint message based on metadata
4. Add metadata to result object
5. Ensure existing tests still pass

**Backward Compatibility:**
- Explicit `worktree: "main"` still works (Tier 1 resolution)
- Explicit `worktree: null` still searches all (Tier 1 resolution)
- Existing code passing worktree parameter is unaffected

### Test File: `packages/maproom-mcp/tests/integration/worktree-scoping.test.ts` (new file)

Integration test scenarios using Vitest:
1. **Auto-detection happy path:** In git repo, search without worktree param → returns results from current branch
2. **Explicit override:** In feature branch, pass `worktree: "main"` → returns results from main
3. **Search all:** Pass `worktree: null` → returns results from all worktrees
4. **Fallback to main:** In unindexed branch → returns results from main with hint
5. **Metadata present:** Verify `auto_detected`, `worktree`, `mode` fields in result
6. **Hint messages:** Verify helpful hints appear for fallback scenarios
7. **Backward compatibility:** Existing search calls continue to work

## Dependencies
- **WTSRCH-1001** (Git branch detection) - MUST be completed
- **WTSRCH-2001** (Worktree resolution) - MUST be completed

Both prerequisite tickets must be implemented and tested before this integration work can begin.

## Risk Assessment
- **Risk**: Breaking existing integrations that rely on default behavior
  - **Mitigation**: Comprehensive backward compatibility testing, only change behavior when parameter is undefined
- **Risk**: Confusing hint messages
  - **Mitigation**: Clear, actionable guidance with specific commands to run
- **Risk**: Performance impact of git detection on every search
  - **Mitigation**: Git detection is fast (single command), resolution logic is efficient with early returns

## Files/Packages Affected
- `packages/maproom-mcp/src/index.ts` - Modify search tool handler function
- `packages/maproom-mcp/tests/integration/worktree-scoping.test.ts` - NEW: Integration tests

## Planning References
- Search tool handler: `packages/maproom-mcp/src/index.ts:600-700`
- Architecture decision AD-3 (Graceful degradation): `.agents/projects/WTSRCH_worktree-scoped-search/planning/architecture.md`
- Integration test examples: `packages/maproom-mcp/tests/search_tool.test.ts`
- Project plan: `.agents/projects/WTSRCH_worktree-scoped-search/planning/plan.md`

## Estimated Effort
2-3 hours (integration, testing, verification)

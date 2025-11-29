# VSCDAEMN-1001: Cleanup Deprecated Spawning Utilities

**Status**: ✅ Verified
**Priority**: LOW
**Estimated Effort**: 1-2 days
**Agent**: general-purpose

## Context

After comprehensive project review (see `planning/project-review.md`), we decided to **keep spawning for scan operations** (it's the appropriate pattern for one-time operations). However, there may be deprecated spawning utilities that are genuinely unused and can be removed.

**Key Decision**: Spawning is APPROPRIATE for scan - no migration to daemon needed.

## Objective

Audit deprecated spawning utilities, remove genuinely unused code, and document when spawning vs daemon should be used.

## Scope

**In Scope:**
- Audit `packages/maproom-mcp/src/utils/process.ts` for actual usage
- Remove functions that are truly unused
- Keep spawning utilities needed for VSCode scan
- Document spawning vs daemon usage guidelines

**Out of Scope:**
- Migrating scan to daemon (decision: keep spawning)
- Modifying VSCode extension scan implementation
- daemon-client enhancements (not needed)
- Performance improvements (spawning overhead is negligible)

## Current State

**VSCode Extension** (packages/vscode-maproom/):
- Uses spawning for `scan` command (one-time operation at activation)
- Located in: `packages/vscode-maproom/src/process/scan.ts`
- Works correctly today

**MCP Server** (packages/maproom-mcp/):
- Uses daemon-client for `search` operations (repeated operations)
- Has deprecated spawning utilities in `src/utils/process.ts`
- Some utilities may be unused

**Deprecated Code**:
- `packages/maproom-mcp/src/utils/process.ts` - marked deprecated in DAEMIGR-4003
- Assumption was these should be removed by migrating scan to daemon
- Reality: scan should keep spawning, but some utilities may still be removable

## Tasks

### Task 1: Audit Spawning Utility Usage

**Goal**: Determine which deprecated spawning utilities are actually used.

**Actions**:
1. Read `packages/maproom-mcp/src/utils/process.ts`
2. List all exported functions (e.g., `trySpawnWithCandidates`, etc.)
3. Search codebase for imports/usage of each function:
   ```bash
   grep -r "trySpawnWithCandidates" packages/vscode-maproom/
   grep -r "trySpawnWithCandidates" packages/maproom-mcp/
   # Repeat for other functions
   ```
4. Categorize:
   - **Keep**: Used by VSCode scan (still needed)
   - **Remove**: Genuinely unused (safe to delete)
   - **Uncertain**: Needs further investigation

**Deliverable**: List of functions categorized by Keep/Remove/Uncertain

**Acceptance Criteria**:
- [ ] All functions in process.ts identified
- [ ] Usage search completed for each function
- [ ] Clear categorization (Keep/Remove/Uncertain)

### Task 2: Remove Unused Utilities

**Goal**: Safely remove genuinely unused spawning utilities.

**Actions**:
1. For each "Remove" function:
   - Delete function from `process.ts`
   - Remove any related types/interfaces
   - Update exports
2. For "Uncertain" functions:
   - Document why uncertain
   - Recommend keeping (conservative approach)
3. For "Keep" functions:
   - Remove deprecation warnings
   - Add comment: "Used by VSCode scan - spawning is appropriate for one-time operations"

**Deliverable**: Updated `process.ts` with unused code removed

**Acceptance Criteria**:
- [ ] Unused functions removed
- [ ] Kept functions have updated comments
- [ ] No broken imports in codebase
- [ ] All tests still pass

### Task 3: Document Spawning vs Daemon Guidelines

**Goal**: Create clear documentation to prevent future confusion.

**Actions**:
1. Create or update `packages/maproom-mcp/README.md` with section:
   ```markdown
   ## When to Use Spawning vs Daemon

   ### Use Spawning When:
   - One-time operations (scan, upsert once)
   - Startup/initialization tasks
   - Operations where spawn overhead (<200ms) is negligible
   - Example: VSCode scan at workspace activation

   ### Use Daemon When:
   - Repeated operations (search queries)
   - Low-latency requirements (<50ms)
   - Connection pooling beneficial
   - Example: MCP server search operations

   ### Current Usage:
   - **VSCode scan**: Spawning (correct - one-time operation)
   - **MCP search**: Daemon (correct - repeated operations)
   ```

2. Add comment in `packages/vscode-maproom/src/process/scan.ts`:
   ```typescript
   // NOTE: Spawning is appropriate here - scan is a one-time operation at activation.
   // Spawn overhead (~100-200ms) is negligible compared to scan time (seconds to minutes).
   // daemon-client is for repeated operations (like search), not one-time ops.
   ```

**Deliverable**: Updated documentation with clear guidelines

**Acceptance Criteria**:
- [ ] README.md has spawning vs daemon section
- [ ] VSCode scan.ts has explanatory comment
- [ ] Guidelines are clear and actionable

### Task 4: Verification

**Goal**: Ensure no regressions from cleanup.

**Actions**:
1. Run all tests:
   ```bash
   cd packages/maproom-mcp && pnpm test
   cd packages/vscode-maproom && pnpm test
   ```
2. Manual verification:
   - VSCode extension still activates correctly
   - Scan operation still works
   - No console errors

**Deliverable**: Verification that all functionality still works

**Acceptance Criteria**:
- [ ] All maproom-mcp tests pass
- [ ] All vscode-maproom tests pass
- [ ] VSCode extension loads without errors
- [ ] Scan operation produces correct results

## Success Criteria

**Must Have:**
- [ ] Unused spawning utilities removed (if any found)
- [ ] Used spawning utilities kept with updated comments
- [ ] Documentation added for spawning vs daemon usage
- [ ] All tests passing (zero regressions)

**Nice to Have:**
- [ ] Examples added to documentation
- [ ] Migration guide for future work

## Testing

### Unit Tests
- Existing tests must continue to pass
- No new tests required (no functional changes)

### Integration Tests
- VSCode extension activation and scan
- MCP server search operations
- Verify both patterns still work correctly

### Manual Testing
- Open VSCode with maproom extension
- Trigger initial scan
- Verify scan completes successfully
- No errors in extension host logs

## Risks and Mitigations

| Risk | Severity | Mitigation |
|------|----------|------------|
| Remove used code by mistake | MEDIUM | Thorough grep search, conservative approach |
| Break VSCode extension | MEDIUM | Run tests before/after, manual verification |
| Ambiguous usage | LOW | Keep uncertain functions (conservative) |

## Dependencies

**Requires**:
- Access to VSCode extension codebase
- Access to maproom-mcp codebase
- Ability to run tests

**Blocks**:
- Nothing (standalone cleanup work)

## References

- **Project Review**: `.crewchief/projects/VSCDAEMN_vscode-daemon-migration/planning/project-review.md`
- **Decision**: Option 3 (Simplified Cleanup) - keep spawning, remove unused utilities
- **VSCode Scan**: `packages/vscode-maproom/src/process/scan.ts`
- **Deprecated Utils**: `packages/maproom-mcp/src/utils/process.ts`
- **DAEMIGR Project**: `.crewchief/projects/DAEMIGR_daemon-client-migration/` (daemon-client package)

## Notes

**Why Not Migrate Scan to Daemon?**
- daemon-client lacks scan/upsert/progress support
- Would require 3-5 weeks of prerequisite work (25-30 tickets)
- Performance improvement would be <5% (spawn overhead negligible for one-time ops)
- Spawning is the appropriate pattern for one-time operations

**Key Insight**: Not all operations should use daemon. Spawning is optimal for one-time operations like scan.

---

**Created**: 2025-01-22
**Last Updated**: 2025-01-22
**Ticket Type**: Cleanup / Code Quality

# Ticket: OPNFIX-2002: Add Optional Root Validation to getWorktreePath

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
Add an optional `expectedRoot` parameter to `getWorktreePath()` that skips database candidates with suspicious `abs_path` values (e.g., /etc, /tmp). This provides defense-in-depth protection against database injection attacks while remaining backward compatible.

## Background
Database pollution could be malicious rather than accidental. If an attacker gains database write access, they could insert worktree entries with malicious `abs_path` values pointing to sensitive directories:

```sql
-- Malicious worktree entry
INSERT INTO maproom.worktrees (repo_id, name, abs_path)
VALUES (1, 'main', '/etc')

-- Malicious file entry
INSERT INTO maproom.files (worktree_id, relpath)
VALUES (1, 'passwd')
```

This would cause the open tool to attempt reading `/etc/passwd`. While `validateWithinRepo()` provides primary protection, this optional validation provides an additional layer of defense by skipping obviously suspicious paths during candidate selection.

This security threat was identified in `.crewchief/projects/OPNFIX_open-path-fix/planning/security-review.md` as **Threat 4: Database Pollution as Attack** and addresses **Security Improvement 2: Validate abs_path is Within Expected Boundaries**.

**Reference:** OPNFIX Phase 2: Security Enhancements (security-review.md lines 202-239)

## Acceptance Criteria
- [x] Optional `expectedRoot?: string` parameter added to function signature
- [x] When provided, skips candidates where `abs_path` does not start with `expectedRoot`
- [x] Warning logged when suspicious path is skipped
- [x] Function continues checking remaining candidates (doesn't throw immediately)
- [x] Existing calls work without parameter (backward compatible)
- [x] All candidates validated if `expectedRoot` not provided (no behavior change)
- [x] No breaking changes to function behavior or return type

## Technical Requirements
- Parameter type: `expectedRoot?: string` (optional)
- Validation logic: `row.abs_path.startsWith(expectedRoot)`
- Log at WARN level (not ERROR - this is defensive, not critical)
- Continue to next candidate after logging (don't throw error)
- Update function signature only; no changes to call sites needed
- Maintain same return type: `Promise<string>`
- Maintain same error handling patterns
- No changes to SQL query

## Implementation Notes

### Current Code Location
File: `packages/maproom-mcp/src/tools/open.ts`
Function: `getWorktreePath()`
Lines: 51-85

### Algorithm Enhancement
1. Add optional `expectedRoot` parameter to function signature
2. Execute SQL query (unchanged)
3. Loop through candidate rows:
   - **NEW:** If `expectedRoot` provided AND `abs_path` doesn't start with it:
     - Log warning with abs_path and expectedRoot
     - `continue` to next candidate
   - Construct candidate path (existing logic)
   - Check file existence (existing logic)
   - Validate within repo (existing logic)
   - Return if valid (existing logic)
4. Throw ValidationError if all candidates fail (existing logic)

### Implementation Pattern
```typescript
async function getWorktreePath(
  client: Client,
  worktreeName: string,
  relpath: string,
  expectedRoot?: string  // NEW: Optional parameter
): Promise<string> {
  const { rows } = await client.query(
    `SELECT DISTINCT w.abs_path
     FROM maproom.worktrees w
     JOIN maproom.files f ON f.worktree_id = w.id
     WHERE f.relpath = $1 AND w.name = $2
     ORDER BY w.id DESC`,
    [relpath, worktreeName]
  )

  if (rows.length === 0) {
    throw new ValidationError(
      `No worktree found with name '${worktreeName}' containing file '${relpath}'`,
      'WORKTREE_NOT_FOUND'
    )
  }

  for (const row of rows) {
    // NEW: Optional root validation
    if (expectedRoot && !row.abs_path.startsWith(expectedRoot)) {
      log.warn(
        { abs_path: row.abs_path, expectedRoot },
        'Skipping worktree with suspicious abs_path'
      )
      continue
    }

    const candidatePath = path.join(row.abs_path, relpath)

    if (await fileExists(candidatePath)) {
      validateWithinRepo(candidatePath, row.abs_path)
      return row.abs_path
    }
  }

  throw new ValidationError(
    `File '${relpath}' not found in any candidate worktree '${worktreeName}'. ` +
    `Checked ${rows.length} candidates. ` +
    `Try running database cleanup.`,
    'FILE_NOT_FOUND'
  )
}
```

### Security Considerations
- **Defense in depth**: This is NOT the primary security control
- **Primary security**: `validateWithinRepo()` validates final path (existing)
- **This adds**: Early detection of malicious database entries
- **Benefit**: Catches malicious paths before filesystem check
- **Flexibility**: Optional parameter maintains backward compatibility
- **Configuration**: Could be extended to read from environment variable

### Backward Compatibility
- Parameter is optional; defaults to `undefined`
- When `undefined`, validation is skipped (current behavior)
- All existing call sites continue to work unchanged
- No breaking changes to function contract
- Can add parameter to call sites incrementally as needed

### Logging Strategy
- **Level**: WARN (not ERROR)
- **Reason**: This is suspicious but not necessarily an error
- **Data**: Include both `abs_path` and `expectedRoot` for investigation
- **Message**: Clear indication of why path was skipped
- **Action**: Operators can investigate database integrity if they see these warnings

## Dependencies
- **Requires**: OPNFIX-1001 (getWorktreePath structure and loop)
- **Requires**: Existing `validateWithinRepo()` function (already in place)
- **Blocks**: None (enhances security, optional feature)
- **Related**: OPNFIX-2001 (companion security enhancement)

## Risk Assessment
- **Risk**: Legitimate worktrees outside expectedRoot could be skipped
  - **Mitigation**: Parameter is optional; only use when root is known
- **Risk**: Warning log spam if many suspicious entries
  - **Mitigation**: Log at WARN level; indicates real database problem needing attention
- **Risk**: Breaking existing functionality
  - **Mitigation**: Optional parameter; all existing calls work unchanged

## Files/Packages Affected
- `packages/maproom-mcp/src/tools/open.ts` - getWorktreePath function (lines 51-85)

## Testing Notes

### Manual Testing Scenarios
1. **Normal operation** (no expectedRoot) - Should work exactly as before
2. **Valid candidate** (with expectedRoot) - Should work normally if abs_path matches
3. **Suspicious candidate** (with expectedRoot) - Should skip and log warning
4. **All candidates suspicious** - Should throw FILE_NOT_FOUND after checking all
5. **Mixed candidates** - Should skip suspicious, use valid ones

### Unit Test Requirements
Unit tests for these scenarios should be created in Phase 3 (OPNFIX-3002).
This ticket focuses on implementation; comprehensive security testing is separate.

## Future Enhancements (Out of Scope)
These are NOT part of this ticket but could be considered later:
- Read allowed root from environment variable: `process.env.MAPROOM_ALLOWED_ROOT`
- Support multiple allowed roots (array parameter)
- Configuration file for allowed/denied paths
- Metric tracking for skipped suspicious paths

## Implementation Checklist
- [ ] Add `expectedRoot?: string` parameter to function signature
- [ ] Add conditional check after query results retrieved
- [ ] Add warning log with proper context data
- [ ] Test with parameter not provided (backward compatibility)
- [ ] Test with parameter provided and matching abs_path
- [ ] Test with parameter provided and non-matching abs_path
- [ ] Verify existing call sites still work
- [ ] Verify error handling unchanged

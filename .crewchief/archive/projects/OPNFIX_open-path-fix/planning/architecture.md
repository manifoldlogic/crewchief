# Architecture: Open Tool Path Resolution Fix

**Date:** 2025-11-18
**Project:** OPNFIX - Open Tool Path Resolution Fix
**Approach:** MVP - Fix the immediate bug, ship value fast

## Design Philosophy

**Goal:** Make open tool work reliably with both clean and polluted database states.

**Non-Goal:** Fix the underlying index pollution (that's Project 3: Index Cleanup).

**MVP Principles:**
- Simple, focused fix
- No database schema changes
- No breaking API changes
- Works as defensive programming against bad data
- Ships in 3-5 days

## Current vs. Proposed Architecture

### Current (Broken)

```typescript
async function getWorktreePath(
  client: Client,
  worktreeName: string,
  relpath: string
): Promise<string> {
  // Problem: Trusts first database match blindly
  const { rows } = await client.query(
    `SELECT w.abs_path FROM maproom.worktrees w
     JOIN maproom.files f ON f.worktree_id = w.id
     WHERE f.relpath = $1 AND w.name = $2
     LIMIT 1`,  // ← Non-deterministic!
    [relpath, worktreeName]
  )

  return rows[0].abs_path  // ← No validation!
}

// Then directly joins and reads
const absolutePath = path.join(worktreePath, relpath)
const content = await fs.readFile(absolutePath, 'utf8')
```

**Problems:**
1. No validation that joined path exists
2. `LIMIT 1` without ORDER BY is non-deterministic
3. No fallback if first match is wrong
4. Silent failure - no debugging info

### Proposed (Robust)

```typescript
async function getWorktreePath(
  client: Client,
  worktreeName: string,
  relpath: string
): Promise<string> {
  // Get ALL candidate worktrees, ordered by preference
  const { rows } = await client.query(
    `SELECT DISTINCT w.abs_path
     FROM maproom.worktrees w
     JOIN maproom.files f ON f.worktree_id = w.id
     WHERE f.relpath = $1 AND w.name = $2
     ORDER BY w.id DESC  -- Prefer most recent
     `,
    [relpath, worktreeName]
  )

  // Try each candidate until we find a valid file
  for (const row of rows) {
    const candidatePath = path.join(row.abs_path, relpath)

    // Validate file exists
    if (await fileExists(candidatePath)) {
      // Validate within repo boundaries (security)
      validateWithinRepo(candidatePath, row.abs_path)

      return row.abs_path
    }
  }

  // None worked - provide helpful error
  throw new ValidationError(
    `File '${relpath}' not accessible in worktree '${worktreeName}'. ` +
    `Tried ${rows.length} candidate paths. ` +
    `This may indicate database pollution. Run maproom cleanup.`,
    'FILE_NOT_FOUND'
  )
}
```

**Benefits:**
1. ✅ Validates filesystem before returning
2. ✅ Deterministic ordering (newest first)
3. ✅ Automatic fallback to alternatives
4. ✅ Clear error messages with actionable advice
5. ✅ Works with both clean and polluted databases

## Key Design Decisions

### Decision 1: Validate Before Trust

**Problem:** Database contains incorrect path relationships.

**Solution:** Always validate that `abs_path + relpath` produces a real, accessible file before using it.

**Implementation:**
```typescript
async function fileExists(path: string): Promise<boolean> {
  try {
    await fs.access(path, fs.constants.R_OK)
    return true
  } catch {
    return false
  }
}
```

**Trade-off:**
- ✅ Pro: Catches all path errors immediately
- ✅ Pro: Works with polluted data
- ⚠️ Con: Extra filesystem call (negligible - ~1ms)

**Decision:** Accept tiny performance cost for reliability.

### Decision 2: Deterministic Ordering

**Problem:** `LIMIT 1` without ORDER BY returns random worktree.

**Solution:** Order by `worktree.id DESC` to prefer most recent.

**Rationale:**
- Newer worktree entries are more likely to be correct
- If user recently re-indexed correctly, use that data
- Provides predictable behavior for debugging

**Alternative Considered:** Order by `abs_path` length (shortest = repository root).
- **Rejected:** Doesn't handle submodules or monorepos correctly

### Decision 3: Multi-Candidate Fallback

**Problem:** First candidate may be polluted data.

**Solution:** Try ALL candidates in order until one works.

**Implementation:**
```typescript
// Get multiple candidates
const { rows } = await client.query(...)

// Try each until success
for (const row of rows) {
  const candidatePath = path.join(row.abs_path, relpath)
  if (await fileExists(candidatePath)) {
    validateWithinRepo(candidatePath, row.abs_path)
    return row.abs_path  // Success!
  }
}

// All failed
throw new ValidationError(...)
```

**Trade-off:**
- ✅ Pro: Automatic recovery from pollution
- ✅ Pro: No manual intervention needed
- ⚠️ Con: Hides database quality issues

**Mitigation:** Add logging at `debug` level:
```typescript
log.debug({ tried: row.abs_path, exists: false }, 'Candidate worktree path invalid')
```

### Decision 4: No Schema Changes

**Problem:** Could add constraints to prevent pollution.

**Solution:** Don't. Fix the symptom, let Project 3 fix the cause.

**Rationale:**
- Schema changes require migration
- Migration needs careful testing
- Adds scope and risk to this quick fix
- Project 3 (Index Cleanup) will address root cause

**MVP Principle:** Ship value fast, iterate later.

## Error Handling Strategy

### Current Error Messages (Unhelpful)

```
Error: ENOENT: no such file or directory, stat '/workspace/crates/maproom/crates/maproom/src/main.rs'
```

**Problems:**
- Doesn't explain WHY path is wrong
- Doesn't suggest remediation
- Doesn't help debugging

### Proposed Error Messages (Actionable)

**Scenario 1: No worktrees found**
```json
{
  "error": "FILE_NOT_FOUND",
  "message": "File 'crates/maproom/src/main.rs' not found in worktree 'main'. Available worktrees for this file: ['dev', 'feature-x']. Check your worktree parameter."
}
```

**Scenario 2: All candidates failed**
```json
{
  "error": "FILE_NOT_FOUND",
  "message": "File 'crates/maproom/src/main.rs' not accessible in worktree 'main'. Tried 3 candidate paths but none exist on disk. This indicates database pollution. Run 'maproom db cleanup-stale' to fix."
}
```

**Scenario 3: Security violation**
```json
{
  "error": "INVALID_PATH",
  "message": "Path '/etc/passwd' escapes repository boundary '/workspace'. This may indicate a security issue or database corruption."
}
```

## Component Design

### Modified: `getWorktreePath()`

**Location:** `packages/maproom-mcp/src/tools/open.ts`

**New Signature:**
```typescript
async function getWorktreePath(
  client: Client,
  worktreeName: string,
  relpath: string
): Promise<string>
```

**Return Value:** Still returns `abs_path`, but now **validated**.

**Key Changes:**
1. Query returns multiple rows (no LIMIT 1)
2. Add ORDER BY for determinism
3. Loop through candidates
4. Validate file existence for each
5. Return first valid match
6. Throw detailed error if none valid

### New: `fileExists()` Helper

**Location:** `packages/maproom-mcp/src/utils/validation.ts`

**Purpose:** Check if file exists and is readable.

```typescript
export async function fileExists(filePath: string): Promise<boolean> {
  try {
    await fs.access(filePath, fs.constants.R_OK)
    return true
  } catch {
    return false
  }
}
```

**Why not use fs.stat():** `access()` is faster, we don't need file metadata.

### Enhanced: Error Messages

**Location:** `packages/maproom-mcp/src/tools/open.ts:51-85`

**Changes:**
- Include number of candidates tried
- Suggest worktree names that DO have the file
- Mention database cleanup if pollution suspected
- Include actual paths tried in debug logs

## Data Flow (Proposed)

```
1. User calls: open({ relpath: "crates/maproom/src/main.rs", worktree: "main" })
   ↓
2. Validate and normalize relpath → "crates/maproom/src/main.rs"
   ↓
3. Query: Get ALL worktrees for (worktree="main", relpath=...) ORDER BY id DESC
   ↓
4. Results: [
     { abs_path: "/workspace/crates/maproom" },  // Polluted
     { abs_path: "/workspace" }                  // Correct
   ]
   ↓
5. Try candidate 1: "/workspace/crates/maproom" + "crates/maproom/src/main.rs"
   → Path: "/workspace/crates/maproom/crates/maproom/src/main.rs"
   → fileExists(): false ❌
   → Continue to next candidate
   ↓
6. Try candidate 2: "/workspace" + "crates/maproom/src/main.rs"
   → Path: "/workspace/crates/maproom/src/main.rs"
   → fileExists(): true ✅
   → validateWithinRepo(): pass ✅
   → Return abs_path: "/workspace"
   ↓
7. readFileFromFilesystem("/workspace", "crates/maproom/src/main.rs")
   ↓
8. Success! Return file contents to user.
```

**Key Insight:** Automatic recovery from pollution without user intervention.

## Performance Considerations

### Additional Costs

1. **Multiple candidate query:** Same as before (LIMIT 1 → no limit)
   - Impact: Negligible (typical: 1-3 rows)

2. **File existence checks:** New cost
   - Impact: ~1ms per check
   - Typical: 1-2 checks (first or second candidate works)
   - Worst case: 5-10ms if trying many polluted entries

3. **Logging:** Debug-level only
   - Impact: Negligible in production (disabled by default)

### Total Impact

- **Best case:** +1ms (first candidate works)
- **Typical case:** +2ms (second candidate works)
- **Worst case:** +10ms (many polluted entries)

**Decision:** Acceptable for correctness. File operations are already slow (~10-100ms to read file).

## Migration Path

**No data migration needed!** This is pure code fix.

**Deployment:**
1. Update `packages/maproom-mcp/src/tools/open.ts`
2. Add `fileExists()` to `src/utils/validation.ts`
3. Update tests
4. Build and deploy

**Rollback:** Simple - revert code changes.

**Compatibility:** No breaking changes to MCP API.

## Monitoring and Debugging

### Logging Strategy

**Info Level:** Normal operation
```
handleOpenTool called with relpath="crates/maproom/src/main.rs"
handleOpenTool completed successfully
```

**Debug Level:** Path resolution details
```
getWorktreePath found 3 candidates
Trying candidate 1: abs_path="/workspace/crates/maproom"
Candidate 1 failed: file does not exist
Trying candidate 2: abs_path="/workspace"
Candidate 2 success: file exists and validated
```

**Error Level:** Failures
```
getWorktreePath exhausted all candidates
File not accessible after trying 3 paths
Returning FILE_NOT_FOUND error
```

### Metrics to Track

After deployment, monitor:
- **Success rate:** Should increase to ~100%
- **Candidate tries:** How often is fallback needed?
- **Error reasons:** Still getting FILE_NOT_FOUND? Why?

## Future Enhancements (Out of Scope)

These are **NOT** in this project (MVP focus):

1. **Cache validated paths:** Avoid repeated filesystem checks
   - **Why not now:** Adds complexity, minimal gain

2. **Worktree preference hints:** Let user specify which worktree to prefer
   - **Why not now:** No user demand, adds API surface

3. **Auto-fix database:** Update incorrect abs_path values
   - **Why not now:** Let Project 3 (Index Cleanup) handle this

4. **Path normalization:** Handle symlinks, case-insensitive filesystems
   - **Why not now:** No evidence this is a problem yet

**Principle:** Ship the simple fix, iterate based on real-world feedback.

## Open Questions

### Q1: Should we update the incorrect worktree entry in the database?

**Answer:** No. Read-only fix is simpler and safer.
- Writing to database adds transaction complexity
- Need to handle write failures
- Project 3 will clean up properly

### Q2: What if file exists in multiple worktrees with different content?

**Answer:** Return the first valid one (ordered by worktree.id DESC).
- This is already the current behavior (LIMIT 1)
- Deterministic ordering makes it predictable
- Content differences should be rare (same branch name)

### Q3: Should we warn when falling back to alternate candidate?

**Answer:** Yes, but only in debug logs.
- Info-level would spam logs in polluted environments
- Debug-level helps troubleshooting without noise

## Conclusion

This architecture delivers a **surgical fix** that:
- ✅ Works with both clean and polluted databases
- ✅ No schema changes or migrations
- ✅ No breaking API changes
- ✅ Minimal performance impact
- ✅ Clear error messages
- ✅ Can ship in 3-5 days

The fix is **defensive programming** - assume database may be wrong, validate against reality (filesystem), proceed carefully.

**Next:** Define test strategy to ensure this never breaks again.

# Analysis: Open Tool Path Resolution Bug

**Date:** 2025-11-18
**Project:** OPNFIX - Open Tool Path Resolution Fix
**Priority:** Critical

## Problem Statement

The `mcp__maproom__open` tool is completely non-functional, producing invalid file paths that prevent any file from being read. When users attempt to retrieve file contents, the tool duplicates path segments, creating paths like:

```
/workspace/crates/maproom/crates/maproom/src/main.rs
                         ^^^^^^^^^^^^^^^^^^^ DUPLICATED
```

This breaks the fundamental workflow: **search → get chunk → open file**.

## Observed Failures

From the MCP context tool failure analysis:

**Attempt 1:**
```typescript
open({
  relpath: "crates/maproom/src/main.rs",
  worktree: "main"
})

// Error: ENOENT: no such file or directory
// stat '/workspace/crates/maproom/crates/maproom/src/main.rs'
```

**Attempt 2:**
```typescript
open({
  relpath: "src/main.rs",
  worktree: "main"
})

// Error: ENOENT: no such file or directory
// stat '/tmp/.tmpZgxDYt/src/main.rs'
//      ^^^^^^^^^^^^^^^ Wrong base path
```

Both attempts failed, making the tool completely unusable.

## Root Cause Analysis

### The Bug: Inconsistent Database State

After deep investigation, the bug stems from **database pollution with inconsistent path relationships**:

**Correct State:**
```
worktrees.abs_path = "/workspace"
files.relpath = "crates/maproom/src/main.rs"
Joined path = "/workspace/crates/maproom/src/main.rs" ✅
```

**Polluted State:**
```
worktrees.abs_path = "/workspace/crates/maproom"  ← Wrong!
files.relpath = "crates/maproom/src/main.rs"
Joined path = "/workspace/crates/maproom/crates/maproom/src/main.rs" ❌
```

### How This Happened

The Rust indexer stores paths as:

```rust
// src/indexer/mod.rs:279-283
let worktree_id = crate::db::get_or_create_worktree(
    &client,
    repo_id,
    worktree,
    root_abs.to_string_lossy().as_ref(),  // Repository root
)
```

The `root_abs` value comes from `root.canonicalize()`, which is the path passed to the scan command. If the indexer is run from different locations or with different path arguments, it creates multiple worktree entries for the same logical worktree:

1. User runs: `maproom scan /workspace` → abs_path="/workspace"
2. User runs: `maproom scan /workspace/crates/maproom` → abs_path="/workspace/crates/maproom"
3. Both index the same files with **relpath relative to their respective roots**

Now the database has **conflicting data** for the same logical files.

### The getWorktreePath Bug

The `open.ts` implementation trusts database data blindly:

```typescript
// packages/maproom-mcp/src/tools/open.ts:51-85
async function getWorktreePath(
  client: Client,
  worktreeName: string,
  relpath: string
): Promise<string> {
  const { rows } = await client.query(
    `SELECT w.abs_path
     FROM maproom.worktrees w
     JOIN maproom.files f ON f.worktree_id = w.id
     WHERE f.relpath = $1 AND w.name = $2
     LIMIT 1`,  // ← Takes FIRST match, which may be wrong!
    [relpath, worktreeName]
  )

  return rows[0].abs_path  // ← No validation!
}
```

**Critical Issues:**

1. **No validation**: Doesn't check if `abs_path + relpath` produces a real file
2. **Non-deterministic**: `LIMIT 1` without `ORDER BY` returns arbitrary worktree
3. **Trusts pollution**: Returns first match even if it's inconsistent data

### Why Tests Didn't Catch This

**Unit Tests (open.test.ts):**
- Only test validation functions in isolation
- Mock all database interactions
- Never validate actual path construction
- Lines 87-127: Path validation tests use hardcoded paths, no database

**Integration Tests (open.int.test.ts):**
- Lines 199-207: **End-to-end tests are SKIPPED**:
  ```typescript
  it.skip('should handle full workflow: filesystem read', async () => {
    // This would require a fully set up test environment with database data
    // Marked as skip for now - implement when test fixtures are available
  })
  ```
- Lines 175-196: Database tests only check if queries work, not path correctness
- No test validates: **search → open → verify contents match**

**Critical Test Gap:**

No test validates the complete data flow:
```
1. Index a file at known path
2. Search for it
3. Get chunk_id from search results
4. Call open tool with chunk's relpath and worktree
5. Verify returned content matches the actual file
```

This end-to-end validation would have caught the bug immediately.

## Current Implementation

### Path Construction Flow

```typescript
// 1. User provides relpath and worktree name
{ relpath: "crates/maproom/src/main.rs", worktree: "main" }

// 2. Query database for worktree abs_path
SELECT w.abs_path FROM maproom.worktrees w
JOIN maproom.files f ON f.worktree_id = w.id
WHERE f.relpath = $1 AND w.name = $2
LIMIT 1

// 3. Join paths
const absolutePath = path.join(worktreePath, relpath)

// 4. Read file
await fs.readFile(absolutePath, 'utf8')
```

### Indexer Path Storage

The Rust indexer consistently uses repository root:

```rust
// All calls pass the same root for a given scan operation
let root_abs = root.canonicalize()?;  // e.g., "/workspace"

let repo_id = db::get_or_create_repo(
    client,
    repo,
    root_abs.to_string_lossy().as_ref()
).await?;

let worktree_id = db::get_or_create_worktree(
    client,
    repo_id,
    worktree,
    root_abs.to_string_lossy().as_ref()  // Same root
).await?;
```

**The indexer is correct** - it consistently uses the same root for all operations within a scan. The problem is **multiple scans with different roots** polluting the database.

## Database Schema (Relevant Tables)

```sql
-- maproom.worktrees
CREATE TABLE maproom.worktrees (
    id BIGSERIAL PRIMARY KEY,
    repo_id BIGINT REFERENCES maproom.repos(id),
    name TEXT NOT NULL,  -- Branch name like "main"
    abs_path TEXT,       -- Absolute path to repository root
    UNIQUE(repo_id, name)
);

-- maproom.files
CREATE TABLE maproom.files (
    id BIGSERIAL PRIMARY KEY,
    repo_id BIGINT,
    worktree_id BIGINT REFERENCES maproom.worktrees(id),
    relpath TEXT NOT NULL,  -- Relative to repository root
    -- ...
);
```

**The Constraint Problem:**

The UNIQUE constraint is `(repo_id, name)`, which means:
- Multiple worktrees can exist for same repo + branch name
- If abs_path changes, it UPDATES the existing row (ON CONFLICT DO UPDATE)
- But old files with old relpath values remain!

This creates **temporal inconsistency** where files from different indexing runs coexist with incompatible path relationships.

## Impact Assessment

**Severity:** Critical - Tool is completely broken

**User Impact:**
- Cannot retrieve file contents from search results
- Cannot use context tool (depends on open tool)
- MCP-based workflows completely blocked
- Users must fall back to manual file reading

**Workarounds:**
- Use Claude Code's `Read` tool directly with absolute paths
- Search to find location, then manually construct file path
- No workaround for context tool (requires database integration)

## Related Issues

This bug is a **symptom of the larger index pollution problem** documented in:
- `.crewchief/reports/2025-11-18_maproom-mcp-context-tool-failure-analysis.md`
- `.crewchief/reports/2025-11-18_maproom-mcp-projects-breakdown.md` (Project 3: Index Cleanup)

**Dependency Chain:**
```
Index Pollution (Project 3)
    ↓ creates
Duplicate/Inconsistent Worktrees
    ↓ breaks
Open Tool Path Resolution (THIS PROJECT)
    ↓ blocks
Context Tool Usage
```

While index cleanup will prevent **future** pollution, this project provides an **immediate fix** that works even with polluted data.

## Success Metrics

Fix is complete when:
- ✅ Open tool reads files successfully with relpath from search results
- ✅ Path construction handles both clean and polluted database states
- ✅ All paths validate against actual filesystem before use
- ✅ End-to-end tests verify search → open workflow
- ✅ Error messages clearly indicate which worktree paths were tried
- ✅ Zero path duplication errors in logs

## Next Steps

1. Design robust path resolution that validates filesystem
2. Add deterministic worktree selection (ORDER BY)
3. Implement fallback to alternative worktrees if first fails
4. Create comprehensive end-to-end test suite
5. Add logging for debugging path resolution
6. Document expected database state

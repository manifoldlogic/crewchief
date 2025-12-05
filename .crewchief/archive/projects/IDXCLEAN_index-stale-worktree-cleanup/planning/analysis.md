# Analysis: Index Stale Worktree Cleanup

## Problem Definition

### Core Issue
The maproom index database contains **100+ worktrees**, of which **~95% no longer exist on disk**. This creates severe search quality degradation:

- Same code chunk appears **15+ times** in search results (one per stale worktree)
- Actual relevant results are buried in noise from duplicate stale entries
- Search result duplication makes it impossible to find the right version of code
- Context tool becomes unusable due to multiple chunk_id values for same conceptual code

### Root Cause Analysis

**Origin:** Genetic algorithm experimentation phase
- Created many temporary worktrees in `.crewchief/` directory for parallel experiments
- Each worktree was indexed as it was created
- When experiments completed, worktrees were deleted from disk
- **Database was never updated** to reflect deletions
- Result: 95+ "zombie" worktree records pointing to non-existent paths

**Persistence Mechanism:**
- Database worktree records remain until explicitly deleted
- No automatic cleanup on worktree deletion
- No validation that `abs_path` still exists on disk
- Foreign key CASCADE relationships mean chunks remain with zombie worktrees

### Impact Assessment

**Search Quality Impact:**
- **Symptom:** Searching for "validate_provider" returns same function 15 times
- **Effect:** User cannot determine which is the "real" version
- **Cascading failure:** Context tool blocked (can't pick correct chunk_id)
- **Severity:** Critical - makes search nearly unusable

**Database Size Impact:**
- Stale worktrees: ~95 records (small)
- Stale chunks: ~500,000 rows (significant)
- Estimated bloat: 2-3 GB of useless data
- Query performance degradation: Joins across 100+ worktrees

**User Experience Impact:**
- Users lose trust in search results
- Manual deduplication required (impossible at scale)
- Workaround: Hope the first result is correct
- Frustration: Can't tell which worktree is "main"

## Existing Solutions and Approaches

### Industry Solutions

**Git worktree management:**
- Git itself has `git worktree prune` command
- Removes worktree admin files for deleted worktrees
- **Limitation:** Only handles `.git/worktrees/`, not application indexes

**Database cleanup patterns:**
- Soft delete with `deleted_at` timestamps
- Periodic vacuum/cleanup jobs
- Cascade delete with foreign keys
- **Standard approach:** Background job + validation + safe deletion

**Index cleanup strategies:**
- Lucene/Elasticsearch: Segment merging and deletion
- Database indexes: VACUUM ANALYZE to reclaim space
- File-based indexes: Remove orphaned entries during merge
- **Common pattern:** Lazy cleanup + periodic deep cleanup

### Similar Problems in Other Tools

**ctags/LSP servers:**
- Index files that may be deleted
- Solution: Regenerate index on file change events
- Limitation: Assumes files are tracked by VCS

**IDE project indexes:**
- IntelliJ IDEA: Invalidates cache when project structure changes
- VS Code: Rebuilds index when workspace folders change
- Solution: File system watchers + incremental updates

**Code search tools (Sourcegraph, OpenGrok):**
- Must handle repository deletions
- Solution: Periodic full re-index + incremental updates
- Trade-off: Full re-index is expensive

### What Works / What Doesn't

**✅ What Works:**
- Periodic background cleanup jobs (standard industry practice)
- Validation before use (check path exists before returning)
- Soft delete + hard delete pattern (two-phase cleanup)
- User-triggered cleanup commands (`prune`, `cleanup`, `gc`)

**❌ What Doesn't Work:**
- Automatic cleanup on every operation (too expensive)
- Relying on file system watchers for deletions (unreliable)
- Full re-index instead of targeted cleanup (too slow)
- No cleanup at all (leads to current problem)

## Current State Analysis

### Database Schema

```sql
-- Current schema (simplified)
CREATE TABLE worktrees (
    id SERIAL PRIMARY KEY,
    repo_id INTEGER NOT NULL,
    name VARCHAR(255) NOT NULL,
    abs_path TEXT NOT NULL,  -- ⚠️ Not validated for existence
    commit_sha VARCHAR(64),
    updated_at TIMESTAMP
);

CREATE TABLE chunks (
    id SERIAL PRIMARY KEY,
    worktree_id INTEGER REFERENCES worktrees(id) ON DELETE CASCADE,
    relpath TEXT NOT NULL,
    symbol_name TEXT,
    start_line INTEGER,
    end_line INTEGER,
    -- ... other fields
);
```

**Key observation:** `worktrees.abs_path` is TEXT with no constraint ensuring it exists on disk.

### Current Cleanup Mechanisms

**None.** There is no existing cleanup mechanism for stale worktrees.

**Existing database operations:**
- INSERT: New worktrees added during `maproom scan`
- SELECT: Worktrees queried during `maproom search` and `mcp__maproom__open`
- UPDATE: `updated_at` timestamp updated during re-scans
- DELETE: Never performed

### User Workarounds

**Current user behavior:**
1. Notice duplicate results in search
2. Manually identify which worktree is "correct" (usually "main")
3. Use worktree filter in search (if they know the correct name)
4. Give up and search codebase manually with `grep`

**Why workarounds fail:**
- Worktree filter requires knowing which worktree is valid
- Can't distinguish "main" from "main-2024-11-15-experiment"
- No way to see which worktrees exist on disk from MCP tools

## Research Findings

### Stale Worktree Detection

**Method 1: Disk existence check**
```rust
use std::path::Path;
Path::new(&worktree.abs_path).exists()
```
- **Pros:** Simple, definitive
- **Cons:** I/O operation per worktree, can be slow
- **Edge case:** Network mounts, temporarily unmounted disks

**Method 2: Git validation**
```bash
git -C <abs_path> rev-parse --is-inside-work-tree
```
- **Pros:** Validates it's actually a git worktree
- **Cons:** Slower than file check, requires git binary
- **Edge case:** Git worktree deleted but directory remains

**Method 3: Cross-reference with git worktree list**
```bash
git worktree list --porcelain
```
- **Pros:** Authoritative source of active worktrees
- **Cons:** Requires access to main repository, expensive to parse
- **Edge case:** Worktrees in different repos

**Recommendation:** Method 1 (disk existence) for MVP, Method 3 for future enhancement

### Exclusion Patterns

**Problem:** `.crewchief/` directory contains temporary worktrees that shouldn't be indexed.

**Pattern matching approaches:**
1. **Path prefix exclusion:** Skip any path starting with `.crewchief/`
2. **Glob pattern exclusion:** Use glob patterns (`.crewchief/**`)
3. **Configuration-based:** User-defined exclusion list in config

**Precedent from other tools:**
- `.gitignore` patterns (widely understood)
- `.dockerignore` patterns (similar use case)
- Ripgrep's `--glob` patterns (performance-optimized)

**Recommendation:** Path prefix exclusion for MVP, extend to glob patterns later

### Safety Considerations

**Risk: False positives (delete valid worktree)**
- **Cause:** Disk temporarily unmounted, network path unavailable
- **Mitigation:** Dry-run mode, require user confirmation, log before delete

**Risk: Cascade deletion removes needed chunks**
- **Cause:** Foreign key CASCADE deletes all chunks when worktree deleted
- **Mitigation:** Intentional design - chunks without worktree are useless

**Risk: Concurrent operations during cleanup**
- **Cause:** Indexing happening while cleanup runs
- **Mitigation:** Database transaction isolation, avoid cleanup during active indexing

### Performance Characteristics

**Cleanup operation costs:**
- Disk existence check: ~1ms per path (SSD), ~10ms per path (HDD)
- Database DELETE: ~5ms per worktree (with CASCADE)
- Total for 95 worktrees: ~100ms (SSD) to ~1s (HDD)

**Acceptable performance:**
- Startup cleanup: <2s acceptable (runs once)
- Periodic cleanup: <500ms acceptable (runs in background)
- Interactive cleanup: <5s acceptable (user initiated)

**Optimization strategies:**
- Batch disk checks (parallel using tokio)
- Use database transactions for atomicity
- Cache last cleanup timestamp to avoid redundant checks

## Key Insights

### Design Principles

1. **Safety first:** Never delete data without validation
2. **Explicit better than implicit:** User should know what will be deleted
3. **Idempotent operations:** Cleanup can run multiple times safely
4. **Fail gracefully:** Don't break indexing if cleanup fails

### Architectural Insights

1. **Worktree lifecycle should be managed:**
   - Created during `scan`
   - Validated during `search`/`open`
   - Cleaned during `cleanup` or `watch`

2. **Separation of concerns:**
   - Detection logic: Identify stale worktrees
   - Deletion logic: Remove from database
   - Exclusion logic: Prevent future pollution

3. **Integration points:**
   - Standalone CLI command: `maproom db cleanup-stale`
   - Watch command integration: Automatic periodic cleanup
   - Search/open validation: Filter out stale results inline

### Constraints and Trade-offs

**Constraint 1: Backward compatibility**
- Must not break existing indexes
- Must handle databases with 0 or 100 stale worktrees

**Constraint 2: Performance**
- Cleanup must not block indexing operations
- Acceptable latency: <2s for startup, <500ms for periodic

**Constraint 3: Safety**
- Must not accidentally delete valid worktrees
- Provide dry-run mode for inspection
- Log all deletions for audit trail

**Trade-off: Aggressive vs Conservative cleanup**
- Aggressive: Delete any path that doesn't exist (fast, risky)
- Conservative: Confirm with user before delete (safe, slow)
- **Decision:** Aggressive for background, conservative for manual

## Problem Space Summary

**Core Problem:** Database pollution from deleted worktrees creates search result duplication (15x) and makes context tool unusable.

**Root Cause:** No cleanup mechanism when worktrees are deleted from disk; database records persist indefinitely.

**Solution Requirements:**
1. Detect stale worktrees (abs_path doesn't exist)
2. Safely remove stale records from database
3. Prevent future pollution via exclusion patterns
4. Integrate cleanup into normal workflows (watch command)

**Success Criteria:**
- Worktree count drops from 100+ to <10
- Search result duplication drops from 15x to 1x
- Cleanup completes in <2s
- Zero data loss for valid worktrees
- Users can trigger cleanup via CLI or automatic via watch

**Next Steps:**
- Architecture design for cleanup system
- Quality strategy for ensuring safety
- Security review for data loss prevention
- Detailed execution plan with phases

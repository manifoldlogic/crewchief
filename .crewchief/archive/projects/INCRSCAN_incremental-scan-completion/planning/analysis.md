# Analysis: Incremental Scan Completion

## Problem Definition

The Maproom indexer has an incomplete incremental scanning feature that causes severe performance degradation. When scanning worktrees with identical code content (same git tree SHA), the system updates all ~474K chunks unnecessarily, taking 2-3 hours per worktree instead of seconds.

### Core Issue

The `worktree_index_state` table is never populated after scan operations complete. The incremental update algorithm (BRANCHX-1007) compares the current git tree SHA against the last indexed SHA stored in this table. When the table is empty, every scan is treated as a first-time index, forcing full processing.

### Impact

**User Experience:**
- Genetic optimizer genetic tests unusable (24-36 hours for 12 worktrees)
- Development workflow severely impacted
- Wasted API costs for re-generating identical embeddings
- Database thrash from unnecessary UPDATE operations

**Technical Debt:**
- Feature advertised as "incremental" but actually performs full scans
- No tree SHA tracking despite infrastructure being in place
- Comment in code indicates intentional deferral (BRANCHX-1008 TODO)

## Current State Analysis

### What Exists

**1. Database Schema (Complete)**
- `worktree_index_state` table with proper structure
- Functions: `get_last_indexed_tree()` and `update_index_state()`
- Migrations applied successfully (migration 0020)

**2. Incremental Algorithm (90% Complete)**
- `incremental::incremental_update()` function exists
- Git tree SHA comparison logic implemented
- `git diff-tree` integration for changed files
- Deletion handling via `remove_worktree_from_chunks()`

**3. CLI Interface (Complete)**
- `--force` flag to bypass incremental mode
- Logging indicates scan mode to user
- Help text documents incremental vs full scan

### What's Missing

**Critical Gap:** Scan command never calls `update_index_state()` after completion.

**Lines 316-320 in `/crates/maproom/src/incremental/tree_sha_update.rs`:**
```rust
// TODO (BRANCHX-1008): Uncomment this after file processing is implemented
// Currently commented to avoid database inconsistency
// update_index_state(client, worktree_id, &current_tree, &stats)
//     .await
//     .context("Failed to update index state")?;
```

**Root Cause:** The `incremental_update()` function was never integrated into the scan command's execution path. The scan command (lines 542-654 in `main.rs`) calls:
- Sequential: `indexer::scan_worktree()`
- Parallel: `indexer::scan_worktree_parallel()`

Neither function updates the `worktree_index_state` table after completion.

### Architectural Context

**Scan Flow:**
```
main.rs:scan command
  ↓
indexer::scan_worktree() or indexer::scan_worktree_parallel()
  ↓
Parse files → Extract chunks → Upsert to DB
  ↓
auto_generate_embeddings() [if enabled]
  ↓
[MISSING: update_index_state()]
```

**Should Be:**
```
main.rs:scan command
  ↓
Get current tree SHA
  ↓
Check against last_tree_sha from worktree_index_state
  ↓
If same: skip scan entirely (0-10ms)
If different: Process changed files only
  ↓
Update worktree_index_state with new tree SHA
```

## Industry Solutions

### Content-Addressed Storage
- Git itself uses tree SHAs for efficient diffing
- Docker uses layer hashing for image caching
- Nix package manager uses content hashing

### Incremental Build Systems
- Bazel: Compares file hashes, rebuilds only changed targets
- Buck: Content-based caching with remote cache support
- Gradle: Task output caching based on input hashes

### Database Indexing
- PostgreSQL: Incremental VACUUM, not full table rewrites
- Elasticsearch: Incremental index updates via `_update` API
- MongoDB: Change streams for incremental synchronization

**Key Insight:** All successful incremental systems store "last known good state" and compare before processing. Our system has the comparison logic but never records the state.

## Research Findings

### Why Was This Deferred?

**INCREMENTAL_INTEGRATION_NOTE.md** reveals:
- BRANCHX-1011 added the `--force` flag infrastructure
- Actual integration deferred due to architectural complexity
- Required refactoring `scan_worktree()` to support pluggable file discovery
- Progress tracking needs adaptation for dynamic file lists

**However:** This analysis identified a much simpler fix. We don't need to refactor the entire scan pipeline. We just need to:
1. Get the git tree SHA before scanning
2. Check if it matches the last indexed SHA
3. Skip scan if unchanged
4. Update state after scan completes

### Performance Gains

**Current Performance (without tree SHA check):**
- Identical worktree scan: 2-3 hours (updates all 474K chunks)
- Cost: $20-30 in redundant embedding API calls
- Database load: Unnecessary UPDATE operations

**Expected Performance (with tree SHA check):**
- Identical worktree scan: 5-10ms (just tree SHA comparison + DB query)
- Changed files only: Proportional to changes (typically 100ms - 5s)
- First-time index: Same as current (10s - 5min)

**5,000-20,000x speedup** for unchanged worktrees.

## Constraints

### Must Maintain

**1. Existing Behavior Unchanged:**
- `--force` flag must continue to work (full scan)
- Default behavior becomes true incremental (as documented)
- Parallel and sequential scan modes both supported

**2. Database Compatibility:**
- No schema changes required (table exists)
- Use existing `update_index_state()` function
- Handle NULL/missing state gracefully (first-time case)

**3. Error Handling:**
- Git command failures should not block scans
- Missing tree SHA should default to full scan (safe fallback)
- Database errors updating state should warn but not fail scan

### Technical Requirements

**1. Git Integration:**
- Must use `git rev-parse HEAD^{tree}` for tree SHA
- Must handle bare repositories, worktrees, and detached HEADs
- Must work across all supported platforms (Linux, macOS, Windows)

**2. Database Operations:**
- Must get worktree ID from repo/worktree names
- Must query `worktree_index_state` before scan
- Must update state after successful scan
- Must be transactional (atomic with scan completion)

**3. Metrics & Observability:**
- Must track files/chunks/embeddings processed
- Must report whether scan was skipped/incremental/full
- Must log tree SHA comparisons for debugging

## Success Criteria

1. **Unchanged worktrees skip scanning** (< 1 second check)
2. **Changed worktrees process only changed files** (proportional performance)
3. **First-time scans work as before** (backward compatible)
4. **`--force` flag overrides incremental** (explicit full scan)
5. **State persists after scan** (`worktree_index_state` populated)
6. **Genetic optimizer runs complete** (12 worktrees < 2 minutes setup)

## Risk Assessment

### Low Risk
- Function `update_index_state()` is well-tested (BRANCHX-1006)
- Tree SHA comparison is deterministic
- Falling back to full scan is always safe

### Medium Risk
- Getting worktree ID requires database query (could fail)
- Git commands could fail in edge cases (bare repos, corrupt repos)
- Multiple scans in parallel could race on state updates

### Mitigation Strategies
1. **Defensive programming:** Treat any error as "no cached state" → full scan
2. **Clear logging:** Debug output for SHA comparisons and decisions
3. **Idempotent updates:** `ON CONFLICT DO UPDATE` handles races
4. **Test coverage:** Integration tests for all scan modes

## Recommendations

### Minimal Fix (This Project)

**Scope:** Add tree SHA check and state update to scan command.

**Changes Required:**
1. Get git tree SHA before scan
2. Query `worktree_index_state` for last SHA
3. If match (and not --force): skip scan, return immediately
4. After scan completes: update state with new SHA + stats
5. Handle errors gracefully (fallback to full scan)

**Estimated Effort:** 2-4 hours (1-2 small tickets)
**Testing:** Integration tests + manual verification with genetic optimizer

### Future Enhancements (Out of Scope)

- Refactor `scan_worktree()` for pluggable file discovery (BRANCHX-1008)
- Integrate `incremental::incremental_update()` as default (proper diff-tree)
- Parallel tree SHA checks for multiple worktrees
- Remote caching of tree SHA state (distributed teams)

**Decision:** Ship the minimal fix now. Defer complex refactoring until we have data on real-world usage patterns.

## Conclusion

This is not a complex architectural problem requiring extensive refactoring. It's a simple bug: **we forgot to save the state after scanning**.

The infrastructure exists. The functions are tested. We just need to wire them together at the scan command level.

**Fix:** 20 lines of code in the right place will unlock 10,000x performance improvement for the common case (unchanged worktrees).

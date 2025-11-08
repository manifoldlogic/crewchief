# BRANCHX: Branch-Aware Indexing

**Status**: Planning Complete
**Slug**: BRANCHX
**Timeline**: 5-6 days
**Dependencies**: BLOBSHA (Content-Addressed Chunk Storage)
**Blocks**: BRWATCH (Branch Switch Detection)

## Problem Statement

After BLOBSHA, we have:
✅ Content-addressed storage with deduplication
✅ Embeddings shared across identical chunks

But we're missing:
❌ No way to query "code in branch X"
❌ Must rescan entire repository on every update
❌ Can't detect if content actually changed vs. just checkout

## Proposed Solution

Add **worktree tracking** and **incremental updates** using Git's tree SHA:

1. **Worktree Tracking**: JSONB array in chunks showing which worktrees contain each chunk
2. **Git Tree SHA**: Compare tree hashes to detect changes instantly
3. **Incremental Updates**: Only rescan files that changed (via `git diff-tree`)
4. **Branch Filtering**: Query specific branch(es) in search

## Success Metrics

- **Tree SHA optimization**: <100ms to detect "no changes"
- **Incremental efficiency**: 5-10x faster for typical branch switch (20% changed)
- **Correctness**: Incremental updates === full scans
- **Query filtering**: Search returns only specified worktree's code

## Architecture

### Schema Changes

```sql
-- Track which worktrees contain each chunk
ALTER TABLE chunks ADD COLUMN worktree_ids JSONB;  -- [1, 2, 5]
CREATE INDEX ON chunks USING gin(worktree_ids);

-- Track indexed state per worktree
CREATE TABLE worktree_index_state (
  worktree_id INT PRIMARY KEY,
  last_tree_sha TEXT,           -- Git tree SHA of last index
  last_indexed TIMESTAMP
);
```

### Incremental Update Algorithm

```
1. Get current git tree SHA (git rev-parse HEAD^{tree})
2. Compare to last indexed tree SHA
3. If identical → SKIP (instant!)
4. If different → git diff-tree to find changed files
5. Only rescan changed files
6. Update tree SHA in database
```

### Example: Branch Switch

```bash
git checkout main
# maproom: Tree SHA = abc123 (matches last indexed) → Skip

git checkout feature
# maproom: Tree SHA = def456 (different from last indexed)
#          git diff-tree finds 100 changed files (out of 1,000)
#          Rescan only 100 files
#          Time: 20 seconds (vs. 5 minutes for full scan)
```

## Implementation Phases

### Phase 1: Worktree Tracking Schema (Days 1-2)
- Add `worktree_ids` JSONB column
- Create `worktree_index_state` table
- Backfill existing chunks
- GIN index for JSONB queries

**Deliverable**: Database schema ready

### Phase 2: Git Integration (Days 2-3)
- Implement `get_git_tree_sha()`
- Implement `git_diff_tree()`
- Database functions for index state
- Unit tests for git integration

**Deliverable**: Git functions working

### Phase 3: Incremental Update Logic (Days 3-4)
- `incremental_update()` algorithm
- Update `upsert_chunk_with_worktree()`
- Handle file deletions
- Integration tests (correctness!)

**Deliverable**: Incremental updates working

### Phase 4: CLI Updates (Day 5)
- `maproom scan` uses incremental by default
- `maproom scan --force` for full scan
- MCP search accepts `worktree` parameter
- E2E tests

**Deliverable**: User-facing features complete

### Phase 5: Documentation (Day 6)
- Architecture documentation
- Changelog
- Buffer for issues

## Testing Strategy

### Critical Path Tests
1. ✅ `test_incremental_equals_full_scan` - Correctness guarantee
2. ✅ `test_tree_sha_skip_unchanged` - Optimization works
3. ✅ `test_worktree_filtering` - Query correctness
4. ✅ `test_git_diff_tree_detection` - Change detection

### Performance Benchmarks
- Tree SHA check: <10ms
- Incremental (20% changed): 5-10x faster than full
- Tree SHA skip (no changes): <100ms

## Agent Assignments

1. **database-engineer** - Schema, migrations
2. **rust-indexer-engineer** - Git integration, incremental logic, CLI
3. **general-purpose** - MCP updates, docs
4. **unit-test-runner** - Test execution
5. **verify-ticket** - Final verification
6. **commit-ticket** - Commit

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Incremental != Full scan | High | Extensive testing, comparison tests |
| Git command errors | Medium | Error handling, fallback to full scan |
| JSONB performance | Medium | GIN index, tested at scale |

## Dependencies

**Requires**: BLOBSHA complete
- Needs blob SHA for deduplication
- Needs cache-aware upsert logic
- Needs code_embeddings table

**Provides**: Foundation for BRWATCH
- Incremental update API
- Worktree-filtered queries

## Expected Outcomes

### Performance Improvements
- Branch switch: 5 minutes → 20 seconds (15x faster)
- Return to cached branch: <1 second (instant)
- No changes detection: <100ms

### Developer Experience
```bash
# Before: Full rescan every time
git checkout feature
maproom scan  # 5 minutes

# After: Incremental by default
git checkout feature
maproom scan  # 20 seconds (only changed files)

git checkout main  # Already indexed
maproom scan  # <1 second (tree SHA match, skipped)
```

### Query Flexibility
```typescript
// Search specific branch
search({ query: 'authentication', worktree: 'main' })

// Search multiple branches
search({ query: 'authentication', worktrees: ['main', 'develop'] })

// Deduplicated cross-branch
search({ query: 'authentication', worktrees: ['*'], deduplicate: true })
```

## Acceptance Criteria

- [ ] All phases complete
- [ ] All tests passing
- [ ] Incremental === full scan (correctness verified)
- [ ] Tree SHA optimization working (skip unchanged)
- [ ] Performance benchmarks met
- [ ] Query filtering by worktree works
- [ ] Documentation updated
- [ ] Manual testing complete

**Timeline**: 5-6 days (1 buffer day)

---

**Next Steps**: Generate tickets using `/create-project-tickets BRANCHX`

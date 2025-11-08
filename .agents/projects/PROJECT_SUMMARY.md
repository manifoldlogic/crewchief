# Branch-Aware Indexing: Project Breakdown

**Created**: 2025-11-08
**Source**: `.agents/research/branch-aware-indexing-architecture.md`
**Evaluation Framework**: `.agents/reference/project-boundry-evaluation.md`

## Project Boundary Decision

The branch-aware indexing architecture was **split into 3 sequential projects** based on the Stable Context Triangle:

1. **Interface Stability** ✅ - Each project builds on stable interfaces from previous
2. **Context Coherence** ✅ - Each project has <10 core concepts, focused scope
3. **Testable Completion** ✅ - Each has binary success criteria

## Project Sequence

```
┌─────────────────────────────────────────────────────────┐
│  BLOBSHA: Content-Addressed Chunk Storage (6-7 days)   │
│  Foundation: Blob SHA computation, deduplication        │
└─────────────────────┬───────────────────────────────────┘
                      │ Provides: blob_sha, code_embeddings
                      ▼
┌─────────────────────────────────────────────────────────┐
│  BRANCHX: Branch-Aware Indexing (5-6 days)             │
│  Tracking: Worktree IDs, incremental updates           │
└─────────────────────┬───────────────────────────────────┘
                      │ Provides: incremental_update API
                      ▼
┌─────────────────────────────────────────────────────────┐
│  BRWATCH: Branch Switch Detection (3-4 days)           │
│  Automation: File watching, auto-triggering            │
└─────────────────────────────────────────────────────────┘

Total Timeline: 14-17 days
```

## Project 1: BLOBSHA - Content-Addressed Chunk Storage

**Slug**: BLOBSHA
**Timeline**: 6-7 days
**Location**: `.agents/projects/BLOBSHA_content-addressed-chunk-storage/`

### Scope
- Implement Git blob SHA computation (Rust + PostgreSQL)
- Create `code_embeddings` table for deduplicated storage
- Add `blob_sha` column to `chunks` table
- Migrate existing embeddings
- Update queries to use JOIN

### Success Metrics
- Zero data loss during migration
- 70-90% deduplication for typical branch overlap
- Query performance within 10% of baseline
- Cache hit rate measurable

### Key Deliverables
- `compute_blob_sha()` function (Rust + SQL)
- Database migration scripts (4 phases)
- Cache-aware upsert logic
- Comprehensive test suite

### Blocks
- BRANCHX (needs blob SHA foundation)

---

## Project 2: BRANCHX - Branch-Aware Indexing

**Slug**: BRANCHX
**Timeline**: 5-6 days
**Location**: `.agents/projects/BRANCHX_branch-aware-indexing/`

### Scope
- Add `worktree_ids` JSONB column to chunks
- Create `worktree_index_state` table
- Implement git tree SHA comparison
- Build incremental update algorithm
- Add `git diff-tree` integration

### Success Metrics
- Tree SHA check: <10ms
- Incremental update 5-10x faster than full scan
- Incremental === full scan (correctness)
- Query filtering by worktree works

### Key Deliverables
- Worktree tracking schema
- `incremental_update()` function
- Git integration (tree SHA, diff-tree)
- CLI updates (`maproom scan`)

### Dependencies
- BLOBSHA (requires blob SHA and code_embeddings)

### Blocks
- BRWATCH (needs incremental update API)

---

## Project 3: BRWATCH - Branch Switch Detection

**Slug**: BRWATCH
**Timeline**: 3-4 days
**Location**: `.agents/projects/BRWATCH_branch-switch-detection/`

### Scope
- Implement `.git/HEAD` file watcher
- Auto-trigger incremental updates on branch switch
- Add `maproom watch` CLI command
- Graceful error handling and shutdown

### Success Metrics
- Detection latency: <1 second
- Update latency: <1 minute (via incremental)
- CPU idle: <5%
- Memory: <20MB
- Reliability: 100% detection

### Key Deliverables
- `BranchWatcher` implementation (notify crate)
- CLI watch command
- Error handling and retry logic
- Long-running stability

### Dependencies
- BRANCHX (requires incremental_update API)

---

## Why This Split Works

### Interface Stability ✅

**BLOBSHA → BRANCHX**:
- BLOBSHA provides stable `blob_sha` column
- BLOBSHA provides stable `code_embeddings` table
- BRANCHX can rely on these existing

**BRANCHX → BRWATCH**:
- BRANCHX provides stable `incremental_update()` API
- BRWATCH simply calls this function
- No interface changes needed

### Context Coherence ✅

**BLOBSHA** (13 concepts):
- Blob SHA, embeddings, deduplication, cache, migration, HNSW index, model version, foreign keys, SQL functions, upsert logic, query JOINs, cost savings, storage efficiency

**BRANCHX** (10 concepts):
- Worktrees, tree SHA, diff-tree, incremental updates, JSONB arrays, GIN index, file changes, branch filtering, index state, git integration

**BRWATCH** (7 concepts):
- File watching, branch detection, auto-triggering, notify crate, debouncing, error handling, graceful shutdown

Each project fits comfortably in an agent's working memory.

### Testable Completion ✅

**BLOBSHA**:
- ✓ All chunks have blob_sha
- ✓ Embeddings deduplicated (count < chunks)
- ✓ Cache hit rate measurable
- ✓ Queries return same results

**BRANCHX**:
- ✓ Worktree tracking works
- ✓ Incremental === full scan
- ✓ Tree SHA skip working
- ✓ Query filtering works

**BRWATCH**:
- ✓ Branch switches detected (100%)
- ✓ Auto-update triggered
- ✓ Resource usage acceptable
- ✓ Graceful shutdown works

## Expected Outcomes

### Cost Savings
**Example: 10 branches, 80% overlap**
- Without dedup: $10.00 (500k embeddings)
- With dedup: $2.00 (100k embeddings)
- **Savings: $8.00 per cycle (80%)**

### Performance Improvements
- **Initial branch**: 5-10 minutes (full scan)
- **Branch switch**: 20 seconds (incremental)
- **Return to cached**: <1 second (tree SHA match)

### Developer Experience

**Before** (manual):
```bash
git checkout feature
maproom scan --worktree feature  # Manual, 5 minutes
```

**After** (automatic):
```bash
git checkout feature  # maproom watch auto-updates in 20s
```

## Project Status

- [x] BLOBSHA - Planning complete
- [x] BRANCHX - Planning complete
- [x] BRWATCH - Planning complete
- [ ] BLOBSHA - Implementation
- [ ] BRANCHX - Implementation
- [ ] BRWATCH - Implementation

## Next Steps

1. Review planning documents for all three projects
2. Generate tickets: `/create-project-tickets BLOBSHA`
3. Execute: `/work-on-project BLOBSHA`
4. After BLOBSHA complete, move to BRANCHX
5. After BRANCHX complete, move to BRWATCH

## Documentation Location

Each project has complete planning in its directory:

```
.agents/projects/
├── BLOBSHA_content-addressed-chunk-storage/
│   ├── README.md
│   ├── planning/
│   │   ├── analysis.md
│   │   ├── architecture.md
│   │   ├── quality-strategy.md
│   │   ├── security-review.md
│   │   └── plan.md
│   └── tickets/  (to be generated)
│
├── BRANCHX_branch-aware-indexing/
│   ├── README.md
│   ├── planning/
│   │   ├── analysis.md
│   │   ├── architecture.md
│   │   ├── quality-strategy.md
│   │   ├── security-review.md
│   │   └── plan.md
│   └── tickets/  (to be generated)
│
└── BRWATCH_branch-switch-detection/
    ├── README.md
    ├── planning/
    │   ├── analysis.md
    │   ├── architecture.md
    │   ├── quality-strategy.md
    │   ├── security-review.md
    │   └── plan.md
    └── tickets/  (to be generated)
```

## Validation

This split satisfies all criteria from `.agents/reference/project-boundry-evaluation.md`:

✅ **Interface Stability** - All external interfaces documented and stable
✅ **Context Coherence** - Each project <20 concepts, <500 words to explain
✅ **Testable Completion** - Binary pass/fail for each project
✅ **Architectural Cohesion** - Each focuses on one architectural layer
✅ **Domain Unity** - Single domain per project
✅ **Independent Value** - Each delivers standalone value

**Total Timeline**: 14-17 days (with built-in buffers)
**Sequential Dependencies**: Must be done in order (BLOBSHA → BRANCHX → BRWATCH)

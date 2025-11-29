# Analysis: Content-Addressed Chunk Storage

## Problem Definition

The current Maproom indexing system stores embeddings directly in the `chunks` table, leading to massive duplication across branches and wasted costs:

### Current State Pain Points

1. **No deduplication**: Same function in multiple branches = multiple embeddings
2. **Redundant API costs**: 500,000 chunks across 10 branches, but 400,000 are duplicates
   - Current cost: $10.00 (500k × $0.00002)
   - Potential cost: $2.00 (100k × $0.00002)
   - **Waste: $8.00 per index cycle (80%)**

3. **Storage bloat**: Embeddings stored redundantly
   - Without dedup: 3GB for 10 branches
   - With dedup: 840MB (72% reduction)

4. **Slow refactoring**: Moving a function between files regenerates embedding
   - Content identical, but stored separately
   - Unnecessary API calls and storage

### Root Cause

The fundamental issue is **location-based storage** rather than **content-based storage**:

```sql
-- Current: Embedding tied to location
chunks (
  chunk_id UUID PRIMARY KEY,
  file_id INT,              -- Location-based
  embedding vector(1536),   -- Duplicated
  content TEXT
)
```

When the same code exists in multiple places (branches, refactored files), we store separate embeddings even though the content is identical.

## Industry Context

This is a solved problem in production systems:

### Sourcegraph Zoekt
- **Approach**: Content-addressed storage with bitmask branch tracking
- **Result**: 10 branches with 90% overlap = 1.1x storage vs. single branch
- **Scale**: Handles millions of files for companies like Uber, Lyft

### GitHub Blackbird
- **Approach**: Blob-level deduplication in code search
- **Result**: 115TB → 28TB after dedup (75% savings)
- **Validation**: Powers GitHub's production code search

### Git Itself
- **Approach**: Content-addressed object storage using blob SHA
- **Result**: Efficient storage of millions of commits
- **Key Insight**: Same file content = same blob SHA, stored once

## Content Addressing Fundamentals

### What is a Blob SHA?

Git's blob SHA is simply a **hashing algorithm applied to content**:

```rust
fn compute_blob_sha(content: &str) -> String {
    let mut hasher = Sha256::new();

    // Git blob format: "blob <size>\0<content>"
    hasher.update(b"blob ");
    hasher.update(content.len().to_string().as_bytes());
    hasher.update(b"\0");
    hasher.update(content.as_bytes());

    format!("{:x}", hasher.finalize())
}
```

### Key Insight: Granularity is Our Choice

Git applies this to **whole files**. We can apply it to **tree-sitter chunks**:

```rust
// Git's granularity
let file_sha = compute_blob_sha(&file_content);

// Our granularity (finer-grained)
let chunk_sha = compute_blob_sha(&chunk.content);
```

**Why this matters:**

```typescript
// file.ts (1 character changes in processOrder)
function calculateTotal(items) {
  return items.reduce((sum, item) => sum + item.price, 0);
}

function processOrder(order) {
  console.log('Processing...'); // Changed line
  return calculateTotal(order.items);
}
```

- **File-level SHA**: Both functions invalidated, 2 new embeddings
- **Chunk-level SHA**: Only `processOrder` invalidated, 1 new embedding
- **Savings**: 50% reduction per file change

## Deduplication Examples

### Example 1: Branch Overlap

**Scenario**: Feature branch shares 80% code with main

```bash
# Index main: 10,000 chunks
# Cost: $0.20

# Switch to feature: 2,000 changed chunks, 8,000 identical
# Without dedup: Generate 10,000 embeddings = $0.20
# With dedup: Generate 2,000 embeddings = $0.04
# Savings: $0.16 (80%)
```

### Example 2: Refactoring

**Before**:
```typescript
// utils.ts
function calculateTotal(items) {
  return items.reduce((sum, item) => sum + item.price, 0);
}
```

**After**:
```typescript
// math.ts (moved file)
function calculateTotal(items) {
  return items.reduce((sum, item) => sum + item.price, 0);
}
```

- Content identical → blob SHA unchanged
- Embedding reused from cache
- Cost: $0 (no API call)

### Example 3: Large File Partial Change

**File**: 50 functions, 1 function changes

- Without chunk-level dedup: 50 embeddings regenerated
- With chunk-level dedup: 1 embedding regenerated
- Savings: 98% (49/50)

## Current System Analysis

### Existing Schema

```sql
CREATE TABLE chunks (
  chunk_id UUID PRIMARY KEY,
  file_id INT,
  start_line INT,
  end_line INT,
  symbol_name TEXT,
  chunk_type TEXT,
  content TEXT,
  embedding vector(1536),  -- 6KB per chunk
  created_at TIMESTAMP,
  updated_at TIMESTAMP
);
```

**Problems**:
1. `embedding` stored per chunk, no sharing
2. No content-based lookup (no blob SHA)
3. No way to detect duplicate content
4. No model versioning for embedding upgrades

### What Needs to Change

**Core requirement**: Separate **content** (chunks) from **embeddings** (expensive, deduplicated)

**New architecture**:
```sql
-- Deduplicated embedding cache (content-addressed)
CREATE TABLE code_embeddings (
  blob_sha TEXT PRIMARY KEY,
  embedding vector(1536),
  model_version TEXT,
  created_at TIMESTAMP
);

-- Chunks reference embeddings by content hash
CREATE TABLE code_chunks (
  chunk_id UUID PRIMARY KEY,
  blob_sha TEXT REFERENCES code_embeddings(blob_sha),
  file_path TEXT,
  -- ... other metadata
);
```

## Research Insights

### From `.crewchief/research/branch-aware-indexing-industry-research.md`

The research document validates:

1. **Content addressing is battle-tested** (Git, Sourcegraph, GitHub)
2. **Blob SHA provides perfect deduplication** (bit-for-bit identical content)
3. **Storage and cost savings are massive** (70-90% typical)
4. **Implementation is straightforward** (standard hashing algorithm)

### Key Findings

- **Sourcegraph**: Uses bitmask + content dedup for multi-branch indexing
- **GitHub Blackbird**: Achieved 75% storage reduction via blob dedup
- **Git internals**: Decades of proven content-addressed storage

## Success Criteria

This project is complete when:

1. **Blob SHA computed for all chunks**
   - Test: All chunks have non-null blob_sha
   - Verification: `SELECT COUNT(*) FROM chunks WHERE blob_sha IS NULL` = 0

2. **Embeddings deduplicated**
   - Test: Identical content shares one embedding
   - Verification: Insert duplicate chunk, verify no new embedding generated

3. **Cache hit rate measurable**
   - Test: Metrics show cache hits vs. misses
   - Verification: Reindex same content, 100% cache hit rate

4. **Cost savings demonstrated**
   - Test: Reindex branch with overlap
   - Verification: Embedding API calls reduced by overlap percentage

5. **Migration successful**
   - Test: All existing chunks migrated
   - Verification: Old and new queries return same results

## Out of Scope

This project focuses solely on content-addressed storage foundation:

**Not included**:
- Branch tracking (JSONB worktree_ids) → BRANCHX project
- Incremental updates → BRANCHX project
- Automatic branch switch detection → BRWATCH project
- Search query changes → Future work

**Why**: These features depend on the blob SHA foundation being stable

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Blob SHA collisions | Very Low | High | Use SHA-256 (cryptographically secure) |
| Migration breaks queries | Medium | High | Extensive testing, gradual rollout |
| Performance regression | Low | Medium | Benchmark queries before/after |
| Disk space during migration | Medium | Low | Migrate in batches, clean up old data |

## Next Steps

1. Design database schema (architecture.md)
2. Plan migration strategy (architecture.md)
3. Define test strategy (quality-strategy.md)
4. Create implementation plan (plan.md)

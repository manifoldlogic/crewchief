# Ticket: BRANCHX-1008: Update chunk upsert to track worktree_ids

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Modify the chunk upsert function to add worktree IDs to the worktree_ids JSONB array, enabling multi-branch tracking. This creates an idempotent upsert operation that appends worktree IDs to chunks without duplication.

## Background
This is Phase 3, Step 3.2 of BRANCHX. The incremental update algorithm (BRANCHX-1007) processes changed files and identifies chunks that need updating, but the upsert logic needs to properly add the current worktree to each chunk's worktree_ids array. This enables the same chunk content to be tracked across multiple branches.

The worktree_ids JSONB column was added in BRANCHX-1001, and BLOBSHA provides the blob_sha computation and embedding deduplication infrastructure that this upsert builds upon.

Reference: `.agents/projects/BRANCHX_branch-aware-indexing/planning/plan.md` - Phase 3.2 (Upsert Chunks with Worktree Tracking)

## Acceptance Criteria
- [ ] `upsert_chunk_with_worktree(pool, chunk, worktree_id)` function created in Rust
- [ ] INSERT operation adds new chunk with worktree_ids = [worktree_id]
- [ ] UPDATE (ON CONFLICT) appends worktree_id to array if not already present
- [ ] Idempotent: running twice with same worktree_id doesn't create duplicates
- [ ] Uses BLOBSHA's ensure_embedding_cached for deduplication
- [ ] Returns chunk_id (UUID) for reference
- [ ] Unit tests verify worktree array operations work correctly
- [ ] Unit tests verify idempotency (same worktree twice = no duplicates)
- [ ] Unit tests verify multi-worktree scenario (same content, different worktrees)

## Technical Requirements
- Use ON CONFLICT (blob_sha, file_path) for upsert detection
- Check if worktree already in array: `worktree_ids ? $worktree_id::TEXT`
- Append if not present: `worktree_ids || jsonb_build_array($worktree_id)`
- No-op if already present (avoid duplicates in array)
- Ensure embedding exists before upserting chunk (BLOBSHA integration)
- Update updated_at timestamp on conflict
- Function signature: `async fn upsert_chunk_with_worktree(pool: &PgPool, chunk: &ParsedChunk, worktree_id: i32) -> Result<Uuid>`
- Return chunk_id after INSERT or UPDATE

## Implementation Notes

Add to `crates/maproom/src/upsert.rs` or create new file if it doesn't exist:

```rust
use crate::blob_sha::compute_blob_sha;
use crate::embeddings::ensure_embedding_cached;
use anyhow::Result;
use sqlx::PgPool;
use uuid::Uuid;

pub struct ParsedChunk {
    pub file_path: String,
    pub symbol_name: Option<String>,
    pub content: String,
    pub start_line: i32,
    pub end_line: i32,
}

pub async fn upsert_chunk_with_worktree(
    pool: &PgPool,
    chunk: &ParsedChunk,
    worktree_id: i32,
) -> Result<Uuid> {
    // Compute blob SHA (from BLOBSHA)
    let blob_sha = compute_blob_sha(&chunk.content);

    // Ensure embedding exists (from BLOBSHA)
    ensure_embedding_cached(pool, &blob_sha, &chunk.content).await?;

    // Upsert chunk, adding this worktree to worktree_ids
    let chunk_id = sqlx::query_scalar!(
        r#"
        INSERT INTO chunks (blob_sha, file_path, symbol_name, content, start_line, end_line, worktree_ids)
        VALUES ($1, $2, $3, $4, $5, $6, jsonb_build_array($7))
        ON CONFLICT (blob_sha, file_path)
        DO UPDATE SET
          worktree_ids = CASE
            WHEN chunks.worktree_ids ? $7::TEXT THEN chunks.worktree_ids
            ELSE chunks.worktree_ids || jsonb_build_array($7)
          END,
          updated_at = NOW()
        RETURNING chunk_id
        "#,
        blob_sha,
        chunk.file_path,
        chunk.symbol_name,
        chunk.content,
        chunk.start_line,
        chunk.end_line,
        worktree_id,
    )
    .fetch_one(pool)
    .await?;

    Ok(chunk_id)
}
```

**Key Implementation Details**:
- The CASE statement checks if worktree_id is already in the array using JSONB's `?` operator
- If present, keep existing array unchanged
- If not present, use `||` operator to append new worktree_id
- blob_sha and file_path form the unique constraint for conflict detection
- updated_at timestamp ensures we track when chunk was last seen in any worktree

**Testing Strategy**:
Create unit tests that verify:
1. New chunk insertion creates array with single worktree_id
2. Conflict with same worktree_id is idempotent (no duplicate)
3. Conflict with different worktree_id appends to array
4. Multiple conflicts build correct array: [wt1, wt2, wt3]

See `architecture.md` section "Upsert Chunk with Worktree Tracking" for complete design and rationale.

## Dependencies
- **BRANCHX-1001**: Complete - worktree_ids column exists in chunks table
- **BLOBSHA**: Complete - blob_sha computation and ensure_embedding_cached function available
- **BRANCHX-1007**: In progress - incremental update algorithm will call this function

This ticket can be implemented independently of BRANCHX-1007 as long as the function interface matches what the incremental update algorithm expects.

## Risk Assessment
- **Risk**: JSONB contains check (`?`) doesn't work as expected, creates duplicates in array
  - **Mitigation**: Unit test specifically verifying idempotency - inserting same worktree_id twice should not create duplicate entries

- **Risk**: ON CONFLICT logic incorrect, loses existing worktree_ids or overwrites incorrectly
  - **Mitigation**: Test multi-worktree scenario with step-by-step verification of array contents after each upsert

- **Risk**: JSONB array operations have unexpected behavior with integer values
  - **Mitigation**: Explicit cast to TEXT in SQL query (`$7::TEXT`), test with actual integer worktree_id values

- **Risk**: Race conditions if multiple processes upsert same chunk simultaneously
  - **Mitigation**: ON CONFLICT handles concurrent inserts safely; CASE statement in UPDATE is atomic

## Files/Packages Affected
- `crates/maproom/src/upsert.rs` (create new file or modify if exists)
- `crates/maproom/src/lib.rs` (add module declaration if creating new file)
- `crates/maproom/src/blob_sha.rs` (dependency - read only)
- `crates/maproom/src/embeddings.rs` (dependency - read only)
- Test files in `crates/maproom/tests/` or inline tests in `upsert.rs`

## Planning References
- `.agents/projects/BRANCHX_branch-aware-indexing/planning/plan.md` - Phase 3.2 (lines 171-213)
- `.agents/projects/BRANCHX_branch-aware-indexing/planning/architecture.md` - Upsert Chunk with Worktree Tracking (lines 210-256)

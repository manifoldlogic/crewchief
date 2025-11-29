# Ticket: VECFIX-1001: Remove vec_chunks code and migrate callers (ATOMIC)

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Remove deprecated `vec_chunks` functions from `mod.rs` and migrate the `pipeline.rs` caller to use the correct `store.upsert_embedding()` API in one atomic operation to prevent compilation failures.

## Background
The Maproom Rust indexer has deprecated code that references the `vec_chunks` table, which was dropped by migration 6. This causes runtime errors ("no such table: vec_chunks") when the VSCode extension attempts to scan workspaces.

Root cause: The `embedding/pipeline.rs` module calls deprecated `store.upsert_embeddings()` (plural) which tries to write to the non-existent `vec_chunks` table.

The `embeddings.rs` module already has the correct implementation (`store.upsert_embedding()` singular). This ticket updates the caller and removes deprecated code atomically to maintain compilation throughout the process.

This ticket implements the first phase of the VECFIX project plan, which removes the problematic `vec_chunks` table references entirely.

**Planning Reference**: `/workspace/.crewchief/projects/VECFIX_vec-chunks-schema-fix/planning/plan.md`

## Acceptance Criteria
- [x] `update_chunk_embeddings()` method removed from `pipeline.rs`
- [x] Call site in `pipeline.rs` updated to use `store.upsert_embedding(blob_sha, ...)`
- [x] `upsert_embeddings()` (plural) removed from `mod.rs`
- [x] `batch_upsert_embeddings()` removed from `mod.rs`
- [x] Code compiles without errors: `cargo build -p crewchief-maproom`
- [x] No unused variable warnings in modified files
- [x] No references to `vec_chunks` table remain in the codebase

## Technical Requirements

### 1. Update pipeline.rs caller (MUST DO FIRST)
**File**: `crates/maproom/src/embedding/pipeline.rs`

- Remove `update_chunk_embeddings()` method (lines 509-549)
- Update call site at lines 424-437

**Before (lines 424-437):**
```rust
for (i, chunk) in batch.iter().enumerate() {
    self.update_chunk_embeddings(store, chunk.id, &code_embeddings[i], &text_embeddings[i]).await?;
    if let Some(blob_sha) = &chunk.blob_sha {
        self.populate_embedding_cache(store, blob_sha, &code_embeddings[i]).await?;
    }
}
```

**After:**
```rust
for (i, chunk) in batch.iter().enumerate() {
    if let Some(blob_sha) = &chunk.blob_sha {
        store.upsert_embedding(blob_sha, &code_embeddings[i], &self.provider_name).await?;
    }
}
```

### 2. Remove deprecated mod.rs functions
**File**: `crates/maproom/src/db/sqlite/mod.rs`

- Remove `upsert_embeddings()` (lines 478-548) - note: plural, with 's'
- Remove `batch_upsert_embeddings()` (lines 550-620)
- **KEEP** `upsert_embedding()` (line 1778) - note: singular, uses embeddings.rs

### 3. Verify no hidden callers
Before removing functions from `mod.rs`, use `rg` to search for any other callers:
```bash
rg "upsert_embeddings\(" crates/maproom/src/
rg "batch_upsert_embeddings\(" crates/maproom/src/
```

Expected: Only finds the definitions being removed, no other call sites.

## Implementation Notes

**Critical ordering**: This ticket MUST be completed atomically to prevent compilation failures:

1. **First**: Update `pipeline.rs` call site to use correct API
2. **Second**: Remove `update_chunk_embeddings()` method from `pipeline.rs`
3. **Third**: Search for hidden callers in codebase
4. **Fourth**: Remove deprecated functions from `mod.rs`
5. **Finally**: Verify compilation with `cargo build -p crewchief-maproom`

**Key differences between APIs**:
- **Old (deprecated)**: `upsert_embeddings()` (plural) - writes to `vec_chunks` table (doesn't exist)
- **New (correct)**: `upsert_embedding()` (singular) - writes to `embeddings` table (exists)

**What gets removed**:
- Text embeddings processing (no longer needed - only code embeddings used)
- Chunk ID-based storage (replaced with blob SHA-based storage)
- Batch operations (single upsert per blob is sufficient)

**What gets kept**:
- Blob SHA-based embedding cache
- Provider name tracking
- `upsert_embedding()` singular function in `mod.rs` (delegates to `embeddings.rs`)

## Dependencies
None (first ticket in VECFIX project)

## Risk Assessment

- **Risk**: Pipeline disruption during refactor could break compilation
  - **Mitigation**: Atomic operation - update caller before removing deprecated code

- **Risk**: Hidden callers to deprecated functions not found by search
  - **Mitigation**: Use `rg` to search entire codebase before removal; compilation will catch any missed callers

- **Risk**: Loss of text embeddings functionality
  - **Mitigation**: Text embeddings were never fully utilized; code embeddings are sufficient for semantic search

## Files/Packages Affected
- `crates/maproom/src/embedding/pipeline.rs` - Remove `update_chunk_embeddings()` method, update call site
- `crates/maproom/src/db/sqlite/mod.rs` - Remove `upsert_embeddings()` and `batch_upsert_embeddings()` functions
- Package: `crewchief-maproom` (Rust crate)

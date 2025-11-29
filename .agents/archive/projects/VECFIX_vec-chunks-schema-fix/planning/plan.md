# Implementation Plan: vec_chunks Schema Fix

## Summary

Remove deprecated `vec_chunks` code from `mod.rs` and `schema.rs`, migrate the pipeline.rs caller to use the correct `embeddings.rs` module, and verify via tests.

## Phase 1: Code Cleanup and Migration

### VECFIX-1001: Remove vec_chunks code and migrate callers (ATOMIC)

**Objective**: Remove deprecated functions and update the single caller in one atomic operation to prevent compilation failures.

**Work Items**:

1. **Update pipeline.rs caller** (must do first):
   - Remove `update_chunk_embeddings()` method (lines 509-549)
   - At call site (line 424-437), the deprecated call becomes unnecessary because `populate_embedding_cache()` already handles embedding storage correctly
   - Modify the loop to only call `populate_embedding_cache()` (or inline it)

   **Before (pipeline.rs:424-437):**
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

2. **Remove deprecated mod.rs functions**:
   - Remove `upsert_embeddings()` (lines 478-548) - note: plural, with 's'
   - Remove `batch_upsert_embeddings()` (lines 550-620)
   - Keep `upsert_embedding()` (line 1778) - note: singular, uses embeddings.rs

3. **Verify compilation**:
   - Run `cargo build -p crewchief-maproom`
   - No references to `vec_chunks` in mod.rs
   - No unused variable warnings

**Agent**: `rust-indexer-engineer`

**Acceptance Criteria**:
- [ ] `update_chunk_embeddings()` method removed from pipeline.rs
- [ ] Call site updated to use `store.upsert_embedding(blob_sha, ...)`
- [ ] `upsert_embeddings()` (plural) removed from mod.rs
- [ ] `batch_upsert_embeddings()` removed from mod.rs
- [ ] Code compiles without errors
- [ ] No unused variable warnings

### VECFIX-1002: Remove vec_chunks from schema.rs

**Objective**: Clean up legacy schema definition.

**Work Items**:
1. Remove `vec_chunks` table creation (line 99)
2. Update any comments referencing old schema

**Agent**: `rust-indexer-engineer`

**Acceptance Criteria**:
- [ ] No references to `vec_chunks` in `schema.rs`
- [ ] Code compiles without errors

## Phase 2: Testing and Verification

### VECFIX-1003: Run test suite and fix failures

**Objective**: Ensure no test regressions.

**Work Items**:
1. Run `cargo test -p crewchief-maproom`
2. Fix any failing tests
3. Update tests that referenced `vec_chunks`

**Agent**: `unit-test-runner` then `rust-indexer-engineer` if fixes needed

**Acceptance Criteria**:
- [ ] All tests pass
- [ ] No tests reference `vec_chunks`

### VECFIX-1004: E2E verification

**Objective**: Verify the full scan and embedding pipeline works.

**Work Items**:
1. Run `./scripts/test_sqlite_e2e.sh`
2. Manually test embedding generation:
   ```bash
   # Fresh database
   rm -f ~/.maproom/maproom.db
   cargo run --bin crewchief-maproom -- db migrate
   cargo run --bin crewchief-maproom -- scan --path /path/to/repo --repo test --worktree main
   cargo run --bin crewchief-maproom -- generate-embeddings --repo test
   cargo run --bin crewchief-maproom -- status --repo test
   ```
3. Verify embeddings are stored in `code_embeddings` table (not vec_chunks)
4. Test VSCode extension scan

**Agent**: `verify-ticket`

**Acceptance Criteria**:
- [ ] E2E script passes
- [ ] Embedding generation completes without "no such table" errors
- [ ] VSCode extension can scan workspace
- [ ] Embeddings stored in `code_embeddings` table

## Timeline

| Phase | Tickets | Estimated Complexity |
|-------|---------|---------------------|
| Phase 1 | VECFIX-1001, 1002 | Simple (code removal + migration) |
| Phase 2 | VECFIX-1003, 1004 | Simple (verification) |

## Dependencies

- None (self-contained refactoring)

## Risks

| Risk | Mitigation |
|------|------------|
| Pipeline disruption | VECFIX-1001 is atomic - caller and deprecated code updated together |
| Test failures | Phase 2 dedicated to test fixes |
| Runtime errors | E2E verification specifically tests embedding pipeline |

## Agent Assignments

| Ticket | Primary Agent |
|--------|---------------|
| VECFIX-1001 | rust-indexer-engineer |
| VECFIX-1002 | rust-indexer-engineer |
| VECFIX-1003 | unit-test-runner |
| VECFIX-1004 | verify-ticket |

## Success Metrics

1. Zero references to `vec_chunks` in `mod.rs`, `schema.rs`, and `pipeline.rs`
2. Only `upsert_embedding()` (singular) remains - uses correct architecture
3. All tests pass
4. VSCode extension scan and embedding generation complete without errors
5. Embeddings properly stored in `code_embeddings` and synced to `vec_code`

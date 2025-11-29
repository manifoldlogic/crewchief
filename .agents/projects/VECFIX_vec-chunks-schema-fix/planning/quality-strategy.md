# Quality Strategy: vec_chunks Schema Fix

## Testing Approach

This is a **refactoring task** that removes dead code and updates callers. The testing strategy focuses on:

1. **Preventing regressions** - Ensure existing functionality still works
2. **Validating removal** - Confirm deprecated code is no longer called
3. **Integration verification** - End-to-end embedding storage works

## Test Categories

### 1. Existing Test Suite

The `embeddings.rs` module already has comprehensive tests:

- `test_vector_table_sync` - Single embedding to vec_code
- `test_vector_table_sync_update` - Update existing embedding
- `test_sync_all_embeddings_to_vec` - Batch sync operations
- `test_768_dim_embedding_storage` - Ollama dimension support
- `test_768_dim_vector_table_sync` - 768-dim to vec_code_768
- `test_mixed_dimensions_storage` - Both dimensions together
- `test_sync_all_mixed_dimensions` - Batch sync with mixed dims
- `test_unsupported_dimension` - Error handling for bad dims

**Action**: Run existing tests to ensure they pass before and after changes.

### 2. Integration Tests

#### E2E Scan Test

Verify the full indexing pipeline:

```bash
# Test with existing script
./scripts/test_sqlite_e2e.sh
```

This should:
1. Initialize database (run migrations)
2. Scan a repository
3. Generate embeddings
4. Verify search returns results

#### Embedding Pipeline Test (CRITICAL)

Specifically test the embedding generation pipeline that was modified:

```bash
# Fresh database
rm -f ~/.maproom/maproom.db

# Initialize
cargo run --bin crewchief-maproom -- db migrate

# Scan (indexes files, no embeddings yet)
cargo run --bin crewchief-maproom -- scan --path /path/to/repo --repo test --worktree main

# Generate embeddings (this uses the modified pipeline.rs code path)
cargo run --bin crewchief-maproom -- generate-embeddings --repo test

# Verify embeddings stored
sqlite3 ~/.maproom/maproom.db "SELECT COUNT(*) FROM code_embeddings;"

# Should show count > 0
```

**Why this is critical**: The main change is in `pipeline.rs`, which is the code path for `generate-embeddings`. We must verify this works.

#### Manual Verification

```bash
# 1. Fresh database
rm ~/.maproom/maproom.db

# 2. Run migrations
cargo run --bin crewchief-maproom -- db migrate

# 3. Scan a repo
cargo run --bin crewchief-maproom -- scan --path /path/to/repo --repo test --worktree main

# 4. Check status (should show indexed files)
cargo run --bin crewchief-maproom -- status --repo test

# 5. Search (FTS should work)
cargo run --bin crewchief-maproom -- search --query "function" --repo test --mode fts
```

### 3. Regression Tests

#### Caller Search

Before removing functions, search for all callers:

```bash
# Find all usages of the deprecated functions
rg "upsert_embedding|batch_upsert_embeddings" crates/maproom/src --type rust
```

Expected result: Only matches in `mod.rs` (the deprecated code) and `embeddings.rs` (the correct implementation).

#### Build Verification

```bash
# Ensure crate compiles without errors
cargo build -p crewchief-maproom

# Run clippy for warnings
cargo clippy -p crewchief-maproom
```

### 4. Test Updates

If any tests reference `vec_chunks`:

```bash
# Find test code referencing vec_chunks
rg "vec_chunks" crates/maproom/src --type rust
```

Update or remove tests that:
- Test the deprecated `vec_chunks` table directly
- Mock or stub `vec_chunks` operations

## Success Criteria

| Criterion | Verification Method |
|-----------|---------------------|
| No `vec_chunks` references in code | `rg vec_chunks src/db/sqlite/{mod,schema}.rs src/embedding/pipeline.rs` returns empty |
| All existing tests pass | `cargo test -p crewchief-maproom` |
| E2E scan works | `./scripts/test_sqlite_e2e.sh` |
| No clippy warnings | `cargo clippy -p crewchief-maproom` |
| Embedding pipeline works | `generate-embeddings` command stores to `code_embeddings` table |
| VSCode extension scans work | Manual test with extension |

## Risk Mitigation

### Risk: Hidden Callers

**Mitigation**: Use `rg` to exhaustively search for all references before removal.

### Risk: Runtime Errors

**Mitigation**: E2E test covers the full scan path that was previously failing.

### Risk: Test Drift

**Mitigation**: Migration tests already verify `vec_chunks` doesn't exist (see `migrations.rs:444-452`).

## Test Execution Plan

1. **Before changes**: Run full test suite, note current state
2. **After code removal**: Run full test suite, verify no regressions
3. **After caller updates**: Run E2E test, verify scan completes
4. **Final verification**: Manual VSCode extension test

## Coverage Expectations

This refactoring task doesn't require new test coverage. The existing `embeddings.rs` tests are comprehensive. The goal is to:

- Remove dead code that causes errors
- Ensure existing tests still pass
- Verify the fix resolves the original issue

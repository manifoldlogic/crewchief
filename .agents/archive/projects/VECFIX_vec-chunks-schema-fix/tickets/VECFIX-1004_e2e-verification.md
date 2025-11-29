# Ticket: VECFIX-1004: E2E Verification

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
- verify-ticket
- commit-ticket

## Summary

Verify the full scan and embedding pipeline works end-to-end after the code changes, ensuring the fix resolves the original "no such table: vec_chunks" errors when the VSCode extension attempts to scan workspaces.

## Background

This is the final verification ticket for the VECFIX project. After removing deprecated `vec_chunks` code (VECFIX-1001), cleaning up the schema (VECFIX-1002), and ensuring all tests pass (VECFIX-1003), we need to verify the complete flow from database initialization through embedding generation works correctly.

The original issue was VSCode extension failures due to references to the non-existent `vec_chunks` table. This ticket verifies that embeddings are now properly stored in `code_embeddings` and synced to the vector tables (`vec_code` and `vec_code_768`).

This implements the "E2E Verification" section from `VECFIX_PLAN.md` (Phase 2, VECFIX-1004).

## Acceptance Criteria

- [x] E2E script (`./scripts/test_sqlite_e2e.sh`) passes without errors
- [x] Embedding generation completes without "no such table: vec_chunks" errors
- [x] VSCode extension can successfully scan a workspace
- [x] Embeddings are stored in `code_embeddings` table (not vec_chunks)
- [x] No references to `vec_chunks` remain in `mod.rs`, `schema.rs`, or `pipeline.rs`

## Technical Requirements

- Run the E2E test script to verify full pipeline integration
- Manually test the complete embedding generation workflow with a fresh database
- Verify embeddings are stored in the correct table using SQLite queries
- Test the VSCode extension scan functionality
- Confirm zero references to deprecated `vec_chunks` code

## Implementation Notes

### E2E Test Script

Run the automated end-to-end test:
```bash
./scripts/test_sqlite_e2e.sh
```

This should:
1. Initialize database (run migrations)
2. Scan a repository
3. Generate embeddings
4. Verify search returns results

### Manual Embedding Pipeline Test

Test the embedding generation pipeline with a fresh database:

```bash
# 1. Fresh database
rm -f ~/.maproom/maproom.db

# 2. Initialize database
cargo run --bin crewchief-maproom -- db migrate

# 3. Scan a repository (indexes files, no embeddings yet)
cargo run --bin crewchief-maproom -- scan --path /path/to/repo --repo test --worktree main

# 4. Generate embeddings (modified pipeline.rs code path)
cargo run --bin crewchief-maproom -- generate-embeddings --repo test

# 5. Check status
cargo run --bin crewchief-maproom -- status --repo test

# 6. Verify embeddings stored in correct table
sqlite3 ~/.maproom/maproom.db "SELECT COUNT(*) FROM code_embeddings;"
# Should return count > 0
```

### Verification Commands

```bash
# Verify no vec_chunks references remain
rg vec_chunks crates/maproom/src/db/sqlite/{mod,schema}.rs crates/maproom/src/embedding/pipeline.rs
# Should return empty (no matches)

# Verify embedding storage
sqlite3 ~/.maproom/maproom.db "SELECT COUNT(*) FROM code_embeddings;"
# Should show count > 0 after generate-embeddings
```

### VSCode Extension Test

Manually test the VSCode extension:
1. Open a workspace in VSCode with the crewchief extension
2. Trigger a workspace scan
3. Verify no "no such table: vec_chunks" errors appear
4. Confirm embeddings are generated successfully

## Dependencies

- **VECFIX-1001**: Code removal and migration must be complete
- **VECFIX-1002**: Schema cleanup must be complete
- **VECFIX-1003**: All tests must pass

This ticket should only be executed after all Phase 1 tickets (VECFIX-1001, VECFIX-1002) and VECFIX-1003 are verified complete.

## Risk Assessment

- **Risk**: E2E test may fail due to environmental differences or missing dependencies
  - **Mitigation**: Test uses a fresh database and self-contained test data; script should handle setup

- **Risk**: VSCode extension test requires manual verification
  - **Mitigation**: Document exact steps for reproducible testing; verify error logs for absence of vec_chunks errors

- **Risk**: Embeddings may fail to sync to vector tables
  - **Mitigation**: Query both `code_embeddings` and `vec_code`/`vec_code_768` tables to verify sync

## Files/Packages Affected

### Files to Verify
- `crates/maproom/src/db/sqlite/mod.rs` - Should have no vec_chunks references
- `crates/maproom/src/db/sqlite/schema.rs` - Should have no vec_chunks references
- `crates/maproom/src/embedding/pipeline.rs` - Should have no vec_chunks references

### Scripts to Execute
- `./scripts/test_sqlite_e2e.sh` - E2E test script

### Database Tables to Verify
- `code_embeddings` - Should contain generated embeddings
- `vec_code` - Should contain synced 1536-dim vectors (if OpenAI used)
- `vec_code_768` - Should contain synced 768-dim vectors (if Ollama used)

### External Integration
- VSCode extension workspace scanning

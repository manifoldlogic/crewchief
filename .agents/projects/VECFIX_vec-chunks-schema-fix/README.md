# VECFIX: vec_chunks Schema Fix

## Problem Statement

The Maproom Rust indexer has deprecated code in `mod.rs` that references the `vec_chunks` table, which was dropped by migration 6. This causes runtime errors ("no such table: vec_chunks") when the VSCode extension attempts to scan workspaces.

**Root cause**: The `embedding/pipeline.rs` module calls `store.upsert_embeddings()` (deprecated), which tries to write to the non-existent `vec_chunks` table.

## Proposed Solution

1. **Migrate pipeline.rs**: Update the embedding pipeline to use the correct `store.upsert_embedding()` API (singular), which stores embeddings by `blob_sha` in `code_embeddings` table
2. **Remove deprecated code**: Delete `upsert_embeddings()` and `batch_upsert_embeddings()` from `mod.rs`
3. **Clean up schema.rs**: Remove legacy `vec_chunks` table creation

## Project Scope

**In Scope**:
- Remove deprecated `vec_chunks` functions from `mod.rs`
- Migrate `pipeline.rs` caller to use correct API
- Remove legacy `vec_chunks` table creation from `schema.rs`
- Test verification

**Out of Scope**:
- New database migrations (not needed)
- Schema changes (migration 6 already handles this)
- New features

## Planning Documents

- [Analysis](planning/analysis.md) - Problem investigation, root cause, and active callers
- [Architecture](planning/architecture.md) - Solution design with migration path
- [Quality Strategy](planning/quality-strategy.md) - Testing approach including pipeline verification
- [Security Review](planning/security-review.md) - Security assessment
- [Plan](planning/plan.md) - Implementation phases and tickets

## Tickets

| ID | Description | Agent |
|----|-------------|-------|
| VECFIX-1001 | Remove vec_chunks code and migrate callers (atomic) | rust-indexer-engineer |
| VECFIX-1002 | Remove vec_chunks from schema.rs | rust-indexer-engineer |
| VECFIX-1003 | Run test suite and fix failures | unit-test-runner |
| VECFIX-1004 | E2E verification | verify-ticket |

## Relevant Agents

- **rust-indexer-engineer**: Primary agent for code changes
- **unit-test-runner**: Test execution
- **verify-ticket**: Final verification

## Files Affected

- `crates/maproom/src/db/sqlite/mod.rs` - Remove deprecated functions
- `crates/maproom/src/db/sqlite/schema.rs` - Remove legacy table definition
- `crates/maproom/src/embedding/pipeline.rs` - Migrate to correct API

## Success Criteria

1. No `vec_chunks` references in affected files
2. Only `upsert_embedding()` (singular) remains in mod.rs - uses correct architecture
3. All tests pass
4. VSCode extension scan works without errors
5. Embeddings stored correctly in `code_embeddings` table

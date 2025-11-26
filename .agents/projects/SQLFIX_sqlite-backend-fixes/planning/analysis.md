# Analysis: SQLite Backend Fixes

## 1. Problem Definition

The SQLVEC project (`.agents/projects/SQLVEC_sqlite-vec-backend/`) started implementing an SQLite backend for Maproom but was left in an incomplete state with multiple compile-time and runtime issues.

### Current Build Failures (Verified)

Running `cargo check --features sqlite` produces 4 errors:

1. **E0432 - Unresolved import**: `crate::db::sqlite::schema` module exists as `schema.rs` but no `mod schema;` declaration in `sqlite/mod.rs`
2. **E0277 - Type mismatch**: `DateTime<Utc>` doesn't satisfy `rusqlite::ToSql` because `rusqlite` needs `features = ["chrono"]`
3. **E0277 - Type mismatch**: Same DateTime issue (appears twice in different functions)
4. **E0382 - Move semantics**: `relpath` variable consumed then reused in `find_chunk_by_symbol`

### Pre-requisite Fixes (Uncommitted)

The following fixes were made in the current session but are NOT yet committed:
- `crates/maproom/src/db/mod.rs`: Added `#[cfg(feature = "sqlite")]` gate on sqlite module
- `crates/maproom/src/db/factory.rs`: Feature-gated SQLite imports and usage
- `crates/maproom/src/db/postgres/mod.rs`: Refactored to use connection pool
- `crates/maproom/src/db/queries.rs`: Fixed SearchHit type import
- `packages/vscode-maproom/src/extension.ts`: Restored working version

**These must be committed before SQLFIX tickets begin.**

### Runtime Issues (Discovered During Review)

Beyond compile errors, the SQLite code has runtime issues that will surface during testing:

1. **FTS5 Query Syntax**: Lines 454-459 of `sqlite/mod.rs` generate queries like `"term1"* "term2"*` which is invalid FTS5 syntax. The `*` prefix operator doesn't work with quoted phrases.

2. **Schema Column Mismatch**: `ChunkRecord` has `ts_doc_text` field but `schema.rs` doesn't create this column in the chunks table.

3. **FTS5 External Content Table**: `schema.rs` creates FTS5 with `content='chunks'` but column names don't match (FTS expects `content`, `docstring`, `symbol_name` to exist in chunks table).

4. **find_chunk_by_symbol Logic Bugs**: Beyond the move semantics error, the function has logic issues where SQL queries have different parameter counts but share the same `sql` variable, leading to parameter binding mismatches.

### Integration Issues

1. **VSCode extension**: Was corrupted during SQLite integration attempt, now restored but missing SQLite mode
2. **Feature gating**: SQLite module now gated but not fully tested
3. **Factory pattern**: `db/factory.rs` conditionally compiles but fallback behavior needs verification

### Missing Functionality

1. **Vector search**: SQLite backend has no vector similarity search implementation (deferred)
2. **Embedding dimension support**: Only 1536-dim hardcoded in schema (deferred)
3. **No integration tests**: Can't verify parity with Postgres backend

## 2. Existing State Analysis

### What Works
- `VectorStore` trait is well-defined in `db/mod.rs`
- `PostgresStore` implementation is complete and working (uses connection pool)
- `sqlite-vec` C extension is vendored and builds successfully
- Basic SQLite connection pooling with `r2d2_sqlite` is set up
- Schema creation tables are defined (with gaps noted above)

### What Doesn't Work
```
❌ cargo check --features sqlite  # Fails with 4 errors
❌ Runtime CRUD operations        # Will fail due to schema mismatches
❌ FTS search                      # Will fail due to query syntax
❌ Integration tests               # None exist
❌ CI testing                       # Only tests postgres feature
```

### Code Quality Issues
- Long commented explanations in SQLite code without actual implementation
- Inconsistent error handling (some use `?`, some use `.context()`)
- Dead code paths from original SQLVEC planning comments

## 3. Root Cause

The original SQLVEC implementation attempted too much at once without incremental validation:
1. Changed VSCode extension before Rust code compiled
2. Skipped unit tests for SQLite module
3. No feature-flag testing in CI
4. Didn't verify FTS5 query syntax against SQLite documentation

## 4. Requirements for MVP

### Must Have
1. `cargo check --features sqlite` compiles cleanly
2. `SqliteStore` passes basic CRUD operations
3. FTS search works with correct query syntax
4. CI tests both `postgres` and `sqlite` features
5. CLI can connect to SQLite when `MAPROOM_DATABASE_URL=sqlite://path.db`

### Should Have
1. Unit tests for SQLite CRUD operations
2. FTS search returns results comparable to Postgres

### Out of Scope (Deferred)
1. Vector similarity search via sqlite-vec
2. VSCode extension SQLite mode
3. Benchmarks comparing SQLite vs Postgres
4. 768-dim embedding support
5. Migration tooling between backends

## 5. Technical Constraints

### Dependency Constraints
- `rusqlite` needs `features = ["bundled", "chrono"]` for DateTime support
- `r2d2_sqlite` version 0.22 matches `rusqlite` 0.29
- `sqlite-vec` is C code requiring `cc` build step (already configured)

### Runtime Constraints
- SQLite single-writer lock limits concurrent indexing
- WAL mode helps but doesn't eliminate bottleneck
- `busy_timeout` PRAGMA needed to avoid SQLITE_BUSY errors

### Architecture Constraints
- Must maintain `VectorStore` trait interface
- Feature flags: `postgres` (default) vs `sqlite`
- Both features cannot be enabled simultaneously at runtime

## 6. Success Criteria

```bash
# All must pass
cargo check --features sqlite
cargo test --features sqlite
cargo check  # No regression in default postgres feature

# CI must test both features
.github/workflows/test.yml includes feature matrix
```

## 7. Risks & Mitigations

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| FTS5 syntax issues | High | Medium | Reference SQLite docs, test queries |
| Schema mismatches | High | High | Align schema.rs with ChunkRecord |
| CI regressions | Medium | High | Add feature matrix testing early |
| Performance issues | Low | Low | Defer to separate project |

## 8. Rollback Strategy

If SQLite feature causes issues:
1. Feature flag provides natural rollback (compile with `--features postgres` only)
2. No changes to Postgres code path
3. Can disable feature in CI without code changes

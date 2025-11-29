# Analysis: Rust Build Cleanup

## Problem Statement

The `crewchief-maproom` Rust crate produces ~58 compiler warnings and has 1 failing test. While the code compiles and runs, warnings indicate code quality issues that could mask future problems and reduce developer productivity.

## Current State

### Warning Categories (~58 Rust warnings)

| Category | Count | Examples |
|----------|-------|----------|
| Unused imports | 17 | `Context as AnyhowContext`, `tokio::sync::Mutex`, `tracing::debug` |
| Unused variables | 32 | `chunk_id`, `store`, `max_depth`, `worktree_id`, `options` |
| Dead code (functions) | 5 | `compute_edges`, `find_test_targets`, `insert_edges`, `is_route_chunk`, `is_test_chunk` |
| Dead code (methods) | 3 | `as_str`, `create_context_item`, `evict_lru_if_needed` |
| Unused structs/fields | 4 | `Edge` struct, multiple unused fields in structs |
| Unexpected cfg | 3 | `disabled_postgresql_test` in `src/indexer/mod.rs` |

**Note:** ~15 unused import warnings can be automatically fixed with `cargo fix --lib`.

### Test Failure (1 test)

- **Test**: `config::hot_reload::tests::test_invalid_config_rejected`
- **Issue**: Test expects negative weight validation to fail, but currently passes
- **Location**: `crates/maproom/src/config/hot_reload.rs:410`

### C Warnings (vendor code - out of scope)

The `sqlite-vec` C extension produces pragma and signedness warnings. These are in vendored third-party code and should not be modified.

## Affected Files

Based on compiler output, the following files contain warnings:

**A/B Testing:**
- `src/ab_testing/logger.rs`

**Context Module:**
- `src/context/cache.rs`
- `src/context/detectors/hooks.rs`
- `src/context/detectors/jsx.rs`
- `src/context/graph.rs`
- `src/context/relationships.rs`
- `src/context/strategies/python.rs`
- `src/context/strategies/react.rs`
- `src/context/strategies/rust.rs`
- `src/context/strategies/typescript.rs`

**Search Module:**
- `src/search/fts.rs`
- `src/search/graph.rs`
- `src/search/signals.rs`
- `src/search/vector.rs`

**Incremental Module:**
- `src/incremental/detector.rs`
- `src/incremental/edge_updater.rs`
- `src/incremental/processor.rs`
- `src/incremental/tree_sha_update.rs`

**Database Module:**
- `src/db/mod.rs`
- `src/db/sqlite/mod.rs`

**Config Module:**
- `src/config/hot_reload.rs` (test failure)

## Root Cause Analysis

1. **Refactoring artifacts**: Much of the dead code appears to be from the PostgreSQL → SQLite migration (SQLIMPL project). Functions and imports were left behind.

2. **Incomplete implementations**: Several `_todo` or stub-like unused variables suggest in-progress features that were partially wired.

3. **Test assumption error**: The `test_invalid_config_rejected` test expects validation during reload, but the YAML parser may be accepting negative numbers without type checking.

## Success Criteria

- `cargo build --bin crewchief-maproom` produces 0 Rust warnings (excluding vendor C warnings)
- `cargo test -p crewchief-maproom` passes 100% (906/906 tests)
- `cargo clippy -p crewchief-maproom` reports no actionable warnings
- No functional regressions

# Ticket: SQLIMPL-1003: Migrate Test Files Batch 2 (Search)

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Migrate search-related test files from PostgreSQL to SQLite. These tests validate FTS, vector search, and fusion ranking functionality.

## Background
Search tests verify the core search functionality of maproom. After migration, these tests will validate the Phase 2 search executor implementations.

This ticket implements Plan Phase 1, Ticket 1003: "Migrate Test Files Batch 2 (Search)".

## Acceptance Criteria
- [ ] `search_pipeline_integration_test.rs` compiles with SQLite
- [ ] `search_executors_test.rs` compiles with SQLite
- [ ] `fusion_integration_test.rs` compiles with SQLite
- [ ] `fusion_quality_test.rs` compiles with SQLite
- [ ] `rrf_fusion_test.rs` compiles with SQLite
- [ ] `weighted_fusion_test.rs` compiles with SQLite
- [ ] All 6 files pass `cargo test -p crewchief-maproom --no-run`

## Technical Requirements
- Use test helpers from `tests/common/mod.rs`
- Replace PostgreSQL connection setup with SQLite
- Tests may need to be marked `#[ignore]` until Phase 2 completes
- Ensure test data setup uses SQLite-compatible types

## Implementation Notes

### Files to Migrate (6 total)
1. `tests/search_pipeline_integration_test.rs` - High complexity
2. `tests/search_executors_test.rs` - High complexity
3. `tests/fusion_integration_test.rs` - Medium complexity
4. `tests/fusion_quality_test.rs` - Medium complexity
5. `tests/rrf_fusion_test.rs` - Low complexity
6. `tests/weighted_fusion_test.rs` - Low complexity

### Expected State After Migration
- Tests should **compile** but may not **pass** until Phase 2 is complete
- Search executor stubs currently return empty results
- Mark tests as `#[ignore]` with comment: `// Requires Phase 2: search executor wiring`

### Test Data Requirements
These tests need:
- Sample chunks with content for FTS matching
- Sample embeddings for vector search
- Expected ranking/scoring behavior

## Dependencies
- SQLIMPL-1001 (Migrate Test Common Module)

## Risk Assessment
- **Risk**: Tests will fail until Phase 2 executors are wired
  - **Mitigation**: Mark as `#[ignore]` during migration, unmask in Phase 2
- **Risk**: FTS5 query syntax may differ from expectations
  - **Mitigation**: SqliteStore already handles FTS5; tests validate that layer

## Files/Packages Affected
- `crates/maproom/tests/search_pipeline_integration_test.rs`
- `crates/maproom/tests/search_executors_test.rs`
- `crates/maproom/tests/fusion_integration_test.rs`
- `crates/maproom/tests/fusion_quality_test.rs`
- `crates/maproom/tests/rrf_fusion_test.rs`
- `crates/maproom/tests/weighted_fusion_test.rs`

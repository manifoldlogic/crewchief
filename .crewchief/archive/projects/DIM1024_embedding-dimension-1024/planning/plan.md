# Plan: embedding dimension 1024

## Overview

Add 1024-dimensional embedding support to enable mxbai-embed-large model. This is a focused fix to address nomic-embed-text tokenization crashes by supporting a more robust model. Implementation follows the established pattern from 768-dim support (Migration #7).

## Phases

### Phase 1: Database Foundation

**Objective**: Add database support for 1024-dimensional embeddings.

**Deliverables:**
- Migration #10: Create `vec_code_1024` virtual table
- Update dimension constants in embeddings.rs (SUPPORTED_DIMENSIONS)
- Update dimension constants in vector.rs (SUPPORTED_DIMENSIONS)
- Update dimension constants in columns.rs (for PostgreSQL compatibility)
- Add dimension mapping cases (1024 → vec_code_1024)
- Unit tests for 1024-dim storage and sync

**Agent Assignments:**
- rust-developer: Implement migration and dimension mappings
- unit-test-runner: Execute Rust unit tests
- verify-ticket: Verify migration idempotency and test coverage
- commit-ticket: Create atomic commit for database changes

**Success Criteria:**
- Migration #10 runs successfully (idempotent)
- 1024-dim embeddings can be stored in code_embeddings table
- 1024-dim embeddings sync to vec_code_1024 table
- Existing 768 and 1536-dim embeddings unaffected
- All unit tests pass (including new 1024-dim tests)

**Files Modified:**
- `/workspace/crates/maproom/src/db/sqlite/migrations.rs` (add Migration #10)
- `/workspace/crates/maproom/src/db/sqlite/embeddings.rs` (update constants and mapping)
- `/workspace/crates/maproom/src/db/sqlite/vector.rs` (update constants and mapping)
- `/workspace/crates/maproom/src/db/columns.rs` (update constants and mapping)

### Phase 2: Provider Configuration

**Objective**: Enable mxbai-embed-large model configuration and make Ollama provider dimension-aware.

**Deliverables:**
- Remove hardcoded dimension=768 from OllamaProvider
- Add dimension parameter to OllamaProvider::new()
- Read dimension from configuration instead of hardcoded value
- Update config validation to allow 1024 for Ollama
- Add helpful warnings for dimension/model mismatches
- Unit tests for configurable dimension

**Agent Assignments:**
- rust-developer: Implement dimension configurability in OllamaProvider
- unit-test-runner: Execute Rust unit tests
- verify-ticket: Verify backward compatibility with existing configs
- commit-ticket: Create atomic commit for provider changes

**Success Criteria:**
- OllamaProvider accepts dimension parameter
- dimension() method returns configured value (not hardcoded 768)
- Config validation passes for Ollama + 1024-dim
- Warning logs for dimension mismatches (not errors)
- Existing 768-dim configurations still work
- All unit tests pass

**Files Modified:**
- `/workspace/crates/maproom/src/embedding/ollama.rs` (make dimension configurable)
- `/workspace/crates/maproom/src/embedding/config.rs` (update validation logic)

### Phase 3: Sanitization Cleanup

**Objective**: Remove character sanitization workaround for mxbai-embed-large while preserving it for nomic-embed-text.

**Deliverables:**
- Add conditional sanitization based on model name
- Extract sanitization logic into helper function
- Apply sanitization only for nomic-embed-text
- Skip sanitization for mxbai-embed-large and other models
- Unit tests for conditional sanitization
- Integration test with problematic characters (|, [], (), Unicode)

**Agent Assignments:**
- rust-developer: Implement conditional sanitization
- unit-test-runner: Execute Rust unit tests
- verify-ticket: Verify no content mangling for mxbai-embed-large
- commit-ticket: Create atomic commit for sanitization changes

**Success Criteria:**
- Sanitization applies only when model == "nomic-embed-text"
- mxbai-embed-large receives raw text (no character replacement)
- Backward compatibility: nomic-embed-text still uses sanitization
- Integration test confirms problematic characters handled correctly
- All unit tests pass

**Files Modified:**
- `/workspace/crates/maproom/src/embedding/ollama.rs` (conditional sanitization)

### Phase 4: Testing and Documentation

**Objective**: Comprehensive testing and user documentation for 1024-dim support.

**Deliverables:**
- End-to-end test: Generate 1024-dim embeddings with mxbai-embed-large
- End-to-end test: Vector search with 1024-dim embeddings
- End-to-end test: Mixed dimensions (768, 1024, 1536) coexist
- Integration test: Migration #10 idempotency
- Update user documentation (ollama-setup.md)
- Update developer documentation (CLAUDE.md)
- Configuration examples for mxbai-embed-large

**Agent Assignments:**
- rust-developer: Write integration tests
- unit-test-runner: Execute all tests
- documentation-writer: Update docs/providers/ollama-setup.md
- documentation-writer: Update crates/maproom/CLAUDE.md
- verify-ticket: Verify test coverage and documentation completeness
- commit-ticket: Create atomic commit for tests and docs

**Success Criteria:**
- E2E test: mxbai-embed-large generates 1024-dim embeddings
- E2E test: Search returns relevant results with 1024-dim
- E2E test: 768, 1024, 1536 dimensions coexist without errors
- Migration test: Running Migration #10 twice is safe
- Documentation includes configuration examples
- Documentation explains storage/performance tradeoffs
- All tests pass (unit + integration + E2E)

**Files Modified:**
- `/workspace/crates/maproom/tests/sqlite_integration.rs` (add integration tests)
- `/workspace/docs/providers/ollama-setup.md` (add mxbai-embed-large config)
- `/workspace/crates/maproom/CLAUDE.md` (update supported dimensions)

## Dependencies

### Cross-Phase Dependencies

- **Phase 2 depends on Phase 1**: Cannot configure dimension until database supports it
- **Phase 3 depends on Phase 2**: Conditional sanitization requires dimension-aware provider
- **Phase 4 depends on Phases 1-3**: Cannot test until all components implemented

### External Dependencies

- **Ollama installation**: mxbai-embed-large model must be pulled (`ollama pull mxbai-embed-large`)
- **sqlite-vec extension**: Must be statically linked in build (already satisfied)
- **Test environment**: Requires Ollama running locally for integration tests (can skip with `#[ignore]`)

## Risk Mitigation

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Migration #10 breaks existing databases | Low | High | Extensive testing, idempotency checks, rollback plan |
| Dimension mismatch errors confuse users | Medium | Medium | Clear error messages listing all supported dimensions |
| Mixed dimensions cause search quality issues | Low | Medium | Document re-embedding requirement, test mixed scenarios |
| Sanitization removal breaks nomic-embed-text | Low | High | Keep conditional sanitization for nomic-embed-text |
| mxbai-embed-large not available | Medium | Medium | Graceful error, document model installation |
| Storage growth concerns | Low | Low | Document 33% increase, storage is cheap |
| Performance regression with 1024-dim | Low | Medium | Benchmark search latency, sqlite-vec is optimized |

## Success Metrics

### Phase 1 Metrics
- [ ] Migration #10 in migrations list
- [ ] `SUPPORTED_DIMENSIONS` contains [768, 1024, 1536]
- [ ] `get_vec_table_name(1024)` returns "vec_code_1024"
- [ ] Unit test: 1024-dim embedding stored successfully
- [ ] Unit test: 1024-dim embedding syncs to vec_code_1024
- [ ] Existing 768/1536 tests still pass

### Phase 2 Metrics
- [ ] OllamaProvider has dimension field
- [ ] dimension() returns configured value
- [ ] Config validation allows 1024 for Ollama
- [ ] Warning logs for mismatches (not errors)
- [ ] Unit test: OllamaProvider accepts dimension=1024
- [ ] Backward compat: dimension=768 still works

### Phase 3 Metrics
- [ ] Sanitization function extracted
- [ ] Model check before sanitization
- [ ] nomic-embed-text uses sanitization
- [ ] mxbai-embed-large skips sanitization
- [ ] Unit test: conditional sanitization logic
- [ ] Integration test: problematic characters handled

### Phase 4 Metrics
- [ ] E2E test: 1024-dim embeddings generated
- [ ] E2E test: Search with 1024-dim returns results
- [ ] E2E test: Mixed dimensions coexist
- [ ] Migration test: Idempotency verified
- [ ] Documentation: mxbai-embed-large config example
- [ ] Documentation: Storage/performance tradeoffs
- [ ] All tests pass (100% pass rate)

## Timeline Estimate

| Phase | Complexity | Estimated Effort |
|-------|------------|------------------|
| Phase 1: Database Foundation | Low (follows pattern) | 2-3 hours |
| Phase 2: Provider Configuration | Medium (refactoring) | 3-4 hours |
| Phase 3: Sanitization Cleanup | Low (conditional logic) | 1-2 hours |
| Phase 4: Testing and Documentation | Medium (comprehensive testing) | 3-4 hours |
| **Total** | | **9-13 hours** |

**Note**: Estimates assume familiarity with Rust and the codebase. Actual time may vary.

## Rollback Strategy

If critical issues discovered after deployment:

### Immediate Rollback
1. Revert environment variables:
   ```bash
   MAPROOM_EMBEDDING_MODEL=nomic-embed-text
   MAPROOM_EMBEDDING_DIMENSION=768
   ```
2. Existing 768-dim embeddings in vec_code_768 continue working
3. No code changes needed (backward compatible by design)

### Database Rollback (if Migration #10 problematic)
1. Stop daemon
2. Connect to SQLite: `sqlite3 ~/.maproom/maproom.db`
3. Drop table: `DROP TABLE IF EXISTS vec_code_1024;`
4. Delete migration record: `DELETE FROM schema_migrations WHERE version = 10;`
5. Restart daemon (skips Migration #10)

### Code Rollback (if changes break existing functionality)
1. Revert commits (git reset or revert)
2. Rebuild: `cargo build --release`
3. Restart daemon
4. Existing 768/1536 embeddings unaffected (no data changes)

## Post-Deployment Validation

### Verify Installation
1. Check migration applied: `SELECT * FROM schema_migrations WHERE version = 10;`
2. Check table exists: `SELECT name FROM sqlite_master WHERE type='table' AND name='vec_code_1024';`
3. Test configuration: `MAPROOM_EMBEDDING_DIMENSION=1024` accepted

### Functional Testing
1. Generate 1024-dim embedding: `crewchief-maproom generate-embeddings --repo test`
2. Verify storage: `SELECT COUNT(*) FROM code_embeddings WHERE embedding_dim = 1024;`
3. Verify sync: `SELECT COUNT(*) FROM vec_code_1024;`
4. Test search: `crewchief-maproom search --query "test" --repo test`
5. Verify results: Search returns chunks from 1024-dim embeddings

### Performance Baseline
1. Measure search latency (10 queries avg): `time crewchief-maproom search --query "..." --repo test`
2. Compare 768 vs 1024 vs 1536 latency (should be comparable)
3. Measure storage: `du -sh ~/.maproom/maproom.db`
4. Track embedding generation throughput: tokens/sec in logs

## Handoff to Ticket Creator

This plan is ready for ticket generation. Each phase maps to 1-2 tickets:
- **DIM1024-1001**: Phase 1 (Database Foundation)
- **DIM1024-2001**: Phase 2 (Provider Configuration)
- **DIM1024-2002**: Phase 3 (Sanitization Cleanup)
- **DIM1024-3001**: Phase 4 (Testing and Documentation)

Each ticket should include:
- Acceptance criteria from phase success metrics
- File list from "Files Modified"
- Agent assignments
- Test requirements

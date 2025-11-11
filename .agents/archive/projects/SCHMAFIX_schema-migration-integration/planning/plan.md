# Execution Plan: Schema Migration Integration

## Project Goal

Integrate existing BLOBSHA/BRANCHX migration SQL files into the Rust migration runner so the database schema matches what the code expects.

**Success**: MCP vector search works, schema is complete, Rust owns migrations.

## Scope Boundaries

### In Scope
- Copy migration SQL files from `packages/maproom-mcp/migrations/` to `crates/maproom/migrations/`
- Update Rust migration runner to include new migrations (0018-0021)
- Test migrations on fresh and existing databases
- Verify MCP tools work with new schema
- Update documentation

### Out of Scope
- Implementing BLOBSHA embedding deduplication logic
- Implementing BRANCHX incremental update algorithms
- Implementing BRWATCH file watching
- Optimizing migration performance
- Creating rollback migrations

## High-Level Phases

### Phase 1: Migration File Preparation
**Goal**: Create Rust migration files from MCP migrations

**Tasks**:
1. Copy MCP migration 001 to `crates/maproom/migrations/0018_add_blob_sha.sql`
2. Copy MCP migration 002 to `crates/maproom/migrations/0019_create_code_embeddings.sql`
3. Copy MCP migration 004 to `crates/maproom/migrations/0020_add_worktree_tracking.sql`
4. Copy MCP migration 005 to `crates/maproom/migrations/0021_complete_branchx_schema.sql`
5. Adapt headers to reference SCHMAFIX tickets
6. Ensure all SQL uses `IF NOT EXISTS` for idempotency

**Agent**: rust-indexer-engineer
**Deliverables**: 4 migration SQL files in `crates/maproom/migrations/`

### Phase 2: Rust Migration Runner Update
**Goal**: Integrate new migrations into Rust binary

**Tasks**:
1. Update `crates/maproom/src/db/queries.rs`
2. Add 4 new migrations to the `migrations` array
3. Ensure all use `concurrent = false` (run in transactions)
4. Verify compile succeeds

**Agent**: rust-indexer-engineer
**Deliverables**: Updated `queries.rs` with migrations 0018-0021

### Phase 3: Migration Testing
**Goal**: Verify migrations work correctly

**Tasks**:
1. Write Rust integration test for fresh database (0000-0021)
2. Write Rust integration test for incremental migration (0017 → 0021)
3. Write Rust integration test for idempotency (run twice)
4. Write schema validation test (blob_sha, code_embeddings exist)
5. Run all tests, verify passing

**Agent**: rust-indexer-engineer (write tests), unit-test-runner (execute)
**Deliverables**: Integration tests in `crates/maproom/tests/migration_integration.rs`

### Phase 4: MCP Integration Verification
**Goal**: Confirm MCP tools work with new schema

**Tasks**:
1. Write TypeScript integration test for code_embeddings table existence
2. Write TypeScript integration test for vector search (shouldn't crash)
3. Update MCP test suite to use new schema
4. Run MCP tests, verify passing

**Agent**: integration-tester
**Deliverables**: MCP integration tests in `packages/maproom-mcp/tests/migrations/schema-integration.test.ts`

### Phase 5: Manual Validation
**Goal**: Human verification of migration correctness

**Tasks**:
1. Run migrations on test database
2. Manually verify schema using `psql` commands
3. Test MCP server startup
4. Test vector search query (mode: vector)
5. Complete manual checklist from quality-strategy.md

**Agent**: verify-ticket
**Deliverables**: Completed manual validation checklist

### Phase 6: Documentation
**Goal**: Document migration integration and ownership

**Tasks**:
1. Update `packages/maproom-mcp/migrations/README.md` to note Rust ownership
2. Update `crates/maproom/CLAUDE.md` to mention new migrations
3. Update `docs/architecture/DATABASE_ARCHITECTURE.md` with new schema
4. Add migration comments to each SQL file

**Agent**: general-purpose
**Deliverables**: Updated documentation

## Ticket Breakdown

### SCHMAFIX-1001: Copy and Adapt Migration SQL Files
**Description**: Copy MCP migrations 001, 002, 004, 005 to Rust as 0018-0021
**Agent**: rust-indexer-engineer
**Acceptance Criteria**:
- 4 SQL files exist in `crates/maproom/migrations/`
- All headers reference SCHMAFIX project
- All SQL uses `IF NOT EXISTS`
- Files compile with `include_str!`

### SCHMAFIX-1002: Update Rust Migration Runner
**Description**: Add migrations 0018-0021 to `crates/maproom/src/db/queries.rs`
**Agent**: rust-indexer-engineer
**Acceptance Criteria**:
- `migrations` array includes 0018-0021
- All use `concurrent = false`
- Rust compiles without errors
- `cargo clippy` passes

### SCHMAFIX-1003: Write Migration Integration Tests
**Description**: Create Rust tests for fresh, incremental, and idempotent migrations
**Agent**: rust-indexer-engineer
**Acceptance Criteria**:
- Fresh database test applies all 22 migrations
- Incremental test applies only 0018-0021 to v0.17 database
- Idempotency test runs migrations twice without errors
- Schema validation test checks blob_sha, code_embeddings exist

### SCHMAFIX-1004: Run Migration Integration Tests
**Description**: Execute migration tests and report results
**Agent**: unit-test-runner
**Acceptance Criteria**:
- All migration tests pass
- No panics or errors
- Test output confirms schema correctness

### SCHMAFIX-1005: Write MCP Integration Tests
**Description**: Create TypeScript tests for MCP schema compatibility
**Agent**: integration-tester
**Acceptance Criteria**:
- Test confirms code_embeddings table exists
- Test confirms vector search doesn't crash
- Tests pass against migrated database

### SCHMAFIX-1006: Manual Migration Validation
**Description**: Manual testing of migrations on test database
**Agent**: verify-ticket
**Acceptance Criteria**:
- Migrations apply cleanly to fresh database
- Migrations apply cleanly to v0.17 database
- MCP server starts without errors
- Vector search executes successfully
- Manual checklist 100% complete

### SCHMAFIX-1007: Update Documentation
**Description**: Document migration ownership and new schema
**Agent**: general-purpose
**Acceptance Criteria**:
- MCP migrations README notes Rust ownership
- Rust CLAUDE.md mentions migrations 0018-0021
- DATABASE_ARCHITECTURE.md updated with new schema
- Migration SQL files have clear comments

## Dependencies and Sequencing

### Critical Path
```
SCHMAFIX-1001 (Copy SQL files)
    ↓
SCHMAFIX-1002 (Update Rust runner)
    ↓
SCHMAFIX-1003 (Write tests)
    ↓
SCHMAFIX-1004 (Run tests) ← Blocker if tests fail
    ↓
SCHMAFIX-1005 (MCP integration tests)
    ↓
SCHMAFIX-1006 (Manual validation)
    ↓
SCHMAFIX-1007 (Documentation)
```

### Parallel Opportunities
- SCHMAFIX-1003 and SCHMAFIX-1005 can be written in parallel (different codebases)
- SCHMAFIX-1007 can start during SCHMAFIX-1006 (documentation is independent)

### Blocking Dependencies
- **MUST pass SCHMAFIX-1004** before proceeding to SCHMAFIX-1005
- **MUST pass SCHMAFIX-1006** before marking project complete

## Risk Mitigation

### Risk: Migration 0018 backfill fails
**Mitigation**: Start with small test database, verify query works
**Contingency**: Simplify backfill or make column nullable

### Risk: Existing schema conflicts with migrations
**Mitigation**: Use `IF NOT EXISTS` extensively, test on v0.17 database
**Contingency**: Update migration SQL to handle existing schema

### Risk: MCP still crashes after migrations
**Mitigation**: Integration tests catch this early
**Contingency**: Investigate root cause, may need code changes

### Risk: Tests fail in CI/CD
**Mitigation**: Run tests locally first, ensure database is available
**Contingency**: Fix test setup, ensure PostgreSQL is running

## Success Metrics

**Quantitative**:
- ✅ All 7 tickets completed
- ✅ 100% test pass rate (Rust + MCP)
- ✅ 0 migration failures in testing
- ✅ Manual checklist 100% complete

**Qualitative**:
- ✅ MCP vector search doesn't crash
- ✅ Schema matches architecture documents
- ✅ Migrations are idempotent
- ✅ Documentation is updated

**User-Facing**:
- ✅ New users can run migrations successfully
- ✅ Existing users can upgrade without data loss
- ✅ Vector search is usable (even if results are empty)

## Timeline Estimates

**Optimistic** (all tests pass first try): 4-6 hours
- Phase 1-2: 1 hour (copy files, update runner)
- Phase 3-4: 2 hours (write and run tests)
- Phase 5: 1 hour (manual validation)
- Phase 6: 1 hour (documentation)

**Realistic** (some test failures): 8-12 hours
- Debugging migration issues: +2-4 hours
- Test fixes and iteration: +2-4 hours

**Pessimistic** (major schema issues): 16-20 hours
- Schema conflicts require SQL rewrites: +4-6 hours
- MCP code changes needed: +4-6 hours

**Expected**: Realistic timeline (8-12 hours)

## Completion Criteria

**Project Complete When**:
1. All 7 tickets have checkmarks in `Task completed` section
2. All Rust migration tests pass
3. All MCP integration tests pass
4. Manual validation checklist 100% complete
5. Documentation updated
6. Commit created with conventional commit message

**Evidence of Success**:
- `SELECT * FROM schema_migrations` shows version 21
- `\d maproom.chunks` shows `blob_sha` column
- `\dt maproom.code_embeddings` shows table exists
- `mcp__maproom__search({ mode: 'vector', query: 'test' })` doesn't crash
- No "table does not exist" errors in MCP logs

## Post-Completion

**Immediate Next Steps** (NOT in this project):
- BLOBSHA-IMPL: Implement blob SHA computation during indexing
- BRANCHX-IMPL: Implement incremental update algorithms
- BRWATCH-IMPL: Implement file watching

**Knowledge Transfer**:
- Archive SCHMAFIX project when complete
- Synthesize learnings to `docs/architecture/migrations.md`
- Update contributor guidelines with migration process

## Agent Assignments

| Phase | Primary Agent | Support Agent |
|-------|---------------|---------------|
| 1. Migration Files | rust-indexer-engineer | - |
| 2. Rust Runner | rust-indexer-engineer | - |
| 3. Migration Tests | rust-indexer-engineer | unit-test-runner |
| 4. MCP Integration | integration-tester | unit-test-runner |
| 5. Manual Validation | verify-ticket | - |
| 6. Documentation | general-purpose | - |

## Communication Plan

**Progress Updates**: After each ticket completion
**Blockers**: Immediately report test failures
**Decisions**: Document in ticket comments
**Changes**: Update plan.md if scope changes

## Rollback Plan

**If Project Must Be Abandoned**:
1. Don't merge any code changes
2. Keep MCP migrations as documentation
3. Note in PROJECT_SUMMARY.md that integration was attempted
4. Create issue for future work

**If Partial Completion**:
- Completed tickets can be merged independently
- Incomplete tickets documented as future work
- No harm from additive migrations (safe to keep)

## Definition of Done

**Ticket Level**:
- Code written and reviewed
- Tests passing
- Documentation updated
- Acceptance criteria met

**Project Level**:
- All 7 tickets complete
- Integration tests passing
- Manual validation complete
- Knowledge documented
- Commit created

**Quality Gates**:
- Zero test failures
- Zero migration errors
- MCP vector search functional
- Schema matches specification

This plan provides clear direction for systematic execution of the schema migration integration while maintaining focus on the MVP goal: make the schema match what the code expects.

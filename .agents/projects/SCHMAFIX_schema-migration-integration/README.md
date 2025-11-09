# SCHMAFIX - Schema Migration Integration

**Status**: 🚧 Planning Complete - Ready for Ticket Creation
**Created**: 2025-11-09
**Tickets**: 0/7 (estimated)

## Overview

Integrate existing BLOBSHA/BRANCHX migration SQL files into the Rust migration runner to fix the critical disconnect between database schema and code expectations.

**The Problem**:
- MCP TypeScript code references `code_embeddings` table that doesn't exist
- Migration SQL files were created but never integrated into Rust runtime
- Vector search crashes with "table does not exist" error

**The Solution**:
- Copy 4 MCP migrations to Rust as 0018-0021
- Update Rust migration runner to include new migrations
- Test migrations thoroughly (fresh, incremental, idempotent)
- Verify MCP tools work with new schema

## Project Scope

### In Scope ✅
- Schema integration only (not feature implementation)
- Copy migration SQL from MCP to Rust
- Update Rust migration runner
- Write comprehensive migration tests
- Verify MCP compatibility
- Update documentation

### Out of Scope ❌
- BLOBSHA embedding deduplication logic
- BRANCHX incremental update algorithms
- BRWATCH file watching implementation
- Migration performance optimization
- Rollback migrations

## Key Documents

### Planning Documents
- [analysis.md](planning/analysis.md) - Problem definition, root cause analysis
- [architecture.md](planning/architecture.md) - Technical solution design
- [quality-strategy.md](planning/quality-strategy.md) - Testing strategy and risk mitigation
- [security-review.md](planning/security-review.md) - Security assessment
- [plan.md](planning/plan.md) - Execution plan with phases and tickets

### Work Tickets
Tickets will be created in `tickets/` directory after plan approval.

**Estimated Tickets**:
1. SCHMAFIX-1001: Copy and adapt migration SQL files
2. SCHMAFIX-1002: Update Rust migration runner
3. SCHMAFIX-1003: Write migration integration tests
4. SCHMAFIX-1004: Run migration integration tests
5. SCHMAFIX-1005: Write MCP integration tests
6. SCHMAFIX-1006: Manual migration validation
7. SCHMAFIX-1007: Update documentation

## Technical Context

### Current Database State
- ✅ `worktree_ids` JSONB column exists (from manual SQL)
- ✅ `worktree_index_state` table exists
- ❌ `blob_sha` column missing from chunks
- ❌ `code_embeddings` table missing entirely

### Migration Files to Integrate
From `packages/maproom-mcp/migrations/`:
- `001_add_blob_sha.sql` → `crates/maproom/migrations/0018_add_blob_sha.sql`
- `002_create_code_embeddings.sql` → `0019_create_code_embeddings.sql`
- `004_add_worktree_tracking.sql` → `0020_add_worktree_tracking.sql`
- `005_complete_branchx_schema.sql` → `0021_complete_branchx_schema.sql`

### Architecture Decision
**Single Source of Truth**: Rust migration runner (`crates/maproom/src/db/queries.rs`)

**Why**:
- Rust binary is standalone (works without Node.js)
- Compile-time validation of migration SQL
- Single binary deployment
- Existing migration framework

## Success Criteria

**Quantitative**:
- ✅ All 7 tickets completed
- ✅ 100% test pass rate
- ✅ 0 migration failures
- ✅ Manual validation checklist complete

**Qualitative**:
- ✅ MCP vector search doesn't crash
- ✅ Schema matches specifications
- ✅ Migrations are idempotent
- ✅ Documentation updated

**Evidence**:
- `SELECT * FROM schema_migrations` shows version 21
- `\d maproom.chunks` shows `blob_sha` column
- `\dt maproom.code_embeddings` shows table exists
- MCP vector search executes without error

## Timeline

**Optimistic**: 4-6 hours (all tests pass first try)
**Realistic**: 8-12 hours (some iteration needed)
**Pessimistic**: 16-20 hours (major schema conflicts)

**Expected**: Realistic (8-12 hours)

## Related Projects

### Archived Projects (Provided Schema)
- **BLOBSHA_content-addressed-chunk-storage** - Created migrations 001, 002
- **BRANCHX_branch-aware-indexing** - Created migrations 004, 005
- **BRWATCH_branch-switch-detection** - Related to incremental updates

### Future Projects (Will Use This Schema)
- **BLOBSHA-IMPL** - Implement blob SHA computation in Rust
- **BRANCHX-IMPL** - Implement incremental update algorithms
- **BRWATCH-IMPL** - Implement file watching

## Dependencies

**Required Infrastructure**:
- PostgreSQL 14+ with pgvector extension
- Rust toolchain (for compilation)
- Node.js + pnpm (for MCP testing)

**Database Access**:
- `DATABASE_URL` environment variable
- Test database for integration tests
- Docker Compose setup (already configured)

## Risk Summary

**High Risk** (Must Address):
- Migration 0018 backfill failure → Use transactions, batching
- Schema drift between environments → Use IF NOT EXISTS
- Data loss during migration → Test thoroughly first

**Medium Risk** (Monitor):
- Database connection exposure → Use env vars (already standard)
- Migration ordering race → Advisory locks (already implemented)
- Breaking changes to data → Additive only (enforced)

**Low Risk** (Accept):
- Brief service interruption during migration (15-70 seconds)
- Unauthorized schema inspection (database-level security)

## Getting Started

### For Project Execution

1. **Create Tickets**: Run `/create-project-tickets SCHMAFIX`
2. **Review Tickets**: Run `/review-tickets SCHMAFIX`
3. **Execute Project**: Run `/work-on-project SCHMAFIX`

### For Manual Exploration

```bash
# Read planning documents
cat planning/analysis.md
cat planning/architecture.md
cat planning/plan.md

# Check current migration state
psql $DATABASE_URL -c "SELECT * FROM maproom.schema_migrations ORDER BY version"

# Check schema
psql $DATABASE_URL -c "\d maproom.chunks"
psql $DATABASE_URL -c "\dt maproom.code_embeddings"
```

## Project Constraints

1. **No breaking changes** - Additive only
2. **Backward compatibility** - Old binaries tolerate new schema
3. **Zero downtime** - Migrations run quickly
4. **Data preservation** - No data loss
5. **Test coverage** - Comprehensive migration tests

## Knowledge Transfer

**After Completion**:
- Archive project to `.agents/archive/projects/SCHMAFIX_schema-migration-integration/`
- Synthesize learnings to `docs/architecture/migrations.md`
- Update contributor guidelines with migration process
- Document schema ownership in README files

## Communication

**Questions**: Ask in planning phase before ticket creation
**Blockers**: Report immediately during execution
**Changes**: Update plan.md and notify stakeholders
**Success**: Commit with conventional commit message

---

**Next Step**: Run `/create-project-tickets SCHMAFIX` to generate individual work tickets from this plan.

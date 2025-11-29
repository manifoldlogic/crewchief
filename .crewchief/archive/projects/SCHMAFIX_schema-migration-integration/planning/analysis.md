# Analysis: Schema Migration Integration

## Problem Definition

Three completed projects (BLOBSHA, BRANCHX, BRWATCH) created database migrations in `/workspace/packages/maproom-mcp/migrations/` but these migrations were **never integrated** into the Rust migration runtime in `/workspace/crates/maproom/src/db/queries.rs`.

This creates a critical disconnect:
- **MCP TypeScript code** references tables that don't exist (`code_embeddings`)
- **Database** is missing columns and tables required by the architecture
- **Rust indexer** doesn't implement logic for schema features that were planned

### Current State

**Database Schema** (as of 2025-11-09):
- ✅ `worktree_ids` JSONB column exists in chunks (from init.sql manual application)
- ✅ `worktree_index_state` table exists
- ❌ `blob_sha` column missing from chunks table
- ❌ `code_embeddings` table missing entirely

**Migration Files Exist But Not Applied**:
```
packages/maproom-mcp/migrations/
├── 001_add_blob_sha.sql (BLOBSHA-1002)
├── 002_create_code_embeddings.sql (BLOBSHA-2001)
├── 004_add_worktree_tracking.sql (BRANCHX-1002)
└── 005_complete_branchx_schema.sql (BRANCHX-1904)
```

**Rust Migration Runner** (crates/maproom/src/db/queries.rs):
- Only includes migrations 0000-0016
- No references to 001, 002, 004, 005 from maproom-mcp
- Uses `include_str!` to embed migration SQL at compile time

**MCP Server Code** (packages/maproom-mcp/src/index.ts:511):
```typescript
// BREAKS: table "code_embeddings" does not exist
SELECT COUNT(*) as count FROM maproom.code_embeddings LIMIT 1
```

### Root Cause Analysis

1. **Dual Migration Systems**:
   - MCP migrations in `packages/maproom-mcp/migrations/`
   - Rust migrations in `crates/maproom/migrations/`
   - No synchronization mechanism

2. **Manual init.sql Application**:
   - Docker Compose comment shows init.sql is NOT auto-mounted
   - Some schema applied manually (worktree_ids), some not (blob_sha)

3. **No Migration Orchestration**:
   - Projects created migration files but didn't update Rust runner
   - No verification that migrations were actually applied
   - Tests passed in isolation but not end-to-end

## Industry Solutions

### Approach 1: Unified Migration Directory
**Example**: Rails, Django, Flyway
- Single source of truth for all migrations
- Numbered sequentially
- Applied in order automatically

**Pros**: Simple, clear ownership
**Cons**: Requires coordinating multiple teams

### Approach 2: Service-Owned Migrations
**Example**: Microservices with schema-per-service
- Each service manages its own schema
- Services apply migrations on startup
- Cross-service coordination via API contracts

**Pros**: Service autonomy, clear boundaries
**Cons**: Schema ownership complexity

### Approach 3: Schema Registry
**Example**: Confluent Schema Registry, Protobuf
- Central registry tracks schema versions
- Services register schema changes
- Compatibility checks prevent breakage

**Pros**: Prevents breaking changes
**Cons**: Additional infrastructure

## Current Project Architecture

CrewChief has **hybrid architecture**:
- **Rust binary** (`crates/maproom`) - Core indexing engine, embeds migrations
- **MCP TypeScript server** (`packages/maproom-mcp`) - Tool interface, wraps Rust binary
- **Single database** - Shared schema between both

**Migration ownership should be**: Rust binary (it's the core engine)
**Migration files should be**: In `crates/maproom/migrations/`
**MCP should**: Use whatever schema Rust creates

## Research Findings

### Why init.sql Wasn't Applied

Docker Compose line 14:
```yaml
# Note: init.sql mount disabled in dev container due to Docker-in-Docker limitations
# Schema will be initialized via migrations or manual SQL execution
```

This means:
- init.sql is reference documentation only
- Actual schema created by Rust migration runner
- Manual SQL was used for worktree_ids (temporary workaround)

### Why Migrations Weren't Integrated

Reviewing ticket history:
- BLOBSHA/BRANCHX tickets created migration SQL files
- Tests validated SQL syntax and functionality
- BUT: No ticket to integrate into Rust migration runner
- Projects marked complete based on SQL files existing

### What Actually Works Today

1. **FTS Search** ✅ - Uses basic chunks table
2. **File Indexing** ✅ - No embedding deduplication
3. **Worktree Tracking** ⚠️  - Schema exists but Rust doesn't use it
4. **Vector Search** ❌ - Fails on code_embeddings check
5. **Hybrid Search** ❌ - Same failure
6. **Incremental Updates** ❌ - Rust doesn't check tree SHA

## Gap Analysis

### Missing Implementation

**BLOBSHA Features**:
- ❌ Blob SHA computation not called during indexing
- ❌ code_embeddings table doesn't exist
- ❌ Embedding deduplication not implemented
- ❌ Cache-aware upsert not implemented

**BRANCHX Features**:
- ⚠️  worktree_ids column exists but not populated
- ⚠️  worktree_index_state exists but not used
- ❌ Tree SHA comparison not implemented
- ❌ Incremental update algorithm not implemented
- ❌ git diff-tree integration missing

**BRWATCH Features**:
- ❌ File watching not implemented
- ❌ Branch switch detection missing
- ❌ Auto-trigger logic not implemented

### Impact on Users

**Current User Experience**:
```bash
# Works fine
maproom search "authentication" --mode fts

# Fails with "table code_embeddings does not exist"
maproom search "authentication" --mode vector

# Works but doesn't do incremental updates
maproom scan /path/to/repo
# (Always full scan, no tree SHA optimization)
```

## Technical Debt Assessment

**High Priority** (Breaks functionality):
- MCP references non-existent code_embeddings table
- Vector search completely broken

**Medium Priority** (Architecture incomplete):
- No embedding deduplication (wastes money)
- No incremental updates (wastes time)
- Rust code doesn't use worktree tracking

**Low Priority** (Future features):
- BRWATCH auto-triggering
- Advanced incremental logic

## Success Criteria

For this project to succeed:

1. **All migrations applied** - Database has all tables/columns
2. **MCP tools work** - Vector search doesn't crash
3. **Rust migration runner owns schema** - Single source of truth
4. **Tests validate schema** - E2E tests confirm database state
5. **Documentation updated** - Clear migration ownership

This is NOT implementing BLOBSHA/BRANCHX features - that's future work. This is fixing the **schema foundation** so those features CAN be implemented later.

## Constraints

1. **No breaking changes** - Existing data must migrate safely
2. **Backward compatibility** - Old Rust binaries should gracefully handle missing columns
3. **Zero downtime** - Migrations must be additive
4. **Data preservation** - No data loss during migration
5. **Test coverage** - Migrations must be testable

## Out of Scope

- Implementing BLOBSHA embedding deduplication logic
- Implementing BRANCHX incremental update algorithms
- Implementing BRWATCH file watching
- Optimizing migration performance
- Adding rollback migrations (can be future work)

This project focuses solely on: **Make the schema match what the code expects**.

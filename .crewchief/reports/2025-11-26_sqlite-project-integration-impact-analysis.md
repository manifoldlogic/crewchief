# SQLITE Project Integration Impact Analysis

**Date**: 2025-11-26
**Project**: SQLITE_full-sqlite-implementation
**Status**: 57% Complete (8 of 14 tickets)
**Purpose**: Identify all codebase locations requiring updates after SQLITE project completion

---

## Executive Summary

The SQLITE project implements SQLite as an alternative database backend for Maproom's semantic code search. While the **storage layer** (`crates/maproom/src/db/sqlite/`) is well-architected with proper trait abstraction, the **consumption layer** (CLI commands, daemon, queries) bypasses this abstraction and directly uses PostgreSQL-specific code.

**Key Finding**: A follow-on integration project is needed to update ~50+ files across 6 components to fully support SQLite-only operation.

---

## What the SQLITE Project Covers

### Completed Work (Phases 0-3)

| Ticket | Description | Status |
|--------|-------------|--------|
| SQLITE-0001 | Migration system | ✅ Complete |
| SQLITE-0002 | sqlite-vec extension verification | ✅ Complete |
| SQLITE-1001 | Schema migrations (junction table, embeddings) | ✅ Complete |
| SQLITE-1002 | CRUD operations for junction table | ✅ Complete |
| SQLITE-2001 | Embedding module with deduplication | ✅ Complete |
| SQLITE-2002 | Vector table population sync | ✅ Complete |
| SQLITE-3001 | Vector search module | ✅ Complete |
| SQLITE-3901 | Vector search tests | ✅ Complete |

### Remaining Work (Phases 4-6)

| Ticket | Description | Status |
|--------|-------------|--------|
| SQLITE-4001 | FTS module extraction | In Progress |
| SQLITE-4002 | Hybrid search module | Pending |
| SQLITE-4003 | Semantic ranking | Pending |
| SQLITE-5001 | Graph module | Pending |
| SQLITE-5901 | Graph tests | Pending |
| SQLITE-6001 | Integration test suite | Pending |
| SQLITE-6002 | Final verification | Pending |

### Key Abstraction

A `VectorStore` trait exists at `crates/maproom/src/db/mod.rs` with implementations:
- `PostgresStore` at `db/postgres/mod.rs`
- `SqliteStore` at `db/sqlite/mod.rs`

Factory function at `db/factory.rs:7`:
```rust
pub async fn get_store() -> anyhow::Result<Arc<dyn VectorStore>>
```

**Problem**: This abstraction is rarely used. Most code calls `db::connect()` or `db::create_pool()` directly.

---

## Components Requiring Updates

### 1. crewchief-maproom Binary CLI (`crates/maproom/src/main.rs`)

**Severity**: HIGH
**Impact**: All CLI commands fail with SQLite

The main binary bypasses the `VectorStore` abstraction entirely:

| Line | Function | Issue |
|------|----------|-------|
| 386 | `auto_generate_embeddings()` | Direct PostgreSQL query |
| 522 | `db migrate` command | `db::connect()` |
| 532 | `db cleanup-stale` | `db::connect()` |
| 681 | `scan` command | `db::connect()` |
| 700, 715 | Scan worktree lookup | `db::get_or_create_*()` |
| 766 | Parallel scan | `db::create_pool()` |
| 853-903 | State persistence | `db::connect()`, `db::get_or_create_*()` |
| 944 | `upsert` command | `db::connect()` |
| 1002 | `watch` command | `db::connect()` |
| 1013-1014 | `search` command | `db::connect()`, `db::search_chunks_fts()` |
| 1033-1082 | `vector-search` command | `db::connect()`, raw queries |
| 1224 | `generate-embeddings` | `db::connect()` |
| 1266 | `migrate` commands | `db::connect()` |

**Required Changes**:
- Replace all `db::connect()` calls with `db::factory::get_store()`
- Update functions to accept `&dyn VectorStore` instead of `&Client`
- Add SQLite-specific implementations for direct queries

---

### 2. Daemon (`crates/maproom/src/daemon/mod.rs`)

**Severity**: HIGH
**Impact**: MCP server cannot function with SQLite

```rust
// Line 8 - PostgreSQL-specific imports
use crewchief_maproom::db::{create_pool, PgPool};

// Line 25 - PostgreSQL pool creation
let pool = create_pool().await?;
```

The daemon contains:
- 15+ raw PostgreSQL queries with `maproom.` schema prefix
- `PgPool` type hardcoded into `DaemonState`
- All search execution uses PostgreSQL client directly

**Required Changes**:
- Replace `PgPool` with `Arc<dyn VectorStore>`
- Move queries into `VectorStore` trait methods
- Update `DaemonState` to be database-agnostic

---

### 3. Rust Query Files (30+ files)

**Severity**: HIGH
**Impact**: Core functionality broken

Files containing hardcoded `maproom.chunks`, `maproom.repos`, `maproom.worktrees`, `maproom.files`:

| Directory | Files | Example Queries |
|-----------|-------|-----------------|
| `db/` | queries.rs, cleanup.rs, pool.rs, index_state.rs | CRUD, migrations |
| `search/` | fts.rs, vector.rs, graph.rs, fusion/mod.rs | Search execution |
| `embedding/` | pipeline.rs | Batch updates |
| `indexer/` | mod.rs | File/chunk insertion |
| `daemon/` | mod.rs | JSON-RPC handlers |
| `context/` | assembler.rs, graph.rs, relationships.rs | Context building |
| `context/strategies/` | default.rs, rust.rs, python.rs, react.rs | Language strategies |
| `incremental/` | detector.rs, processor.rs, edge_updater.rs, tree_sha_update.rs | Incremental indexing |
| `migrate/` | markdown.rs | Content migration |
| `upsert.rs` | - | File upsert |

**Required Changes**:
- Abstract queries into `VectorStore` trait methods
- Move PostgreSQL-specific queries to `db/postgres/queries.rs`
- Implement equivalent methods in `db/sqlite/` modules

---

### 4. VSCode Extension (`packages/vscode-maproom/`)

**Severity**: MEDIUM
**Impact**: Extension requires unnecessary PostgreSQL setup

| File | Issue |
|------|-------|
| `src/services/postgres-checker.ts` | Entire file is PostgreSQL-specific |
| `config/docker-compose.yml` | Defines pgvector container |
| `config/docker-compose.test.yml` | Test container config |
| VSCode settings contributions | References database host/port settings |

**Required Changes**:
- Replace `postgres-checker.ts` with `database-checker.ts` (check SQLite file exists)
- Make Docker containers optional (for PostgreSQL mode only)
- Update settings schema for SQLite file path option
- Simplify activation (no external services required)

---

### 5. MCP Server (`packages/maproom-mcp/`)

**Severity**: MEDIUM
**Impact**: MCP requires PostgreSQL URL format

| File | Line | Issue |
|------|------|-------|
| `src/daemon.ts` | 55 | `MAPROOM_DATABASE_URL` required (PostgreSQL format) |
| `src/utils/resolve-database.ts` | - | PostgreSQL-specific defaults |
| `tests/helpers/database.ts` | - | Uses `pg` npm package |

**Required Changes**:
- Update URL parsing to support `sqlite://` scheme
- Add SQLite file path detection
- Refactor test helpers to support both backends (or SQLite-only)

---

### 6. Docker/Configuration Files

**Severity**: LOW
**Impact**: Unnecessary infrastructure complexity

| File | Action |
|------|--------|
| `config/docker-compose.yml` | Make optional or remove |
| `.devcontainer/docker-compose.yml` | Make postgres service optional |
| `packages/vscode-maproom/config/docker-compose.yml` | Make optional |
| `config/init.sql` | PostgreSQL-only, keep for PG mode |

**Required Changes**:
- Document SQLite as default (zero-config)
- Make PostgreSQL containers optional for advanced users

---

### 7. CI/CD (`.github/workflows/test.yml`)

**Severity**: LOW
**Impact**: CI still requires PostgreSQL service

| Section | Issue |
|---------|-------|
| Lines 64-79 | PostgreSQL service container definition |
| Line 84 | `TEST_MAPROOM_DATABASE_URL` with PostgreSQL URL |
| Lines 134-172 | Database migration using PostgreSQL |

**Required Changes**:
- Add SQLite test job (no external services)
- Keep PostgreSQL job for backward compatibility
- Make SQLite the primary/default test path

---

### 8. Documentation

**Severity**: LOW
**Impact**: Incorrect setup instructions

| File | Updates Needed |
|------|----------------|
| `docs/architecture/DATABASE_ARCHITECTURE.md` | Add SQLite architecture |
| `crates/maproom/CLAUDE.md` | Update environment variables |
| `packages/maproom-mcp/CLAUDE.md` | Update database connection docs |
| `packages/vscode-maproom/` (if exists) | Update setup instructions |
| Root `CLAUDE.md` | Update quick start |

---

## Components NOT Requiring Changes

### crewchief CLI (`packages/cli/`)

The TypeScript CLI does **not** directly interact with the database:
- Uses `maproom-mcp` package for all database operations
- Commands spawn the `crewchief-maproom` binary
- Database access is abstracted through MCP/daemon layers

**No changes required** to `packages/cli/`.

---

## Recommended Follow-on Project

### SQLITE-INTEGRATION Project

**Objective**: Complete SQLite integration across all consumption layers

#### Phase 1: Core Abstraction (High Priority)
| Ticket | Description | Estimate |
|--------|-------------|----------|
| INT-1001 | Abstract daemon to use `VectorStore` trait | 2-3 days |
| INT-1002 | Update main.rs commands to use `get_store()` | 2-3 days |
| INT-1003 | Move PostgreSQL queries to `db/postgres/queries.rs` | 3-4 days |

#### Phase 2: Query Implementation (High Priority)
| Ticket | Description | Estimate |
|--------|-------------|----------|
| INT-2001 | Implement search queries in `VectorStore` trait | 2-3 days |
| INT-2002 | Implement context queries in `VectorStore` trait | 2-3 days |
| INT-2003 | Implement incremental queries in `VectorStore` trait | 2 days |

#### Phase 3: Infrastructure (Medium Priority)
| Ticket | Description | Estimate |
|--------|-------------|----------|
| INT-3001 | Update VSCode extension for SQLite | 2 days |
| INT-3002 | Update MCP server URL handling | 1 day |
| INT-3003 | Update CI/CD for SQLite-first testing | 1 day |

#### Phase 4: Cleanup (Low Priority)
| Ticket | Description | Estimate |
|--------|-------------|----------|
| INT-4001 | Make Docker containers optional | 1 day |
| INT-4002 | Update all documentation | 1 day |
| INT-4003 | Remove deprecated PostgreSQL-only code paths | 1 day |

**Total Estimate**: 18-24 days

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking PostgreSQL support | Medium | High | Keep both backends, feature-flag SQLite |
| Performance regression | Low | Medium | Benchmark before/after |
| Missing query translations | Medium | High | Comprehensive test coverage |
| CI/CD disruption | Low | Medium | Gradual migration, parallel jobs |

---

## Conclusion

The SQLITE project provides a solid foundation with proper storage layer abstraction. However, significant work remains to update the consumption layer. A follow-on SQLITE-INTEGRATION project should:

1. **Prioritize** daemon and CLI command updates (blocks all functionality)
2. **Abstract** PostgreSQL-specific queries into the `VectorStore` trait
3. **Simplify** infrastructure by making Docker optional
4. **Maintain** backward compatibility with PostgreSQL for advanced users

The end goal is **zero-configuration operation**: users run `crewchief-maproom scan` and get a local `.maproom.db` file without any external services.

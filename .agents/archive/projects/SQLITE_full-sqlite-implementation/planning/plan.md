# Plan: Full SQLite Implementation

## Overview

Implement a complete SQLite-based storage and search backend for Maproom. This is a **SQLite-only implementation** - no trait abstraction or PostgreSQL parity concerns.

**Starting Point**: SQLFIX project completed basic CRUD and FTS5. This project adds vector search, embedding deduplication, hybrid search, and graph traversal.

**MVP Scope**: 1536-dim embeddings only. 768-dim support deferred to post-MVP.

## Phase 0: Migration Infrastructure (BLOCKING)

**Goal**: Establish versioned migration system before any schema changes

**Est. Time**: 1-2 days

> **CRITICAL**: This phase MUST complete before ANY schema changes in Phase 1+.
> The migration system is a blocking prerequisite. Do not skip or parallelize.

### Ticket: Migration System
- Create `schema_migrations` table to track applied migrations
- Implement `get_current_version()` function
- Implement `run_migrations()` that applies pending migrations in order
- Implement `needs_migration()` check
- Create migration file infrastructure (separate SQL files or embedded)
- Add rollback capability (best-effort, not guaranteed)

**Agent**: rust-indexer-engineer

### Ticket: Extension Verification
- Add runtime check that sqlite-vec extension is loaded correctly
- Implement graceful fallback if extension missing (FTS-only mode)
- Add clear error message if extension fails to load
- Test extension verification in CI

**Agent**: rust-indexer-engineer

## Phase 1: Schema Foundation

**Goal**: Establish correct schema with junction table and embedding storage

**Est. Time**: 1-2 days

### Ticket: Schema Migration
- Remove `worktree_ids JSON` column from chunks table (via migration)
- Add `chunk_worktrees` junction table
- Add `code_embeddings` table for blob_sha deduplication
- Add `vec_code` virtual table for 1536-dim vectors (768-dim deferred)
- Update FTS5 sync (manual INSERT, not triggers - matches PostgreSQL pattern)
- Create versioned migration files using Phase 0 infrastructure

**Agent**: rust-indexer-engineer

### Ticket: CRUD Updates for Junction Table
- Update `insert_chunk` to use junction table instead of JSON
- Add `add_chunk_to_worktree()` method
- Add `get_chunk_worktrees()` method
- Update all queries that filter by worktree to JOIN junction table
- Ensure batch operations use transactions

**Agent**: rust-indexer-engineer

## Phase 2: Embedding Storage

**Goal**: Implement deduplicated embedding storage

**Est. Time**: 2-3 days

### Ticket: Embedding Module
- Create `embeddings.rs` module
- Implement `upsert_embedding(blob_sha, code_embedding, text_embedding)`
- Implement `upsert_embeddings_batch()` with deduplication
- Implement `has_embedding(blob_sha)` check
- Convert Vec<f32> to/from BLOB correctly for sqlite-vec (little-endian bytes)
- Support 1536-dim only for MVP (768-dim deferred)

**Agent**: rust-indexer-engineer

### Ticket: Vector Table Population
- Sync `code_embeddings` table with `vec_code` virtual table
- Implement rowid mapping between tables (code_embeddings.id -> vec_code.rowid)
- 1536-dim only for MVP

> **Note**: No data migration needed. Fresh indexing populates all tables from scratch.

**Agent**: rust-indexer-engineer

## Phase 3: Vector Search

**Goal**: Implement similarity search via sqlite-vec

**Est. Time**: 2-3 days

### Ticket: Vector Search Module
- Create `vector.rs` module
- Implement `search_vector(query_embedding, repo, worktree, limit)`
- Lazy-load sqlite-vec extension on first vector operation
- Join through `chunks` → `code_embeddings` → `vec_code`
- Convert L2 distance to similarity score (0-1, higher = better)
- Handle missing extension gracefully

**Agent**: rust-indexer-engineer

### Ticket: Vector Search Tests
- Test basic similarity search
- Test empty index behavior
- Test worktree filtering
- Test extension missing scenario (graceful degradation)

**Agent**: rust-indexer-engineer

## Phase 4: Hybrid Search

**Goal**: Combine FTS5 and vector search with RRF fusion

**Est. Time**: 3-4 days

### Ticket: FTS Module Extraction
- Extract FTS logic from `mod.rs` to `fts.rs`
- Refactor `search_chunks_fts` to return `FtsResult` with rank position
- Normalize FTS5 rank to 0-1 scale
- Improve query building for edge cases
- Add worktree filter via junction table JOIN

**Agent**: rust-indexer-engineer

### Ticket: Hybrid Search Module
- Create `hybrid.rs` module
- Implement RRF (Reciprocal Rank Fusion) algorithm
- Implement `search_hybrid(query, query_embedding, ...)`
- Configurable weights for FTS vs vector
- Fallback to FTS-only when no embeddings
- Return `SearchHit` with combined scores

**Agent**: rust-indexer-engineer

### Ticket: Semantic Ranking
- Add `SemanticRanking` struct with multipliers
- Apply `kind_multipliers` (function=1.2, class=1.1, etc.)
- Apply `exact_match_boost` for symbol name matches (import `normalize_for_exact_match()` from `src/search/fts.rs`)
- Factor in `recency_score` from chunks
- Integrate into hybrid search pipeline

**Agent**: rust-indexer-engineer

## Phase 5: Graph Traversal

**Goal**: Support caller/callee and import relationship queries

**Est. Time**: 2-3 days

### Ticket: Graph Module
- Create `graph.rs` module
- Implement `find_callers(chunk_id, max_depth)` with recursive CTE
- Implement `find_callees(chunk_id, max_depth)` with recursive CTE
- Implement `find_imports(chunk_id, direction)`
- Handle cycles gracefully (visited tracking)
- Return path information for visualization

**Agent**: rust-indexer-engineer

### Ticket: Graph Tests
- Test direct relationships
- Test transitive relationships (depth > 1)
- Test cycle handling
- Test with empty graph
- Test depth limiting

**Agent**: rust-indexer-engineer

## Phase 6: Integration Testing

**Goal**: Validate complete pipeline works end-to-end

**Est. Time**: 2-3 days

### Ticket: Integration Test Suite
- Test full index→embed→search cycle
- Test multi-worktree scenarios
- Test embedding deduplication across files
- Test graph traversal accuracy
- Performance sanity checks (not benchmarks)

**Agent**: rust-indexer-engineer, unit-test-runner

### Ticket: Final Verification
- Run all tests: `cargo test --features sqlite`
- Verify no regressions in existing functionality
- Document any known limitations
- Update CLAUDE.md if needed

**Agent**: verify-ticket

## Dependencies

```
Phase 0 (Migration) ─────► Phase 1 (Schema) ─────┬──► Phase 2 (Embeddings) ──► Phase 3 (Vector)
      ▲                                           │                                    │
      │                                           │                                    ▼
   BLOCKING                                       └──► Phase 4a (FTS Extract) ──► Phase 4b (Hybrid)
                                                                                        │
                                                                                        ▼
                                                                                 Phase 4c (Ranking)
                                                                                        │
Phase 5 (Graph) ◄───────────────────────────────────────────────────────────────────────┘
     │
     ▼
Phase 6 (Integration)
```

Note:
- **Phase 0 (Migration) is BLOCKING** - Must complete before ANY schema changes
- Phase 5 (Graph) can start in parallel with Phase 3-4 after Phase 1 completes
- Code reorganization done incrementally within each phase (no separate reorg phase)

## Agent Assignments

| Phase | Tickets | Primary Agent | Est. |
|-------|---------|---------------|------|
| 0 | Migration System, Extension Verification | rust-indexer-engineer | 1-2d |
| 1 | Schema, CRUD Updates | rust-indexer-engineer | 1-2d |
| 2 | Embeddings, Vector Tables | rust-indexer-engineer | 2-3d |
| 3 | Vector Search, Tests | rust-indexer-engineer | 2-3d |
| 4 | FTS, Hybrid, Ranking | rust-indexer-engineer | 3-4d |
| 5 | Graph, Tests | rust-indexer-engineer | 2-3d |
| 6 | Integration, Verify | rust-indexer-engineer, unit-test-runner | 2-3d |

**Total Estimate: 14-20 days**

## Success Criteria

### Per-Phase Criteria

| Phase | Criterion |
|-------|-----------|
| 0 | Migration system works, extension verification passes |
| 1 | Schema migration runs without error, junction table created |
| 2 | Embeddings stored with deduplication verified |
| 3 | Vector search returns similar chunks |
| 4 | Hybrid search combines FTS+vector, ranking applied |
| 5 | Graph traversal returns correct paths |
| 6 | All tests pass, no regressions |

### Project Completion Criteria

```bash
# All must pass:
cargo check --features sqlite
cargo test --features sqlite
cargo clippy --features sqlite -- -D warnings

# Critical specific tests:
cargo test --features sqlite test_migration_upgrade_path
cargo test --features sqlite test_extension_missing_graceful
cargo test --features sqlite test_file_based_integration
```

**Manual verification:**
1. Index a real codebase with hybrid search
2. Run search query, verify relevant ranked results
3. Switch branches, verify embedding dedup works (no re-embedding unchanged content)
4. Kill process mid-index, verify WAL recovery works

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| sqlite-vec not bundled correctly | Test extension loading in CI |
| Performance issues at scale | Add basic perf tests, profile if slow |
| Schema migration fails on fresh DB | Version migrations, test fresh database creation |
| FTS5 query syntax edge cases | Comprehensive query sanitization |

> **Note**: No existing SQLite databases require data migration. All data populated via fresh indexing.

## Known Limitations (MVP)

These are explicit MVP boundaries, not bugs:

- **1536-dim embeddings only** - OpenAI/Vertex compatible. 768-dim (Ollama) deferred.
- **No database encryption** - Database file contains code snippets. Treat as sensitive as source.
- **Single-user only** - No concurrent multi-process access. WAL mode handles single-user concurrency.
- **No PostgreSQL migration** - This is a parallel implementation, not a replacement path.

## Deferred to Post-MVP

- 768-dim embedding support (Ollama) - requires separate vec_code_768 table and routing
- Custom FTS5 BM25 column weights
- Typed SqliteError enum (use anyhow for MVP)

## Out of Scope

- VSCode extension integration (separate SQLITE_EXT project)
- PostgreSQL parity or shared abstractions
- Database encryption
- Multi-user access
- Network/remote access

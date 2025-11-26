# SQLITE Ticket Index

## Project Overview

**Project**: SQLITE - Full SQLite Implementation
**Total Tickets**: 14
**Estimated Duration**: 14-20 days
**Primary Agent**: rust-indexer-engineer

## Ticket Summary by Phase

| Phase | Ticket ID | Title | Status | Est. |
|-------|-----------|-------|--------|------|
| 0 | SQLITE-0001 | Migration System | Not Started | 4-6h |
| 0 | SQLITE-0002 | Extension Verification | Not Started | 2-4h |
| 1 | SQLITE-1001 | Schema Migration | Not Started | 4-6h |
| 1 | SQLITE-1002 | CRUD Updates for Junction Table | Not Started | 4-6h |
| 2 | SQLITE-2001 | Embedding Module | Not Started | 4-6h |
| 2 | SQLITE-2002 | Vector Table Population | Not Started | 3-4h |
| 3 | SQLITE-3001 | Vector Search Module | Not Started | 4-6h |
| 3 | SQLITE-3901 | Vector Search Tests | Not Started | 2-4h |
| 4 | SQLITE-4001 | FTS Module Extraction | Not Started | 4-6h |
| 4 | SQLITE-4002 | Hybrid Search Module | Not Started | 4-6h |
| 4 | SQLITE-4003 | Semantic Ranking | Not Started | 3-4h |
| 5 | SQLITE-5001 | Graph Module | Not Started | 4-6h |
| 5 | SQLITE-5901 | Graph Tests | Not Started | 2-4h |
| 6 | SQLITE-6001 | Integration Test Suite | Not Started | 4-6h |
| 6 | SQLITE-6002 | Final Verification | Not Started | 2-4h |

## Phase Details

### Phase 0: Migration Infrastructure (BLOCKING)
**Est. Time**: 1-2 days
**Status**: Not Started

> **CRITICAL**: Must complete before any schema changes in Phase 1+

| Ticket | Description | Dependencies |
|--------|-------------|--------------|
| [SQLITE-0001](SQLITE-0001_migration-system.md) | Schema versioning and migration runner | None |
| [SQLITE-0002](SQLITE-0002_extension-verification.md) | sqlite-vec extension verification with fallback | SQLITE-0001 |

### Phase 1: Schema Foundation
**Est. Time**: 1-2 days
**Status**: Not Started

| Ticket | Description | Dependencies |
|--------|-------------|--------------|
| [SQLITE-1001](SQLITE-1001_schema-migration.md) | Junction table, embedding tables, vec_code (no data migration) | SQLITE-0001, SQLITE-0002 |
| [SQLITE-1002](SQLITE-1002_crud-junction-table.md) | Update CRUD to use junction table | SQLITE-1001 |

### Phase 2: Embedding Storage
**Est. Time**: 2-3 days
**Status**: Not Started

| Ticket | Description | Dependencies |
|--------|-------------|--------------|
| [SQLITE-2001](SQLITE-2001_embedding-module.md) | Deduplicated embedding storage by blob_sha | SQLITE-1001 |
| [SQLITE-2002](SQLITE-2002_vector-table-population.md) | Sync embeddings to vec_code table | SQLITE-2001, SQLITE-0002 |

### Phase 3: Vector Search
**Est. Time**: 2-3 days
**Status**: Not Started

| Ticket | Description | Dependencies |
|--------|-------------|--------------|
| [SQLITE-3001](SQLITE-3001_vector-search-module.md) | sqlite-vec similarity search | SQLITE-2002, SQLITE-1002 |
| [SQLITE-3901](SQLITE-3901_vector-search-tests.md) | Vector search test suite | SQLITE-3001 |

### Phase 4: Hybrid Search
**Est. Time**: 3-4 days
**Status**: Not Started

| Ticket | Description | Dependencies |
|--------|-------------|--------------|
| [SQLITE-4001](SQLITE-4001_fts-module-extraction.md) | Extract FTS to module, normalize ranks | SQLITE-1002 |
| [SQLITE-4002](SQLITE-4002_hybrid-search-module.md) | RRF fusion of FTS + vector | SQLITE-4001, SQLITE-3001 |
| [SQLITE-4003](SQLITE-4003_semantic-ranking.md) | Kind multipliers, exact match boost | SQLITE-4002 |

### Phase 5: Graph Traversal
**Est. Time**: 2-3 days
**Status**: Not Started

| Ticket | Description | Dependencies |
|--------|-------------|--------------|
| [SQLITE-5001](SQLITE-5001_graph-module.md) | Recursive CTE caller/callee traversal | SQLITE-1001 |
| [SQLITE-5901](SQLITE-5901_graph-tests.md) | Graph traversal test suite | SQLITE-5001 |

### Phase 6: Integration Testing
**Est. Time**: 2-3 days
**Status**: Not Started

| Ticket | Description | Dependencies |
|--------|-------------|--------------|
| [SQLITE-6001](SQLITE-6001_integration-test-suite.md) | End-to-end integration tests | All Phase 0-5 |
| [SQLITE-6002](SQLITE-6002_final-verification.md) | Final verification and documentation | SQLITE-6001 |

## Dependency Graph

```
Phase 0 (BLOCKING)
├── SQLITE-0001 Migration System
└── SQLITE-0002 Extension Verification
         │
         ▼
Phase 1 Schema
├── SQLITE-1001 Schema Migration ────┬──────────────────┐
└── SQLITE-1002 CRUD Junction        │                  │
         │                           │                  │
         ▼                           ▼                  ▼
Phase 2 Embeddings               Phase 4 FTS       Phase 5 Graph
├── SQLITE-2001 Embedding Module  SQLITE-4001      SQLITE-5001
└── SQLITE-2002 Vector Population     │            SQLITE-5901
         │                            │
         ▼                            ▼
Phase 3 Vector                   SQLITE-4002 Hybrid
├── SQLITE-3001 Vector Search         │
└── SQLITE-3901 Vector Tests          ▼
         │                       SQLITE-4003 Ranking
         └─────────────────────────────┘
                       │
                       ▼
              Phase 6 Integration
              ├── SQLITE-6001 Integration Tests
              └── SQLITE-6002 Final Verification
```

## Critical Path

The following tickets are on the critical path and cannot be parallelized:

1. **SQLITE-0001** → **SQLITE-1001** → **SQLITE-2001** → **SQLITE-3001** → **SQLITE-4002**

This represents the core pipeline: migrations → schema → embeddings → vector search → hybrid search.

## Parallel Opportunities

After Phase 1 completes:
- Phase 4a (FTS extraction) can run in parallel with Phase 2-3
- Phase 5 (Graph) can run in parallel with Phase 3-4

## Success Criteria

```bash
# All must pass on completion:
cargo check --features sqlite
cargo test --features sqlite
cargo clippy --features sqlite -- -D warnings

# Critical path tests:
cargo test --features sqlite test_migration_upgrade_path
cargo test --features sqlite test_extension_missing_graceful
cargo test --features sqlite test_file_based_integration
```

## Known Limitations (MVP)

- 1536-dim embeddings only (OpenAI/Vertex compatible)
- 768-dim (Ollama) deferred to post-MVP
- No database encryption
- Single-user only

## Plan References

- [Project Plan](../planning/plan.md)
- [Architecture](../planning/architecture.md)
- [Quality Strategy](../planning/quality-strategy.md)

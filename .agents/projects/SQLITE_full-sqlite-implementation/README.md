# SQLITE: Full SQLite Implementation

## Summary

Complete the SQLite backend for Maproom to enable zero-config semantic code search without PostgreSQL/Docker dependencies.

## Problem Statement

The current Maproom implementation requires PostgreSQL with pgvector, creating adoption friction:
- Docker must be running
- Database configuration across environments is complex
- First-time setup is non-trivial

A complete SQLite implementation enables:
- Single-file database (no external services)
- Works across IDEs, devcontainers, machines
- Zero configuration for new users

## Current State

The SQLFIX project established:
- Basic CRUD operations (working)
- FTS5 full-text search (working)
- 10 unit tests passing

**Missing**:
- Vector similarity search
- Embedding deduplication
- Hybrid search (FTS + vector fusion)
- Graph traversal queries
- Multi-worktree tracking (junction table)

## Proposed Solution

Build a complete SQLite-native implementation (not a trait abstraction):

1. **Schema**: Junction table for worktrees, embedding deduplication tables
2. **Embeddings**: Content-addressed storage by blob_sha
3. **Vector Search**: sqlite-vec extension for similarity queries
4. **Hybrid Search**: RRF fusion of FTS5 + vector results
5. **Semantic Ranking**: Kind multipliers, exact match boosts
6. **Graph Traversal**: Recursive CTEs for caller/callee chains

## Relevant Agents

| Agent | Role |
|-------|------|
| rust-indexer-engineer | Primary implementer for all phases |
| unit-test-runner | Test execution and verification |
| verify-ticket | Final acceptance verification |

## Planning Documents

- [Analysis](planning/analysis.md) - Problem definition and current state
- [Architecture](planning/architecture.md) - Technical design and module structure
- [Quality Strategy](planning/quality-strategy.md) - Testing approach
- [Security Review](planning/security-review.md) - Security considerations
- [Plan](planning/plan.md) - Phased implementation plan

## Phases

| Phase | Focus | Deliverable | Est. |
|-------|-------|-------------|------|
| 0 | Migration Infrastructure | Versioned migrations, extension verification | 1-2d |
| 1 | Schema Foundation | Junction table, embedding tables | 1-2d |
| 2 | Embedding Storage | Deduplicated embedding storage | 2-3d |
| 3 | Vector Search | sqlite-vec similarity queries | 2-3d |
| 4 | Hybrid Search | RRF fusion, semantic ranking | 3-4d |
| 5 | Graph Traversal | Recursive CTE queries | 2-3d |
| 6 | Integration Testing | End-to-end validation | 2-3d |

**Total: ~14-20 days**

**CRITICAL**: Phase 0 (Migration Infrastructure) MUST complete before any schema changes. The migration system is a blocking prerequisite.

## Success Criteria

```bash
# All must pass
cargo check --features sqlite
cargo test --features sqlite
cargo clippy --features sqlite -- -D warnings

# Specific critical tests
cargo test --features sqlite test_migration_upgrade_path
cargo test --features sqlite test_extension_missing_graceful
cargo test --features sqlite test_file_based_integration
```

**Manual verification:**
1. Index a real codebase with hybrid search
2. Run search query, verify relevant ranked results
3. Switch branches, verify embedding dedup works (no re-embedding unchanged content)
4. Kill process mid-index, verify WAL recovery works

## Known Limitations

These are explicit MVP boundaries, not bugs:

- **1536-dim embeddings only** - OpenAI/Vertex compatible. 768-dim (Ollama) deferred to post-MVP.
- **No database encryption** - Database file contains code snippets. Treat as sensitive as source.
- **Single-user only** - No concurrent multi-process access. WAL mode handles single-user concurrency.
- **No PostgreSQL migration** - This is a parallel implementation, not a replacement path.

## Out of Scope

- VSCode extension integration (separate project)
- PostgreSQL compatibility or shared abstractions
- Database encryption (enterprise feature)
- Multi-user/network access

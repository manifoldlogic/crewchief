# VECSTORE: VectorStore Trait Completion

## Project Summary

Complete the `VectorStore` trait abstraction so both PostgreSQL and SQLite backends can be used interchangeably. This is the **foundation project** that enables all downstream SQLite integration work.

**Ollama Priority**: This project includes CRITICAL work to enable 768-dimensional embeddings in SQLite, which unblocks the zero-config experience (SQLite + Ollama) and the EMBPERF optimization project.

## Problem Statement

The `crewchief-maproom` codebase has two database backends (PostgreSQL and SQLite), but the `VectorStore` trait is incomplete. Many operations bypass the trait and call PostgreSQL-specific code directly:

- CLI commands use `db::connect()` instead of the trait
- Daemon uses raw `pool.get()` instead of `Arc<dyn VectorStore>`
- Indexer mixes trait calls with direct pool access
- Vector/hybrid search not exposed through trait

This architectural gap prevents the SQLite backend from being used by the CLI, daemon, or indexer.

## Solution

Expand the `VectorStore` trait with all missing methods:
- Vector and hybrid search
- Context assembly (chunk lookup)
- Repository/worktree queries
- Index state management
- Cleanup operations

Implement all methods in both `PostgresStore` and `SqliteStore`.

## Scope

**In Scope**:
- VectorStore trait expansion
- PostgresStore implementation
- SqliteStore implementation
- Contract and parity tests

**Out of Scope** (separate projects):
- CLI migration to trait (MAPCLI)
- Daemon migration to trait (MAPCLI)
- MCP server updates (MCPDB)
- VSCode extension updates (VSCODEDB)
- CI/CD and documentation (SQLINFRA)

## Relevant Agents

- **rust-indexer-engineer**: Primary agent for trait implementation
- **database-engineer**: Backup for complex query optimization
- **integration-tester**: Test suite creation
- **unit-test-runner**: Test execution verification

## Planning Documents

- [Analysis](./planning/analysis.md) - Problem definition, current state, research findings
- [Architecture](./planning/architecture.md) - Solution design, ADRs, component structure
- [Quality Strategy](./planning/quality-strategy.md) - Testing approach, test layers, CI integration
- [Security Review](./planning/security-review.md) - Security assessment, risk analysis
- [Plan](./planning/plan.md) - Phases, tickets, agent assignments

## Tickets

| Ticket | Description | Agent | Priority |
|--------|-------------|-------|----------|
| **VECSTORE-1000** | **SQLite 768-dim Support** | rust-indexer-engineer | **CRITICAL** |
| VECSTORE-1001 | Vector Search Methods | rust-indexer-engineer | High |
| VECSTORE-1002 | Hybrid Search Methods | rust-indexer-engineer | High |
| VECSTORE-1003 | Context Assembly Methods | rust-indexer-engineer | Medium |
| VECSTORE-1004 | Repository Query Methods | rust-indexer-engineer | Medium |
| VECSTORE-1005 | Index State Methods | rust-indexer-engineer | Medium |
| VECSTORE-1006 | Cleanup Methods | rust-indexer-engineer | Medium |
| VECSTORE-1007 | Contract and Parity Tests | integration-tester | High |

## Success Criteria

1. **SQLite supports 768-dim embeddings** (Ollama zero-config works)
2. `cargo test --features sqlite` passes all trait tests (both 768 and 1536 dim)
3. `cargo test` (PostgreSQL) passes all trait tests
4. No raw SQL queries outside `db/postgres/` or `db/sqlite/`
5. `get_store()` returns working store for both backends
6. All new trait methods implemented in both `PostgresStore` and `SqliteStore`
7. Contract tests verify both backends implement trait correctly

**Note:** CLI/daemon/indexer migration to use `Arc<dyn VectorStore>` is handled by MAPCLI project after VECSTORE completes.

## Dependencies

- **Blocks**: MAPCLI, MCPDB, VSCODEDB, SQLINFRA, **EMBPERF**
- **Blocked By**: None (foundation project)

**EMBPERF Relationship**: The EMBPERF project (Ollama parallel optimization) produces 768-dim embeddings. VECSTORE-1000 must complete before EMBPERF can be fully utilized with SQLite backends.

## Status

**Phase**: Planning Complete
**Next Step**: Create tickets with `/create-project-tickets VECSTORE`

# MAPCLI - Maproom CLI Abstraction

**Status**: ✅ Complete (Archived 2025-11-26)

## Project Summary

Update the `crewchief-maproom` CLI binary and daemon to use the `VectorStore` trait abstraction, enabling SQLite backend support for zero-configuration semantic search.

## Problem Statement

The CLI and daemon currently use direct PostgreSQL connections (`db::connect()`, `PgPool`), preventing use of the SQLite backend implemented in VECSTORE. Users must configure PostgreSQL (via Docker or external service) before using semantic search features.

## Proposed Solution

Replace direct database access with the `VectorStore` trait abstraction:

1. **Factory Pattern**: Use `get_store()` to obtain appropriate backend based on URL
2. **Trait Methods**: Route all queries through `VectorStore` trait methods
3. **Auto-Detection**: Detect SQLite database at `~/.maproom/maproom.db` by default
4. **Graceful Fallback**: Disable PostgreSQL-specific features (parallel scan) for SQLite

## MVP Scope (Phase 1)

Focus on search, status, and daemon operations with SQLite:

1. **Search commands** work with both PostgreSQL and SQLite backends
2. **Status command** works with both backends (refactored to use trait methods)
3. **Daemon** serves JSON-RPC requests using VectorStore trait
4. Zero-configuration operation with SQLite default
5. Preserve all existing PostgreSQL functionality

**Deferred to Phase 2** (requires indexer abstraction):
- `scan` command with SQLite
- `upsert` command with SQLite
- `watch` command with SQLite
- `generate-embeddings` with SQLite

## Dependencies

- **VECSTORE** (completed 2025-11-26): VectorStore trait with both backend implementations

## Relevant Agents

| Agent | Role |
|-------|------|
| rust-indexer-engineer | Primary implementation agent for Rust CLI/daemon |
| integration-tester | Creates E2E test suite |
| unit-test-runner | Executes test suites |
| verify-ticket | Verifies acceptance criteria |
| commit-ticket | Creates commits |

## Planning Documents

- [Analysis](planning/analysis.md) - Problem definition and research findings
- [Architecture](planning/architecture.md) - Design decisions and component changes
- [Quality Strategy](planning/quality-strategy.md) - Testing approach and verification
- [Security Review](planning/security-review.md) - Security assessment
- [Plan](planning/plan.md) - Phased execution plan with tickets

## Tickets

| Ticket | Summary | Status |
|--------|---------|--------|
| [MAPCLI-1000](tickets/MAPCLI-1000_add-backend-type-enum.md) | Add BackendType Enum and Trait Method | Pending |
| [MAPCLI-1001](tickets/MAPCLI-1001_main-rs-factory-pattern.md) | Update main.rs to use get_store() Factory | Pending |
| [MAPCLI-1002](tickets/MAPCLI-1002_daemon-vectorstore-refactor.md) | Refactor Daemon to use VectorStore Trait | Pending |
| [MAPCLI-1003](tickets/MAPCLI-1003_sqlite-backend-detection.md) | Add SQLite Backend Detection/Configuration | Pending |
| [MAPCLI-1004](tickets/MAPCLI-1004_cli-commands-status-refactor.md) | Update CLI Commands and Refactor status.rs | Pending |
| [MAPCLI-1005](tickets/MAPCLI-1005_e2e-integration-tests.md) | E2E Integration Tests with SQLite Backend | Pending |

**Ticket Index**: [MAPCLI_TICKET_INDEX.md](tickets/MAPCLI_TICKET_INDEX.md)

## Success Metrics

### Phase 1 (MVP)
1. `crewchief-maproom search` returns results from SQLite
2. `crewchief-maproom status` works with SQLite
3. `crewchief-maproom serve` daemon works with SQLite (all search modes)
4. `status.rs` refactored to use VectorStore trait (no direct PostgreSQL connection)
5. All existing PostgreSQL tests pass
6. E2E test script passes with pre-indexed SQLite database

### Phase 2 (Future)
1. `crewchief-maproom scan` works with SQLite
2. `crewchief-maproom upsert` works with SQLite
3. `crewchief-maproom watch` works with SQLite

## Quick Reference

```bash
# Build with SQLite support
cargo build --features sqlite --bin crewchief-maproom

# Test PostgreSQL backend (existing)
cargo test

# Test SQLite backend
cargo test --features sqlite

# Run MVP commands with SQLite
export MAPROOM_DATABASE_URL="sqlite://~/.maproom/maproom.db"
cargo run --features sqlite --bin crewchief-maproom -- search --repo myrepo --query "function"
cargo run --features sqlite --bin crewchief-maproom -- status
cargo run --features sqlite --bin crewchief-maproom -- serve

# Note: scan/upsert/watch require PostgreSQL until Phase 2
```

# Project: SQLIMPL_sqlite-implementation

**Status**: Planning Complete (Reviewed & Updated)
**Created**: 2025-11-27
**Tickets**: 19 planned across 5 phases (15 core MVP + 4 optional)

## Problem Statement

The maproom indexer (`crates/maproom/`) has SQLite infrastructure in place but 52 functions are stubbed with TODO comments returning empty/placeholder values. Additionally, 35 test files reference PostgreSQL and don't run. Core features (semantic search, incremental indexing, watch command) don't work properly.

## Objective

Complete the SQLite implementation to make maproom fully functional:
- All 35 test files migrate to SQLite and pass
- Search returns real, ranked results (not empty)
- Incremental indexing persists file changes
- Watch command monitors and updates continuously

## Current State

| Command | Status |
|---------|--------|
| `scan` | Works (creates chunks) |
| `upsert` | Works (processes files) |
| `search` | Returns results but executors stubbed |
| `watch` | Disabled (returns error) |
| `cargo test` | 35 files don't compile |

## Scope

### In Scope
- 52 TODO stub implementations across 23 files
- 35 test file migrations from PostgreSQL to SQLite
- Enable watch command

### Out of Scope
- PostgreSQL restoration
- New features
- Schema changes
- Performance optimization

## Planning Documents

| Document | Description |
|----------|-------------|
| [analysis.md](planning/analysis.md) | Problem space analysis and current state |
| [architecture.md](planning/architecture.md) | Delegation patterns (wiring executors to SqliteStore) |
| [quality-strategy.md](planning/quality-strategy.md) | Test migration and validation approach |
| [security-review.md](planning/security-review.md) | Security considerations (SQL injection, paths) |
| [plan.md](planning/plan.md) | Phased execution plan with 19 tickets |
| [project-review.md](planning/project-review.md) | Critical review findings |
| [review-updates.md](planning/review-updates.md) | Changes made post-review |

## Phases

| Phase | Focus | Tickets | Status |
|-------|-------|---------|--------|
| 1 | Test Infrastructure | 1001-1005 | Core MVP |
| 2 | Search Wiring | 2001-2004 | Core MVP |
| 3 | Incremental Updates | 3001-3004 | Core MVP |
| 4 | Context Assembly | 4001-4004 | **Optional** |
| 5 | Watch Command | 5001-5002 | Core MVP |

**Key Insight:** Phase 2 is "wiring" executors to existing SqliteStore methods, not reimplementing SQL. The database layer is already complete.

## Primary Agents

| Agent | Role |
|-------|------|
| **rust-indexer-engineer** | Primary implementer |
| **unit-test-runner** | Test execution |
| **verify-ticket** | Acceptance validation |
| **commit-ticket** | Git commits |

## Success Criteria

```bash
# All tests pass
cargo test -p crewchief-maproom

# Search returns real results
cargo run --bin crewchief-maproom -- search "function" | grep -v "empty"

# Watch command works
cargo run --bin crewchief-maproom -- watch --repo test
```

## Related Projects

- **IDXABS** (Archived) - Previous project that removed PostgreSQL but left stubs
- **UNIWATCH** - Depends on this project's watch command implementation

## Next Steps

1. ~~Run `/review-project SQLIMPL` to validate project quality~~ ✅ Done
2. ~~Run `/create-project-tickets SQLIMPL` to generate tickets~~ ✅ Done (19 tickets created)
3. Run `/work-on-project SQLIMPL` to begin execution

# Project: SQLFIX - SQLite Backend Fixes

## Project Summary

Fix the broken SQLite backend implementation from the SQLVEC project (`SQLVEC_sqlite-vec-backend`). The original implementation left the codebase in a non-compiling state with multiple errors. This project addresses those issues to deliver a working SQLite storage backend for Maproom.

## Problem Statement

The SQLVEC project attempted to add SQLite+sqlite-vec as an alternative to PostgreSQL+pgvector, but was abandoned mid-implementation leaving:

- **4 compile-time errors** when building with `--features sqlite`
- **Missing module exports** (`schema.rs` not exported)
- **Dependency issues** (missing chrono feature for DateTime serialization)
- **Runtime issues** (FTS5 query syntax errors, schema column mismatches)
- **Incomplete integration** (factory pattern broken, VSCode extension corrupted)

## Proposed Solution

A focused fix project that:

1. **Commits prerequisite fixes** - Baseline changes from investigation
2. **Fixes compile errors** - Dependency and module export issues
3. **Establishes CI early** - Feature matrix testing catches regressions
4. **Completes CRUD operations** - Basic storage functionality works
5. **Adds test coverage** - Unit tests for SQLite backend

**Explicitly deferred**: Vector similarity search, VSCode SQLite mode, benchmarks

## Relevant Agents

| Agent | Role |
|-------|------|
| **rust-indexer-engineer** | Primary implementation - Rust code fixes and tests |
| **github-actions-specialist** | CI workflow updates for feature testing |

## Tickets

| ID | Description | Agent | Phase |
|----|-------------|-------|-------|
| SQLFIX-1000 | Commit baseline fixes (prerequisites) | rust-indexer-engineer | 0 |
| SQLFIX-1001 | Fix SQLite compilation (merged) | rust-indexer-engineer | 1 |
| SQLFIX-1005 | Update CI for SQLite feature | github-actions-specialist | 1 |
| SQLFIX-1002 | Fix schema initialization | rust-indexer-engineer | 2 |
| SQLFIX-1003 | Fix CRUD operations + FTS | rust-indexer-engineer | 2 |
| SQLFIX-1004 | Add SQLite unit tests | rust-indexer-engineer | 3 |

**Total**: 6 tickets across 4 phases

## Planning Documents

- [Analysis](./planning/analysis.md) - Problem definition and current state
- [Architecture](./planning/architecture.md) - Technical design and decisions
- [Quality Strategy](./planning/quality-strategy.md) - Testing approach
- [Security Review](./planning/security-review.md) - Security assessment
- [Implementation Plan](./planning/plan.md) - Phased execution plan
- [Project Review](./planning/project-review.md) - Pre-ticket review findings
- [Review Updates](./planning/review-updates.md) - Changes made after review

## Success Criteria

```bash
# All must pass
cargo check --features sqlite
cargo test --features sqlite
cargo check  # No regression in default postgres feature
```

## Related Projects

- **SQLVEC_sqlite-vec-backend** - Original (incomplete) implementation
- Future: SQLVEC2 - Vector search implementation in SQLite

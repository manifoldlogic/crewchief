# MAPCLI Ticket Index

**Project**: MAPCLI - Maproom CLI Abstraction
**Created**: 2025-11-26
**Plan Reference**: [plan.md](../planning/plan.md)

## Phase 1: MVP - SQLite Backend Support

### Prerequisites
| Ticket | Title | Status | Dependencies |
|--------|-------|--------|--------------|
| [MAPCLI-1000](MAPCLI-1000_add-backend-type-enum.md) | Add BackendType Enum and Trait Method | Pending | None |

### Foundation
| Ticket | Title | Status | Dependencies |
|--------|-------|--------|--------------|
| [MAPCLI-1001](MAPCLI-1001_main-rs-factory-pattern.md) | Update main.rs to use get_store() Factory | Pending | MAPCLI-1000 |

### Core Refactoring
| Ticket | Title | Status | Dependencies |
|--------|-------|--------|--------------|
| [MAPCLI-1002](MAPCLI-1002_daemon-vectorstore-refactor.md) | Refactor Daemon to use VectorStore Trait | Pending | MAPCLI-1000 |
| [MAPCLI-1003](MAPCLI-1003_sqlite-backend-detection.md) | Add SQLite Backend Detection/Configuration | Pending | MAPCLI-1000 |

### CLI Commands
| Ticket | Title | Status | Dependencies |
|--------|-------|--------|--------------|
| [MAPCLI-1004](MAPCLI-1004_cli-commands-status-refactor.md) | Update CLI Commands and Refactor status.rs | Pending | MAPCLI-1001 |

### Testing
| Ticket | Title | Status | Dependencies |
|--------|-------|--------|--------------|
| [MAPCLI-1005](MAPCLI-1005_e2e-integration-tests.md) | E2E Integration Tests with SQLite Backend | Pending | MAPCLI-1004 |

## Execution Order

```
MAPCLI-1000 (BackendType - PREREQUISITE)
    │
    ▼
MAPCLI-1001 (Foundation) ─────────┐
    │                             │
    ▼                             ▼
MAPCLI-1002 (Daemon)        MAPCLI-1003 (Detection)
    │                             │
    └──────────┬──────────────────┘
               ▼
        MAPCLI-1004 (Commands)
               │
               ▼
        MAPCLI-1005 (Testing)
```

## Summary

- **Total Tickets**: 6
- **Phase 1 (MVP)**: 6 tickets
- **Phase 2 (Deferred)**: Indexer abstraction (separate project)

## MVP Scope

**Included**:
- Search commands with SQLite
- Status command with SQLite
- Daemon with SQLite (all search modes)
- Backend auto-detection
- db cleanup-stale with SQLite

**Deferred to Phase 2**:
- scan/upsert/watch with SQLite
- generate-embeddings with SQLite
- Parallel scan for SQLite

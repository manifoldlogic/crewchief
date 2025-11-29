# GITPOLL Ticket Index

## Project Overview

Replace the notify-based file watcher with git status polling to eliminate "too many open files" errors on large repositories.

## Ticket Summary

| Ticket | Title | Status | Agent |
|--------|-------|--------|-------|
| [GITPOLL-1001](GITPOLL-1001_git-state-module.md) | Implement GitState module | Pending | rust-indexer-engineer |
| [GITPOLL-1002](GITPOLL-1002_git-poller-module.md) | Implement GitPoller module | Pending | rust-indexer-engineer |
| [GITPOLL-1901](GITPOLL-1901_unit-tests.md) | Unit tests for parsing and diffing | Pending | rust-indexer-engineer |
| [GITPOLL-2001](GITPOLL-2001_watcher-integration.md) | Integrate GitPoller into watcher.rs | Pending | rust-indexer-engineer |
| [GITPOLL-2002](GITPOLL-2002_worktree-watcher-integration.md) | Update WorktreeWatcher integration | Pending | rust-indexer-engineer |
| [GITPOLL-2901](GITPOLL-2901_integration-tests.md) | Integration tests with temp git repos | Pending | rust-indexer-engineer |
| [GITPOLL-3001](GITPOLL-3001_cleanup-and-docs.md) | Cleanup and documentation | Pending | rust-indexer-engineer |

## Phases

### Phase 1: Core Implementation (1xxx)

Core git polling components that can be developed and tested independently.

- **GITPOLL-1001**: GitState module - state representation and git status parsing
- **GITPOLL-1002**: GitPoller module - polling loop and event emission
- **GITPOLL-1901**: Unit tests for parsing and state diff logic

### Phase 2: Integration (2xxx)

Wire the new components into existing watcher infrastructure.

- **GITPOLL-2001**: Integrate GitPoller into watcher.rs facade
- **GITPOLL-2002**: Update WorktreeWatcher to use GitPoller
- **GITPOLL-2901**: Integration tests with actual git repositories

### Phase 3: Cleanup (3xxx)

Remove old implementation and finalize documentation.

- **GITPOLL-3001**: Remove notify dependency, update documentation

## Dependencies

```
GITPOLL-1001 (GitState)
    └── GITPOLL-1002 (GitPoller)
            └── GITPOLL-1901 (Unit Tests)
                    └── GITPOLL-2001 (Watcher Integration)
                            └── GITPOLL-2002 (WorktreeWatcher)
                                    └── GITPOLL-2901 (Integration Tests)
                                            └── GITPOLL-3001 (Cleanup)
```

## Plan Reference

See [planning/plan.md](../planning/plan.md) for full implementation plan.

---

🎯 Next step: Run `/review-tickets GITPOLL` to validate quality or proceed to `/work-on-project GITPOLL` to execute tickets

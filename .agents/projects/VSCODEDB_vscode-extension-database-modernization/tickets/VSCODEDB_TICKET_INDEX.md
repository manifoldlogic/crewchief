# VSCODEDB Ticket Index

## Project Overview

**Project:** VSCode Extension Database Modernization
**Slug:** VSCODEDB
**Goal:** Add SQLite support to the Maproom VSCode extension as the default, zero-config database backend.

## Ticket Summary

| Ticket ID | Title | Status | Agent | Priority | Est. |
|-----------|-------|--------|-------|----------|------|
| VSCODEDB-1001 | Create database-checker.ts | Not Started | vscode-extension-specialist | MVP | 0.75d |
| VSCODEDB-1002 | Extension Settings Schema | Not Started | vscode-extension-specialist | MVP | 0.25d |
| VSCODEDB-1003 | Docker Optional | Not Started | vscode-extension-specialist | MVP | 0.5d |
| VSCODEDB-1004 | Core Activation Flow | Not Started | vscode-extension-specialist | MVP | 0.5d |
| VSCODEDB-1005 | Documentation | Not Started | vscode-extension-specialist | MVP | 0.5d |
| VSCODEDB-1006 | Setup Wizard Enhancement | Not Started | vscode-extension-specialist | Enhancement | 0.5d |

## Phase Organization

### Phase 1: Database Abstraction (Day 1)

Foundation tickets that create the database abstraction layer.

| Ticket | Title | Blocks | Plan Reference |
|--------|-------|--------|----------------|
| [VSCODEDB-1001](VSCODEDB-1001_database-checker.md) | Create database-checker.ts | 1002, 1003, 1004, 1005, 1006 | plan.md Phase 1 |
| [VSCODEDB-1002](VSCODEDB-1002_extension-settings-schema.md) | Extension Settings Schema | 1004 | plan.md Phase 1 |

### Phase 2: Conditional Activation (Day 2)

Tickets that implement conditional Docker/SQLite activation logic.

| Ticket | Title | Blocks | Plan Reference |
|--------|-------|--------|----------------|
| [VSCODEDB-1003](VSCODEDB-1003_docker-optional.md) | Docker Optional | 1004 | plan.md Phase 2 |
| [VSCODEDB-1004](VSCODEDB-1004_core-activation-flow.md) | Core Activation Flow | 1005 | plan.md Phase 2 |

### Phase 3: Documentation (Day 3-4)

Documentation updates and polish.

| Ticket | Title | Blocks | Plan Reference |
|--------|-------|--------|----------------|
| [VSCODEDB-1005](VSCODEDB-1005_documentation.md) | Documentation | (none) | plan.md Phase 3 |

### Enhancement Phase (Post-MVP)

Optional enhancements for improved UX.

| Ticket | Title | Blocks | Plan Reference |
|--------|-------|--------|----------------|
| [VSCODEDB-1006](VSCODEDB-1006_setup-wizard-enhancement.md) | Setup Wizard Enhancement | (none) | plan.md Enhancement |

## Dependency Graph

```
VSCODEDB-1001 (database-checker.ts)
    │
    ├──────────────────┐
    ▼                  ▼
VSCODEDB-1002      VSCODEDB-1003
(Settings Schema)  (Docker Optional)
    │                  │
    └────────┬─────────┘
             ▼
      VSCODEDB-1004
      (Core Activation)
             │
    ┌────────┴────────┐
    ▼                 ▼
VSCODEDB-1005    VSCODEDB-1006 (optional)
(Documentation)  (Setup Wizard Enhancement)
```

## Execution Order

### Recommended Sequence

1. **VSCODEDB-1001** - Foundation, blocks everything
2. **VSCODEDB-1002** + **VSCODEDB-1003** - Can run in parallel after 1001
3. **VSCODEDB-1004** - After 1002 and 1003 complete
4. **VSCODEDB-1005** - After all implementation complete
5. **VSCODEDB-1006** - Optional, after MVP verification

### Critical Path

```
1001 → [1002 + 1003] → 1004 → 1005
```

Minimum completion time: ~2.5 days

## Success Criteria

### MVP (Required) - Tickets 1001-1005

- [ ] Extension activates without Docker for SQLite mode
- [ ] `~/.maproom/maproom.db` auto-detected
- [ ] Search commands work with SQLite backend
- [ ] Existing PostgreSQL functionality unchanged
- [ ] All tests pass

### Quality (Desired)

- [ ] Activation time <500ms for SQLite mode
- [ ] Clear error messages guide users
- [ ] Documentation shows SQLite as default
- [ ] No new npm dependencies

## Files Overview

### New Files (Created)
- `src/services/database-checker.ts` - Unified database checker
- `src/services/database-checker.test.ts` - Unit tests

### Modified Files
- `src/extension.ts` - Conditional activation, status bar
- `src/ui/setupWizard.ts` - SQLite path selection (enhancement)
- `src/services/postgres-checker.ts` - Deprecation comment
- `package.json` - New `sqlitePath` setting
- `README.md` - Documentation updates

## Agent Assignments

| Ticket | Primary Agent | Backup Agent |
|--------|--------------|--------------|
| VSCODEDB-1001 | vscode-extension-specialist | general-purpose |
| VSCODEDB-1002 | vscode-extension-specialist | general-purpose |
| VSCODEDB-1003 | vscode-extension-specialist | process-management-specialist |
| VSCODEDB-1004 | vscode-extension-specialist | process-management-specialist |
| VSCODEDB-1005 | vscode-extension-specialist | general-purpose |
| VSCODEDB-1006 | vscode-extension-specialist | general-purpose |

## Testing Milestones

| After Ticket | Test | Command |
|--------------|------|---------|
| 1001 | Unit tests | `pnpm test -- src/services/database-checker.test.ts` |
| 1002 | Schema validation | `pnpm vsce:package --no-dependencies` |
| 1004 | All tests | `pnpm test` |
| 1005 | Manual smoke test | See quality-strategy.md |

## Related Documents

- [analysis.md](../planning/analysis.md) - Problem definition
- [architecture.md](../planning/architecture.md) - Solution design
- [plan.md](../planning/plan.md) - Execution plan
- [quality-strategy.md](../planning/quality-strategy.md) - Test strategy
- [project-review.md](../planning/project-review.md) - Project review

---

**Created:** 2025-11-26
**Total Tickets:** 6 (5 MVP + 1 Enhancement)
**Estimated Duration:** 3-4 days

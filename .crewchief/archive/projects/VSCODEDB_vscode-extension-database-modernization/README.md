# VSCODEDB - VSCode Extension Database Modernization

**Status**: ✅ Complete (Archived 2025-11-26)

## Project Summary

This project adds SQLite support to the Maproom VSCode extension as the default, zero-config database backend. Users will be able to use semantic code search immediately after installation without requiring Docker or PostgreSQL.

## Problem Statement

The Maproom VSCode extension currently requires PostgreSQL via Docker as a hard dependency. This creates friction for new users:

- **Setup complexity**: Docker Desktop, container management, and database configuration required
- **Resource overhead**: PostgreSQL container consumes ~100MB RAM even for light usage
- **Platform limitations**: Some environments restrict Docker access
- **Competitive gap**: Modern tools (Cursor, Continue) work immediately after installation

The Rust indexer and MCP server already support SQLite. The extension is the final component needing modernization.

## Proposed Solution

Transform the extension to support both database backends with SQLite as the sensible default:

1. **Create `database-checker.ts`**: Unified abstraction supporting both SQLite (file existence) and PostgreSQL (TCP check)
2. **Make Docker optional**: Only start containers when PostgreSQL mode explicitly configured
3. **SQLite-first activation**: Auto-detect `~/.maproom/maproom.db` and skip Docker entirely
4. **Updated settings**: Add `sqlitePath` setting while preserving all PostgreSQL options

### User Experience

**New User (SQLite - Default)**:
```
1. Install extension from marketplace
2. Run: crewchief-maproom scan /path/to/your/repo
3. Extension auto-detects ~/.maproom/maproom.db
4. Search commands work immediately
```

**Power User (PostgreSQL)**:
```
1. Install extension
2. Change setting: maproom.database.provider → postgres
3. Start Docker containers (manual or via extension)
4. Full feature set with team-sharing capabilities
```

## Relevant Agents

| Agent | Role |
|-------|------|
| **vscode-extension-specialist** | Primary implementation (all tickets) |
| **process-management-specialist** | Backup for orchestrator changes |
| **general-purpose** | Backup for documentation |

## Planning Documents

| Document | Description |
|----------|-------------|
| [analysis.md](planning/analysis.md) | Problem definition, current state, research findings |
| [architecture.md](planning/architecture.md) | Solution design, component interfaces, data flow |
| [quality-strategy.md](planning/quality-strategy.md) | Test approach, critical paths, coverage |
| [security-review.md](planning/security-review.md) | Security analysis, risk assessment, mitigations |
| [plan.md](planning/plan.md) | Execution phases, timeline, success criteria |

## Tickets

### MVP Tickets

| Ticket | Title | Status | Agent | Est. |
|--------|-------|--------|-------|------|
| VSCODEDB-1001 | Create database-checker.ts | Not Started | vscode-extension-specialist | 0.75d |
| VSCODEDB-1002 | Update Extension Settings Schema | Not Started | vscode-extension-specialist | 0.25d |
| VSCODEDB-1003 | Make Docker Containers Optional | Not Started | vscode-extension-specialist | 0.5d |
| VSCODEDB-1004 | Update Core Activation Flow | Not Started | vscode-extension-specialist | 0.5d |
| VSCODEDB-1005 | Update Documentation | Not Started | vscode-extension-specialist | 0.5d |

### Enhancement Tickets

| Ticket | Title | Status | Agent | Est. |
|--------|-------|--------|-------|------|
| VSCODEDB-1006 | Setup Wizard SQLite Path Selection | Not Started | vscode-extension-specialist | 0.5d |

## Dependencies

**External (All Complete)**:
- ✅ MCPDB - MCP server SQLite support (complete)
- ✅ Daemon SQLite support - Verified via direct testing (see analysis.md)

**Internal**:
- VSCODEDB-1001 blocks all other tickets
- VSCODEDB-1002, 1003 can run in parallel after 1001
- VSCODEDB-1004 depends on 1002 and 1003
- VSCODEDB-1005 depends on all implementation tickets

## Success Criteria

### MVP (Required)

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

## Key Files

### Modified

- `packages/vscode-maproom/src/extension.ts` - Conditional activation
- `packages/vscode-maproom/src/services/postgres-checker.ts` - Deprecate
- `packages/vscode-maproom/src/process/orchestrator.ts` - Database URL config
- `packages/vscode-maproom/src/ui/setupWizard.ts` - SQLite path selection
- `packages/vscode-maproom/package.json` - New settings

### Created

- `packages/vscode-maproom/src/services/database-checker.ts` - Unified checker
- `packages/vscode-maproom/src/services/database-checker.test.ts` - Tests

## Timeline

| Day | Phase | Deliverables |
|-----|-------|--------------|
| 1 | Database Abstraction | database-checker.ts, settings schema |
| 2 | Conditional Activation | Docker optional, activation flow |
| 3-4 | Documentation + Testing | README, smoke tests, polish |

---

**Created**: 2025-11-26
**Source**: `.crewchief/reports/2025-11-26_sqlite-integration-project-decomposition.md`
**Project Slug**: VSCODEDB

# VSCODEDB - Execution Plan

## Overview

This plan outlines the implementation phases for adding SQLite support to the VSCode extension as the default, zero-config database backend.

**Total Estimated Effort**: 3-4 days
**Total Tickets**: 5 MVP + 1 Enhancement
**Primary Agent**: vscode-extension-specialist

## Dependencies

### External Dependencies (Must Be Complete)

| Dependency | Status | Notes |
|------------|--------|-------|
| MCPDB Project | ✅ Complete | MCP server SQLite support |
| Daemon SQLite Support | ✅ Verified | See analysis.md - daemon accepts `MAPROOM_DATABASE_URL=sqlite://...` |

**Note**: VECSTORE and MAPCLI projects were originally listed as dependencies but are NOT required. Direct testing confirmed the daemon already supports SQLite via environment variable (see analysis.md "Daemon SQLite Support Verification" section).

### Internal Dependencies

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

**Note**: VSCODEDB-1006 (Setup Wizard SQLite Path Selection) is an enhancement ticket, not MVP. The core SQLite functionality works without it.

## Phase 1: Database Abstraction (Day 1)

### VSCODEDB-1001: Create database-checker.ts

**Agent**: vscode-extension-specialist
**Estimate**: 0.75 days

**Objective**: Create unified database checker supporting both SQLite and PostgreSQL.

**Deliverables**:
- `src/services/database-checker.ts` with:
  - `DatabaseConfig` interface
  - `resolveDatabaseConfig()` function
  - `checkDatabaseAvailable()` function
  - `getDatabaseUrl()` function
  - `getDatabaseUnavailableMessage()` function
- `src/services/database-checker.test.ts` with unit tests
- Deprecation comment on `postgres-checker.ts`

**Acceptance Criteria**:
- [ ] `resolveDatabaseConfig()` returns SQLite config when `provider=sqlite`
- [ ] `resolveDatabaseConfig()` returns PostgreSQL config when `provider=postgres`
- [ ] `checkDatabaseAvailable()` uses `existsSync` for SQLite
- [ ] `checkDatabaseAvailable()` uses TCP check for PostgreSQL
- [ ] All unit tests pass

**Files Modified**:
- `packages/vscode-maproom/src/services/database-checker.ts` (new)
- `packages/vscode-maproom/src/services/database-checker.test.ts` (new)
- `packages/vscode-maproom/src/services/postgres-checker.ts` (deprecation comment)

### VSCODEDB-1002: Update Extension Settings Schema

**Agent**: vscode-extension-specialist
**Estimate**: 0.25 days

**Objective**: Add `sqlitePath` setting and update documentation strings.

**Deliverables**:
- `maproom.database.sqlitePath` setting in `package.json`
- Updated descriptions for existing settings
- Clear distinction between SQLite and PostgreSQL modes

**Acceptance Criteria**:
- [ ] New setting appears in VSCode settings UI
- [ ] Default value works correctly (empty string → ~/.maproom/maproom.db)
- [ ] Setting only affects SQLite mode (ignored for PostgreSQL)

**Files Modified**:
- `packages/vscode-maproom/package.json`

## Phase 2: Conditional Activation (Day 2)

### VSCODEDB-1003: Make Docker Containers Optional

**Agent**: vscode-extension-specialist
**Estimate**: 0.5 days

**Objective**: Only start Docker services when PostgreSQL mode is configured.

**Deliverables**:
- Updated `initializeServices()` with database type branching
- Updated `runFirstTimeSetup()` to skip Docker for SQLite
- New `ensureSqliteAvailable()` function
- Integration tests verifying Docker skip behavior

**Acceptance Criteria**:
- [ ] Docker not started when `provider=sqlite`
- [ ] Docker started normally when `provider=postgres`
- [ ] `ensureSqliteAvailable()` validates file exists
- [ ] Error message shown when SQLite file missing

**Files Modified**:
- `packages/vscode-maproom/src/extension.ts`

### VSCODEDB-1004: Update Core Activation Flow for SQLite

**Agent**: vscode-extension-specialist
**Estimate**: 0.5 days

**Objective**: Complete core activation flow changes for SQLite mode.

**Deliverables**:
- Updated `ProcessOrchestrator` configuration to use `databaseUrlOverride`
- Updated status bar to show database mode
- Auto-detection of existing `~/.maproom/maproom.db`

**Acceptance Criteria**:
- [ ] Extension activates without Docker for SQLite mode
- [ ] ProcessOrchestrator receives correct database URL via `databaseUrlOverride`
- [ ] Status bar indicates "SQLite" or "PostgreSQL" mode (see architecture.md UI spec)
- [ ] Activation time <500ms for SQLite mode

**Files Modified**:
- `packages/vscode-maproom/src/extension.ts`

**Note**: Setup wizard enhancements moved to VSCODEDB-1006 (enhancement ticket).

## Phase 3: Documentation (Day 3-4)

### VSCODEDB-1005: Update Extension Documentation

**Agent**: vscode-extension-specialist
**Estimate**: 0.5 days

**Objective**: Update all documentation to reflect SQLite-first default.

**Deliverables**:
- Updated README.md with SQLite getting started
- PostgreSQL documented as "advanced" option
- Troubleshooting guide for common SQLite issues
- Migration guide from PostgreSQL to SQLite

**Acceptance Criteria**:
- [ ] README shows SQLite as default getting started path
- [ ] PostgreSQL setup clearly marked as optional/advanced
- [ ] Common error messages documented with solutions
- [ ] Settings reference table updated

**Files Modified**:
- `packages/vscode-maproom/README.md`
- `packages/vscode-maproom/CLAUDE.md` (if exists)

## Enhancement Phase (Post-MVP)

### VSCODEDB-1006: Setup Wizard SQLite Path Selection (Enhancement)

**Agent**: vscode-extension-specialist
**Estimate**: 0.5 days
**Priority**: Optional/Enhancement

**Objective**: Enhance setup wizard with SQLite file selection and better guidance.

**Deliverables**:
- File picker for selecting existing SQLite database
- Auto-detection of `~/.maproom/maproom.db` with "Use existing" option
- Improved guidance for first-time users

**Acceptance Criteria**:
- [ ] Setup wizard detects existing `~/.maproom/maproom.db` and offers "Use existing index?"
- [ ] File picker allows selecting alternative SQLite paths
- [ ] Clear guidance on running `crewchief-maproom scan` if no database exists

**Files Modified**:
- `packages/vscode-maproom/src/ui/setupWizard.ts`

**Note**: This is an enhancement ticket. The MVP SQLite functionality works without it - users can manually configure `maproom.database.sqlitePath` in settings.

## Testing Milestones

### Milestone 1: Database Checker Tests (After VSCODEDB-1001)

```bash
cd packages/vscode-maproom
pnpm test -- src/services/database-checker.test.ts
```

**Expected**: All unit tests pass

### Milestone 2: Extension Build (After VSCODEDB-1002)

```bash
cd packages/vscode-maproom
pnpm compile
pnpm vsce:package
```

**Expected**: VSIX builds without errors

### Milestone 3: Integration Tests (After VSCODEDB-1004)

```bash
cd packages/vscode-maproom
pnpm test
```

**Expected**: All tests pass, including new integration tests

### Milestone 4: Manual Smoke Test (After All Tickets)

1. Install VSIX in fresh VSCode instance
2. Verify extension activates with existing `~/.maproom/maproom.db`
3. Verify search commands work
4. Verify PostgreSQL mode still works

## Security Checkpoints

### Checkpoint 1: Path Handling (VSCODEDB-1001)

- [ ] Verify path expansion doesn't escape home directory
- [ ] Verify absolute path resolution
- [ ] Review error message content for sensitive data

### Checkpoint 2: Settings Validation (VSCODEDB-1002)

- [ ] Verify settings schema rejects invalid types
- [ ] Verify defaults are sensible

## Agent Assignments

| Ticket | Primary Agent | Backup Agent | Notes |
|--------|--------------|--------------|-------|
| VSCODEDB-1001 | vscode-extension-specialist | general-purpose | MVP |
| VSCODEDB-1002 | vscode-extension-specialist | general-purpose | MVP |
| VSCODEDB-1003 | vscode-extension-specialist | process-management-specialist | MVP |
| VSCODEDB-1004 | vscode-extension-specialist | process-management-specialist | MVP |
| VSCODEDB-1005 | vscode-extension-specialist | general-purpose | MVP |
| VSCODEDB-1006 | vscode-extension-specialist | general-purpose | Enhancement (post-MVP) |

## Rollback Plan

If issues are discovered post-implementation:

1. **Settings Schema**: Remove `sqlitePath` setting, revert to postgres-only
2. **Extension Code**: Revert `extension.ts` changes via git
3. **User Communication**: Document that SQLite mode is disabled pending fixes

## Success Criteria

### MVP Success (All Required)

- [ ] Extension activates without Docker when SQLite mode
- [ ] `~/.maproom/maproom.db` auto-detected
- [ ] Search commands work with SQLite backend
- [ ] Existing PostgreSQL functionality unchanged
- [ ] All tests pass

### Quality Success (Highly Desired)

- [ ] Activation time <500ms for SQLite mode
- [ ] Clear error messages guide users to fix issues
- [ ] Documentation shows SQLite as default
- [ ] No new npm dependencies added

## Timeline

| Day | Tickets | Milestone |
|-----|---------|-----------|
| 1 | VSCODEDB-1001, VSCODEDB-1002 | Database abstraction complete |
| 2 | VSCODEDB-1003, VSCODEDB-1004 | Conditional activation complete |
| 3 | VSCODEDB-1005 | Documentation complete |
| 4 | Buffer/Testing | Final testing and polish |

## Post-Implementation

After all tickets complete:

1. **CI Verification**: Ensure all CI tests pass
2. **VSIX Publishing**: Build and test VSIX package
3. **Project Archive**: Move to `.crewchief/archive/projects/`
4. **Knowledge Transfer**: Update `/docs/` with permanent documentation

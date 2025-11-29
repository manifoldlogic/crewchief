# SQLINFRA Execution Plan

## Overview

This plan outlines the execution phases for updating infrastructure to present SQLite as the default database backend. All work is documentation and CI/CD configuration - no application code changes.

## Phase Summary

| Phase | Focus | Tickets | Agent |
|-------|-------|---------|-------|
| 1 | CI Workflow Updates | 2 | github-actions-specialist |
| 2 | Core Documentation | 2 | general-purpose |
| 3 | Docker Documentation | 1 | general-purpose |

**Total**: 5 tickets

## Phase 1: CI Workflow Updates

**Goal**: Reorganize CI to present SQLite as primary, PostgreSQL as optional integration testing.

### Ticket SQLINFRA-1001: Rename and Reorganize CI Jobs

**Description**: Update `.github/workflows/test.yml` to clearly distinguish SQLite-primary and PostgreSQL-integration jobs.

**Changes**:
- Rename `test` job to `test-postgres` with clear "PostgreSQL Integration" label
- Rename `test-rust` to show clear backend names in matrix
- Group SQLite jobs together at top of workflow
- Add comments explaining job purposes

**Acceptance Criteria**:
- [ ] All existing tests pass
- [ ] Job names clearly indicate backend
- [ ] SQLite jobs appear first in workflow visualization
- [ ] PostgreSQL job labeled as "integration"

**Agent**: github-actions-specialist

### Ticket SQLINFRA-1002: Add CI Summary and Documentation

**Description**: Add workflow summary annotations and update CI documentation.

**Changes**:
- Add job annotations for GitHub Actions summary
- Update `.github/CLAUDE.md` with SQLite-first context
- Add comment block at top of test.yml explaining architecture

**Acceptance Criteria**:
- [ ] Workflow run summary shows clear backend organization
- [ ] `.github/CLAUDE.md` reflects SQLite-first CI
- [ ] Developers understand CI structure from comments

**Agent**: github-actions-specialist

## Phase 2: Core Documentation

**Goal**: Update main documentation to show SQLite as default path.

### Ticket SQLINFRA-1003: Update README Quick Start

**Description**: Rewrite README.md to present SQLite as the default, zero-configuration option.

**Changes**:
- Rewrite Quick Start section for SQLite (no Docker)
- Move PostgreSQL setup to "Advanced: PostgreSQL (Team Sharing)" section
- Update Requirements section to show SQLite as default
- Add brief SQLite benefits description
- Link to PostgreSQL docs for advanced users

**Acceptance Criteria**:
- [ ] Quick Start works without Docker/PostgreSQL
- [ ] All Quick Start commands execute successfully
- [ ] PostgreSQL path still documented (in advanced section)
- [ ] Requirements clearly show SQLite default

**Agent**: general-purpose

### Ticket SQLINFRA-1004: Update Database Architecture Documentation

**Description**: Add SQLite section to DATABASE_ARCHITECTURE.md.

**Changes**:
- Add "Database Backend Options" section after Overview
- Include comparison table (SQLite vs PostgreSQL)
- Add SQLite architecture details
- Add SQLite troubleshooting section
- Preserve all existing PostgreSQL documentation

**Acceptance Criteria**:
- [ ] SQLite section appears prominently
- [ ] Comparison table helps users choose
- [ ] SQLite troubleshooting available
- [ ] PostgreSQL docs unchanged

**Agent**: general-purpose

## Phase 3: Docker Documentation

**Goal**: Clarify when Docker/PostgreSQL is needed.

### Ticket SQLINFRA-1005: Update Docker Compose Documentation

**Description**: Add header comments to Docker compose files explaining use cases.

**Changes**:
- `config/docker-compose.yml`: Add header comment explaining PostgreSQL is for team sharing
- `packages/vscode-maproom/config/docker-compose.yml`: Add comment linking to SQLite option
- Update any Docker-related documentation sections

**Acceptance Criteria**:
- [ ] Docker compose files have explanatory comments
- [ ] Comments link to SQLite documentation
- [ ] Users understand when Docker is needed
- [ ] No functional changes to Docker configs

**Agent**: general-purpose

## Execution Order

```
Phase 1: CI (Foundation)
├── SQLINFRA-1001: Rename CI jobs
└── SQLINFRA-1002: CI documentation

Phase 2: Core Docs (Highest visibility)
├── SQLINFRA-1003: README update
└── SQLINFRA-1004: Architecture docs

Phase 3: Docker (Cleanup)
└── SQLINFRA-1005: Docker comments
```

**Rationale**:
1. CI first establishes SQLite-first in the codebase infrastructure
2. README second because it's highest visibility
3. Docker docs last as lower priority cleanup

## Agent Assignments

| Ticket | Primary Agent | Backup Agent |
|--------|---------------|--------------|
| SQLINFRA-1001 | github-actions-specialist | general-purpose |
| SQLINFRA-1002 | github-actions-specialist | general-purpose |
| SQLINFRA-1003 | general-purpose | - |
| SQLINFRA-1004 | general-purpose | - |
| SQLINFRA-1005 | general-purpose | - |

## Dependencies

### External Dependencies

| Dependency | Status | Required For |
|------------|--------|--------------|
| VECSTORE | Complete | All tickets |
| MAPCLI | Complete | All tickets |
| MCPDB | Complete | All tickets |
| VSCODEDB | Complete | All tickets |

### Internal Dependencies

| Ticket | Depends On | Notes |
|--------|------------|-------|
| SQLINFRA-1002 | SQLINFRA-1001 | CI docs follow structure |
| SQLINFRA-1003 | None | Can start immediately |
| SQLINFRA-1004 | None | Can start immediately |
| SQLINFRA-1005 | SQLINFRA-1003 | Reference README patterns |

**Parallel Execution**:
- Phase 1 tickets sequential (1001 → 1002)
- Phase 2 tickets parallel (1003 || 1004)
- Phase 3 after Phase 2

## Success Criteria

### Phase 1 Complete When

- [ ] CI workflow jobs renamed appropriately
- [ ] All tests pass on PR
- [ ] GitHub Actions summary shows clear backend organization

### Phase 2 Complete When

- [ ] README Quick Start uses SQLite
- [ ] DATABASE_ARCHITECTURE.md has SQLite section
- [ ] New user can search code without Docker

### Phase 3 Complete When

- [ ] Docker compose files have explanatory comments
- [ ] All documentation links work
- [ ] Project complete

## Verification

Each ticket follows the standard workflow:
1. Implementation by assigned agent
2. Unit test verification (N/A for docs - manual smoke test)
3. verify-ticket agent checks acceptance criteria
4. commit-ticket agent creates commit

### Smoke Test Protocol

After all tickets complete:

```bash
# 1. Clean environment test
rm -rf ~/.maproom/
crewchief maproom:scan /path/to/small/repo
crewchief maproom:search "function"

# 2. Verify CI passes
# (Observe GitHub Actions on PR)

# 3. Link validation
# (Manual review of documentation links)
```

## Risk Mitigation

| Risk | Probability | Mitigation |
|------|-------------|------------|
| CI workflow breaks | Low | Test on PR before merge |
| Documentation unclear | Medium | Peer review |
| Missing PostgreSQL users | Low | Preserve PostgreSQL docs |

## Timeline

Estimated effort: 2-3 days

| Day | Phase | Tickets |
|-----|-------|---------|
| 1 | Phase 1 | SQLINFRA-1001, SQLINFRA-1002 |
| 2 | Phase 2 | SQLINFRA-1003, SQLINFRA-1004 |
| 2-3 | Phase 3 | SQLINFRA-1005, verification |

## Post-Completion

After all tickets complete:

1. **Archive Project**: Move to `.crewchief/archive/projects/`
2. **Update INDEX**: Reflect SQLite-first in project indexes
3. **Monitor**: Watch for user feedback on documentation

## Notes

- This is the **final project** in the SQLite integration sequence
- All application code changes already complete (VECSTORE, MAPCLI, MCPDB, VSCODEDB)
- Focus is entirely on infrastructure messaging and documentation
- PostgreSQL functionality preserved - just de-emphasized

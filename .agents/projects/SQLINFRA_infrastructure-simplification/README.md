# SQLINFRA - Infrastructure Simplification

## Project Summary

Update CI/CD workflows and documentation to present SQLite as the default, zero-configuration database backend for Maproom semantic search. This is the final project in the SQLite integration sequence, focusing on infrastructure messaging rather than code changes.

## Problem Statement

The SQLite backend has been fully implemented (VECSTORE, MAPCLI, MCPDB, VSCODEDB), but infrastructure and documentation still default to PostgreSQL:

- **CI/CD** runs PostgreSQL tests as primary, requiring Docker service containers
- **README** Quick Start requires Docker/PostgreSQL setup
- **Documentation** emphasizes PostgreSQL as the only option
- **Docker compose** files lack context about when PostgreSQL is needed

Users can achieve full semantic search with zero dependencies using SQLite, but this path isn't visible.

## Proposed Solution

### Phase 1: CI Workflow Updates
- Rename jobs to clearly distinguish SQLite (primary) from PostgreSQL (integration)
- Add CI documentation explaining the backend organization

### Phase 2: Core Documentation
- Rewrite README Quick Start for SQLite (no Docker required)
- Add SQLite section to DATABASE_ARCHITECTURE.md
- Move PostgreSQL to "Advanced Setup" documentation

### Phase 3: Docker Documentation
- Add explanatory comments to docker-compose files
- Link to SQLite as the default alternative

## Relevant Agents

| Agent | Role |
|-------|------|
| **github-actions-specialist** | CI workflow updates (Phase 1) |
| **general-purpose** | Documentation updates (Phase 2-3) |
| **verify-ticket** | Acceptance criteria verification |
| **commit-ticket** | Conventional commit creation |

## Planning Documents

| Document | Description |
|----------|-------------|
| [analysis.md](planning/analysis.md) | Problem definition, current state, research findings |
| [architecture.md](planning/architecture.md) | Solution design, ADRs, component structure |
| [quality-strategy.md](planning/quality-strategy.md) | Test approach, verification methods |
| [security-review.md](planning/security-review.md) | Security assessment (LOW risk - docs only) |
| [plan.md](planning/plan.md) | Phased execution plan with tickets |

## Tickets

| Ticket | Title | Phase | Agent |
|--------|-------|-------|-------|
| SQLINFRA-1001 | Rename and Reorganize CI Jobs | 1 | github-actions-specialist |
| SQLINFRA-1002 | Add CI Summary and Documentation | 1 | github-actions-specialist |
| SQLINFRA-1003 | Update README Quick Start | 2 | general-purpose |
| SQLINFRA-1004 | Update Database Architecture Documentation | 2 | general-purpose |
| SQLINFRA-1005 | Update Docker Compose Documentation | 3 | general-purpose |

## Dependencies

### Prerequisites (All Complete)

| Project | Status | Description |
|---------|--------|-------------|
| VECSTORE | Complete | VectorStore trait with SQLite implementation |
| MAPCLI | Complete | CLI and daemon SQLite support |
| MCPDB | Complete | MCP server SQLite URLs |
| VSCODEDB | Complete | VSCode extension SQLite-first |

### External Dependencies

None - this project modifies only configuration and documentation.

## Success Criteria

### MVP (Required)

- [ ] CI jobs clearly labeled as SQLite (primary) and PostgreSQL (integration)
- [ ] README Quick Start works without Docker/PostgreSQL
- [ ] DATABASE_ARCHITECTURE.md includes SQLite section
- [ ] All existing tests continue to pass

### Quality (Desired)

- [ ] New user can search code within 5 minutes of install
- [ ] PostgreSQL path still documented and working
- [ ] No broken documentation links
- [ ] Clear visual hierarchy (SQLite default, PostgreSQL advanced)

## Quick Reference

### Key Files Modified

```
.github/workflows/test.yml           # CI job reorganization
.github/CLAUDE.md                    # CI documentation
README.md                            # Quick Start rewrite
docs/architecture/DATABASE_ARCHITECTURE.md  # SQLite section
config/docker-compose.yml            # Explanatory comments
packages/vscode-maproom/config/docker-compose.yml  # Comments
```

### Verification Commands

```bash
# Test SQLite Quick Start path
rm -rf ~/.maproom/
crewchief maproom:scan /path/to/repo
crewchief maproom:search "function"

# Verify PostgreSQL path still works
cd config && docker compose up -d
export MAPROOM_DATABASE_URL="postgresql://maproom:maproom@localhost:5433/maproom"
crewchief maproom:search "function"
```

## Timeline

**Estimated**: 2-3 days

| Day | Phase | Deliverables |
|-----|-------|--------------|
| 1 | Phase 1 | CI workflow reorganized |
| 2 | Phase 2 | README and architecture docs updated |
| 2-3 | Phase 3 | Docker docs, verification, completion |

## Notes

- This is a **documentation-only** project - no application code changes
- All SQLite functionality already works (implemented in prior projects)
- PostgreSQL remains fully supported, just de-emphasized
- Security risk is LOW (see security-review.md)

---

**Created**: 2025-11-26
**Source**: `.agents/reports/2025-11-26_sqlite-integration-project-decomposition.md`
**Project Slug**: SQLINFRA

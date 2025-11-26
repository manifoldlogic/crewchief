# SQLINFRA Analysis: Infrastructure Simplification

## Problem Definition

The SQLite backend has been fully implemented across the codebase (VECSTORE, MAPCLI, MCPDB, VSCODEDB projects), but the infrastructure and documentation still default to PostgreSQL. This creates a disjointed experience where:

1. **CI/CD workflows** run PostgreSQL tests by default, requiring Docker service containers
2. **Documentation** emphasizes PostgreSQL setup as the primary path
3. **Docker compose files** focus on PostgreSQL infrastructure
4. **Getting started** guides send users through Docker/PostgreSQL setup unnecessarily

Users can now achieve full semantic search functionality with zero external dependencies using SQLite, but the infrastructure doesn't reflect this reality.

## Context

### Dependency Chain Position

This project is the **final project** in the SQLite integration sequence:

```
VECSTORE (Foundation) → MAPCLI (CLI/Daemon) → MCPDB & VSCODEDB (Parallel) → SQLINFRA (Cleanup)
```

**Prerequisites (all complete)**:
- **VECSTORE**: VectorStore trait with SQLite implementation
- **MAPCLI**: CLI and daemon work with SQLite
- **MCPDB**: MCP server supports SQLite URLs
- **VSCODEDB**: VSCode extension supports SQLite-first activation

### Pattern Classification

This project follows the **Capability Layer** pattern:
- Horizontal infrastructure changes across CI/CD, Docker, and documentation
- No code changes to application logic
- Configuration and documentation focus
- Infrastructure cleanup after code complete

## Current State Analysis

### CI/CD Workflows (`.github/workflows/`)

**Current Test Workflow** (`test.yml`):
```yaml
# Jobs:
- test                 # PostgreSQL service container required
- test-rust            # Matrix: sqlite, postgres (no service needed for SQLite)
- test-sqlite-e2e      # SQLite CLI tests (no service)
- test-mcp-sqlite      # SQLite MCP tests (no service)
```

**Observations**:
1. **PostgreSQL tests dominate** - The main `test` job requires PostgreSQL service container
2. **SQLite jobs exist** - `test-rust`, `test-sqlite-e2e`, and `test-mcp-sqlite` run without PostgreSQL
3. **No ordering/gating** - SQLite tests don't block PostgreSQL tests or vice versa
4. **Mixed messaging** - PostgreSQL still appears as the "primary" backend

**Missing**:
- Clear SQLite-first CI presentation
- SQLite test results highlighted in workflow summary
- Optional PostgreSQL job (not required for basic PRs)

### Docker Compose Files

**Standalone** (`config/docker-compose.yml`):
- PostgreSQL with pgvector
- Ollama commented out
- External volume for init.sql
- No mention of SQLite alternative

**VSCode Extension** (`packages/vscode-maproom/config/docker-compose.yml`):
- Similar PostgreSQL setup
- Has `docker-compose.test.yml` for test isolation
- VSCODEDB project made this optional but documentation unclear

**DevContainer** (`.devcontainer/docker-compose.yml`):
- PostgreSQL required for devcontainer
- May still be appropriate (development environment)

### Documentation State

**Main README.md**:
```markdown
## Requirements
- PostgreSQL (for Maproom)
```

Issues:
- Lists PostgreSQL as requirement, not SQLite
- Quick Start shows `PG_DATABASE_URL`
- No mention of SQLite zero-config option
- Embedding section mentions Ollama but not SQLite backend

**Database Architecture** (`docs/architecture/DATABASE_ARCHITECTURE.md`):
- 467 lines focused entirely on PostgreSQL
- Detailed troubleshooting for Docker/PostgreSQL
- Schema documentation valuable but PostgreSQL-centric
- No SQLite architecture section

**SQLite Integration Tests** (`docs/testing/SQLITE_INTEGRATION_TESTS.md`):
- Good SQLite test documentation exists
- Shows SQLite is well-tested
- Hidden in `/docs/testing/` - not prominent

**Provider Docs** (`docs/providers/`):
- Ollama, OpenAI, Google Vertex AI documented
- Focus on embedding providers, not database backends
- Could add SQLite section or link

### Existing Solutions / Industry Patterns

**SQLite-First Tools**:
- **Cursor** - Local-first with optional cloud sync
- **Obsidian** - SQLite for local vault indexing
- **DuckDB** - Analytics tool, zero-config default
- **LanceDB** - Vector DB, file-based default

**PostgreSQL-Required Tools**:
- **PostHog** - Self-hosted analytics (PostgreSQL required)
- **Supabase** - PostgreSQL-as-a-platform

**Hybrid Approaches**:
- **PocketBase** - SQLite default, PostgreSQL optional
- **Directus** - SQLite for quick start, PostgreSQL for production

**Industry Consensus**: Developer tools should work out-of-the-box with minimal configuration. SQLite provides this for single-user workflows.

## Research Findings

### CI Best Practices

1. **Fast Path First**: SQLite tests run faster (no container startup)
2. **Optional Integration Tests**: PostgreSQL tests can be "matrix expansion" rather than required
3. **Clear Job Naming**: "SQLite (Default)" vs "PostgreSQL (Team/Advanced)"
4. **Status Checks**: Configure branch protection to require SQLite, optional PostgreSQL

### Documentation Patterns

1. **Zero to Hello World**: Time from install to first successful operation
2. **Progressive Disclosure**: Simple default → Advanced options later
3. **Visual Hierarchy**: Quick Start prominent, advanced setup in separate section

### User Segmentation

| User Type | Database Need | Docker Need |
|-----------|---------------|-------------|
| Individual Developer | SQLite | None |
| Small Team (shared index) | PostgreSQL | Optional |
| Enterprise (managed infra) | PostgreSQL | Likely K8s |
| CI/CD Pipeline | SQLite (tests) | None |

## Gap Analysis

### What Needs to Change

| Area | Current State | Target State |
|------|---------------|--------------|
| README Quick Start | PostgreSQL setup | SQLite auto-detect |
| Requirements section | "PostgreSQL required" | "SQLite (default) or PostgreSQL" |
| CI workflow | PostgreSQL primary | SQLite primary |
| Docker docs | Only option shown | "Advanced: Team setup" |
| Database architecture docs | PostgreSQL-only | Dual-backend sections |

### What's Already Good

1. SQLite E2E tests exist and pass in CI
2. MCP SQLite tests run without PostgreSQL
3. VSCode extension already supports SQLite-first
4. Docker compose files work when needed
5. All migration tooling supports both backends

## Scope Definition

### In Scope

1. **CI Workflow Updates**
   - Add SQLite-only test job as the primary/default
   - Make PostgreSQL service container optional
   - Update workflow job naming for clarity

2. **Documentation Updates**
   - Rewrite README Quick Start for SQLite
   - Update requirements section
   - Add SQLite section to DATABASE_ARCHITECTURE.md
   - Update troubleshooting guides

3. **Docker Configuration**
   - Add comments to docker-compose files explaining when PostgreSQL needed
   - Update VSCode extension docker-compose documentation
   - Link to SQLite as default option

4. **Developer Guidance**
   - Update CLAUDE.md with SQLite-first context
   - Update contribution guides if needed

### Out of Scope

1. **Code Changes** - All code complete in prior projects
2. **New Features** - No new functionality
3. **Schema Changes** - Database schemas unchanged
4. **Extension Code** - VSCODEDB already complete

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Documentation becomes stale | Medium | Low | Link docs, avoid duplication |
| CI changes break builds | Low | Medium | Test changes in PR first |
| Users confused by options | Low | Low | Clear visual hierarchy |
| PostgreSQL users feel abandoned | Low | Low | Keep PostgreSQL docs intact |

## Success Criteria

1. **Documentation**
   - [ ] README shows SQLite as default Quick Start
   - [ ] DATABASE_ARCHITECTURE.md includes SQLite section
   - [ ] Clear guidance on when to use PostgreSQL vs SQLite

2. **CI/CD**
   - [ ] SQLite tests labeled as primary/default
   - [ ] PostgreSQL tests run but clearly labeled "integration"
   - [ ] All existing tests still pass

3. **Developer Experience**
   - [ ] New contributor can search code without Docker in 5 minutes
   - [ ] PostgreSQL setup documented as "advanced" option
   - [ ] No broken links or missing documentation

## Recommendations

1. **Start with CI** - Clear CI messaging establishes SQLite-first mindset
2. **Update README Next** - Highest visibility document
3. **Docker Docs Last** - Lower priority, reference existing content
4. **Avoid Duplication** - Link between docs, don't copy content
5. **Keep PostgreSQL Path** - Don't remove, just de-emphasize

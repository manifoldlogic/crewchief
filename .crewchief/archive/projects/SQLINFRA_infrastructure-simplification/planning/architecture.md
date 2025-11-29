# SQLINFRA Architecture: Infrastructure Simplification

## Overview

This document describes the infrastructure changes needed to present SQLite as the default, zero-configuration option for Maproom semantic search. No application code changes are required - this is purely configuration, CI/CD, and documentation work.

## Design Principles

1. **Progressive Disclosure** - Simple default path, advanced options available
2. **No Breaking Changes** - Existing PostgreSQL workflows continue to work
3. **Additive Changes** - New SQLite-first content, existing content preserved
4. **Link Don't Duplicate** - Reference existing docs, avoid content sprawl

## Component Architecture

### CI/CD Workflow Structure

**Current Structure**:
```
test.yml
├── test               # PostgreSQL required (service container)
├── test-rust          # Matrix [sqlite, postgres]
├── test-sqlite-e2e    # SQLite only
└── test-mcp-sqlite    # SQLite only
```

**Target Structure**:
```
test.yml
├── test-sqlite-core        # Primary: All SQLite tests (fast, no deps)
├── test-mcp-sqlite         # MCP TypeScript + SQLite (no deps)
├── test-postgres           # Optional: PostgreSQL integration
└── test-rust-postgres      # Optional: Rust PostgreSQL tests
```

### Documentation Hierarchy

**Current**:
```
README.md
├── Quick Start (PostgreSQL required)
├── Requirements (PostgreSQL listed)
└── Embedding Configuration

docs/
├── architecture/
│   └── DATABASE_ARCHITECTURE.md  # PostgreSQL-only
└── testing/
    └── SQLITE_INTEGRATION_TESTS.md  # Hidden
```

**Target**:
```
README.md
├── Quick Start (SQLite default - zero config)
├── Requirements (SQLite default, PostgreSQL optional)
├── SQLite Usage
└── Advanced: PostgreSQL Setup (link to docs)

docs/
├── architecture/
│   └── DATABASE_ARCHITECTURE.md  # Dual-backend with SQLite section
└── guides/
    └── GETTING_STARTED.md  # New: SQLite-first guide
```

## Design Decisions

### ADR-001: CI Job Organization

**Context**: CI workflow has multiple test jobs with unclear priority.

**Decision**: Group jobs by database backend with SQLite as primary.

**Consequences**:
- SQLite jobs run first/always
- PostgreSQL jobs clearly labeled as "integration"
- PR status shows SQLite health prominently
- Reduces required CI time for basic PRs

### ADR-002: Documentation Strategy

**Context**: DATABASE_ARCHITECTURE.md is comprehensive but PostgreSQL-focused.

**Decision**: Add SQLite section to existing doc rather than creating parallel doc.

**Consequences**:
- Single source of truth for database architecture
- Easier to maintain consistency
- Avoids documentation sprawl
- Preserves existing valuable PostgreSQL content

### ADR-003: README Quick Start

**Context**: README Quick Start assumes Docker/PostgreSQL.

**Decision**: Rewrite Quick Start for SQLite with PostgreSQL as "Advanced Setup".

**Consequences**:
- New users see zero-config path first
- "Time to first search" dramatically reduced
- PostgreSQL users still find their path (in separate section)
- Competitive with modern developer tools

### ADR-004: Docker Compose Comments

**Context**: Docker compose files have no context about when they're needed.

**Decision**: Add header comments explaining use cases, link to SQLite alternative.

**Consequences**:
- Users understand when Docker is needed
- SQLite path discoverable from Docker files
- No functional changes to Docker setup
- Low effort, high clarity

## Technical Specifications

### CI Workflow Changes

**Job Renaming**:
```yaml
# Before
jobs:
  test:           # Ambiguous
  test-rust:      # Ambiguous

# After
jobs:
  test-sqlite:          # Clear: SQLite primary
  test-postgres:        # Clear: PostgreSQL integration
```

**Service Container Updates**:
```yaml
# PostgreSQL job becomes optional
test-postgres:
  name: PostgreSQL Integration Tests
  runs-on: ubuntu-latest
  # Only run on main branch and PRs touching postgres-specific code
  if: github.ref == 'refs/heads/main' || contains(github.event.pull_request.labels.*.name, 'postgres')
  services:
    postgres-test:
      image: pgvector/pgvector:pg16
      # ... existing config
```

### README Structure

```markdown
# CrewChief

## Quick Start (SQLite - Recommended)

Works immediately - no Docker, no database setup required!

\`\`\`bash
# Install
npm install -g @crewchief/cli

# Index your code (creates ~/.maproom/maproom.db automatically)
crewchief maproom:scan /path/to/your/repo

# Search semantically
crewchief maproom:search "authentication flow"
\`\`\`

## Requirements

- Node.js >= 18
- Git
- **Optional**: Docker (for PostgreSQL team sharing)
- **Optional**: iTerm2 (for agent features, macOS)

## Advanced: PostgreSQL Setup (Team Sharing)

For shared team indices or enterprise deployments, see [PostgreSQL Setup Guide](docs/guides/postgresql-setup.md).
```

### DATABASE_ARCHITECTURE.md Updates

Add new section after Overview:

```markdown
## Database Backend Options

CrewChief supports two database backends for semantic search:

| Backend | Use Case | Setup Required | Performance |
|---------|----------|----------------|-------------|
| **SQLite** (Default) | Individual use, CI/CD | None | Fast for single-user |
| **PostgreSQL** | Team sharing, production | Docker/managed | Better for concurrent use |

### SQLite Backend

SQLite is the recommended default for most users:

- **Zero configuration** - Works immediately after install
- **No Docker required** - Single file at `~/.maproom/maproom.db`
- **Full feature parity** - Search, status, MCP tools all work
- **CI/CD friendly** - Tests run without service containers

#### SQLite Limitations

- Single-writer (no concurrent indexing)
- Vector search via sqlite-vec (768/1536 dimensions)
- No parallel query execution

### PostgreSQL Backend

PostgreSQL is recommended for:

- Teams sharing a code index
- High-concurrency production deployments
- Parallel indexing across multiple worktrees
- Advanced features (recursive CTE queries)

[Existing PostgreSQL documentation continues below...]
```

## Component Interaction

```
┌─────────────────────────────────────────────────────────────────┐
│                        User Journey                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   README.md                                                      │
│   ├── Quick Start (SQLite) ←── Most users start here            │
│   │   └── Works immediately                                      │
│   │                                                              │
│   └── Advanced: PostgreSQL ←── Power users discover this        │
│       └── Links to docs/guides/                                  │
│                                                                  │
│   docs/architecture/DATABASE_ARCHITECTURE.md                     │
│   ├── Backend Options (new section)                              │
│   │   ├── SQLite (default)                                       │
│   │   └── PostgreSQL (advanced)                                  │
│   └── [Existing PostgreSQL detail]                               │
│                                                                  │
│   .github/workflows/test.yml                                     │
│   ├── test-sqlite (primary) ←── PRs require this                │
│   └── test-postgres (optional) ←── Integration validation       │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## File Changes Summary

### Modified Files

| File | Change Type | Description |
|------|-------------|-------------|
| `README.md` | Major rewrite | SQLite-first Quick Start |
| `.github/workflows/test.yml` | Restructure | Job naming, optional PostgreSQL |
| `docs/architecture/DATABASE_ARCHITECTURE.md` | Addition | SQLite section at top |
| `config/docker-compose.yml` | Comments | When to use PostgreSQL |
| `packages/vscode-maproom/config/docker-compose.yml` | Comments | Reference SQLite option |

### New Files

| File | Purpose |
|------|---------|
| `docs/guides/GETTING_STARTED.md` | Optional: Expanded zero-to-search guide |

### Unchanged Files

| File | Reason |
|------|--------|
| All source code | No code changes needed |
| `packages/maproom-mcp/README.md` | Already MCP-focused |
| `.devcontainer/docker-compose.yml` | Dev environment still needs PostgreSQL |

## Dependencies

### External Dependencies

- None - this project modifies only configuration and documentation

### Internal Dependencies

- Depends on: VECSTORE, MAPCLI, MCPDB, VSCODEDB (all complete)
- Blocks: None (final project in sequence)

## Performance Considerations

### CI Performance

- SQLite tests run ~30-60 seconds faster (no container startup)
- PostgreSQL tests add ~2-3 minutes for container startup
- Making PostgreSQL optional reduces average CI time

### Documentation Performance

- No performance impact
- Improved user onboarding time (reduced "time to first search")

## Security Considerations

See [security-review.md](./security-review.md) for detailed assessment.

Key points:
- No new code, minimal security surface
- Documentation changes don't introduce vulnerabilities
- CI changes don't expose secrets

## Future Considerations

### Potential Enhancements (Post-MVP)

1. **Interactive Backend Selector** - CLI wizard for backend choice
2. **Migration Tool** - SQLite ↔ PostgreSQL data migration
3. **Hybrid Mode** - Local SQLite with PostgreSQL sync
4. **CI Badge** - Show SQLite/PostgreSQL test status separately

### Known Limitations

1. **DevContainer Still PostgreSQL** - Development environment benefits from PostgreSQL, not changing
2. **No Automated Migration** - Users must re-index when switching backends
3. **Documentation Drift Risk** - Two backend paths need ongoing maintenance

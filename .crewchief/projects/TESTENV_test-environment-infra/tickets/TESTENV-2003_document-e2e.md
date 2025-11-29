# Ticket: TESTENV-2003: Document E2E testing workflow

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**: N/A - Documentation ticket. Verification is that documentation is accurate and complete.

## Agents
- typescript-engineer
- verify-ticket
- commit-ticket

## Summary
Create comprehensive documentation for the test environment infrastructure, including how to run fixture-based tests, E2E tests with the daemon, and how to regenerate fixtures when the schema changes.

## Background
The test infrastructure now has two modes: fixture-based (fast, default) and daemon-based (E2E, optional). Developers need clear documentation on how to use each mode, when to use them, and how to maintain the fixtures.

Reference: [plan.md](../planning/plan.md) - Documentation Updates section
Reference: [quality-strategy.md](../planning/quality-strategy.md) - Schema Drift Detection section

## Acceptance Criteria
- [ ] `packages/maproom-mcp/tests/README.md` created with complete testing guide
- [ ] Documentation explains fixture-based vs E2E testing
- [ ] Instructions for running each test mode
- [ ] Fixture regeneration process documented
- [ ] Troubleshooting section for common issues
- [ ] `packages/maproom-mcp/CLAUDE.md` updated with test section

## Technical Requirements

### tests/README.md Structure

Create `packages/maproom-mcp/tests/README.md`:

```markdown
# Maproom MCP Tests

## Overview

This package uses a two-tier testing approach:

1. **Fixture-based tests** (default) - Fast, deterministic, no daemon required
2. **E2E tests** (optional) - Real indexing with daemon, requires Docker

## Quick Start

### Run Fixture-Based Tests (Default)
```bash
cd packages/maproom-mcp
pnpm test
```

This runs all integration tests using pre-loaded SQL fixtures. No additional setup required.

### Run E2E Tests (With Daemon)
```bash
# Start daemon with e2e profile
docker compose -p crewchief-dev-env --profile e2e up -d

# Run tests with daemon URL
MAPROOM_DAEMON_URL=http://localhost:3000 pnpm test

# Stop daemon when done
docker compose -p crewchief-dev-env --profile e2e down
```

## Test Categories

| Category | Location | Daemon Required | Fixtures |
|----------|----------|-----------------|----------|
| Schema validation | `tests/integration/*.test.ts` | No | No |
| Search quality | `tests/integration/search-*.test.ts` | No | Yes |
| E2E indexing | `tests/e2e/*.test.ts` | Yes | No |

## Fixtures

### What Are Fixtures?

SQL fixtures contain pre-indexed test data (repos, worktrees, files, chunks) that simulate what the daemon would produce. This enables fast, deterministic testing without running the daemon.

### Fixture Location
- `tests/setup/test-fixtures.sql` - SQL INSERT statements
- `tests/corpus/` - Source files used to generate fixtures

### Regenerating Fixtures

When to regenerate:
- New migration added to `crates/maproom/migrations/`
- Schema columns changed (chunks, files, repos tables)
- Test corpus files modified

How to regenerate:
```bash
cd packages/maproom-mcp

# Ensure daemon is running
docker compose -p crewchief-dev-env --profile e2e up -d

# Generate new fixtures
./scripts/create-test-fixtures.sh

# Verify fixtures load
pnpm test

# Commit updated fixtures
git add tests/setup/test-fixtures.sql
git commit -m "chore: regenerate test fixtures"
```

### Fixture Version Tracking

Fixtures include a version header:
```sql
-- Fixture Version: 1.0.0
-- Compatible Schema: migrations 0000-0020
-- Generated: 2025-XX-XX
```

If CI fails with "fixture schema mismatch", regenerate fixtures.

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `TEST_MAPROOM_DATABASE_URL` | (from docker-compose) | Test database connection |
| `MAPROOM_DAEMON_URL` | (not set) | Daemon URL for E2E tests |

## Troubleshooting

### Tests fail with "relation does not exist"

Schema not initialized. Run:
```bash
docker compose -p crewchief-dev-env down
docker compose -p crewchief-dev-env up -d postgres-test
pnpm test
```

### E2E tests skip unexpectedly

`MAPROOM_DAEMON_URL` not set. Set it explicitly:
```bash
MAPROOM_DAEMON_URL=http://localhost:3000 pnpm test
```

### Fixture load fails with constraint error

Fixtures may be stale. Regenerate:
```bash
./scripts/create-test-fixtures.sh
```

### Daemon health check fails

Check daemon logs:
```bash
docker logs maproom-daemon
```

Common issues:
- Database not ready (wait for postgres-test health)
- Port 3000 in use (stop other services)
- Build failed (check `docker compose --profile e2e build`)

## Architecture

```
┌────────────────────────────────────────────────┐
│            Test Environment                     │
│                                                 │
│   postgres-test (always)                        │
│   ├── Schema (init-schema.sql)                 │
│   └── Fixtures (test-fixtures.sql)             │
│                                                 │
│   maproom-daemon (e2e profile only)            │
│   └── Real indexing for E2E tests              │
│                                                 │
│   Vitest                                        │
│   ├── Fixture tests (default)                  │
│   └── E2E tests (when daemon available)        │
└────────────────────────────────────────────────┘
```

## CI Integration

CI runs fixture-based tests by default. The test workflow:
1. Starts postgres-test service container
2. Initializes schema via Rust migrations
3. Loads test fixtures
4. Runs vitest

E2E tests run separately if configured in CI (future enhancement).
```

### CLAUDE.md Update

Add to `packages/maproom-mcp/CLAUDE.md`:

```markdown
## Testing

### Quick Commands
```bash
pnpm test          # Run all tests (fixture-based)
pnpm test:watch    # Watch mode
```

### E2E Testing (with daemon)
```bash
docker compose -p crewchief-dev-env --profile e2e up -d
MAPROOM_DAEMON_URL=http://localhost:3000 pnpm test
```

### Fixture Management
- Fixtures: `tests/setup/test-fixtures.sql`
- Corpus: `tests/corpus/`
- Regenerate: `./scripts/create-test-fixtures.sh`

See `tests/README.md` for full testing documentation.
```

## Implementation Notes

1. **Test actual commands** - Verify all documented commands work before committing

2. **Keep examples current** - Use actual file paths and environment variables from the codebase

3. **Include troubleshooting** - Add common issues encountered during development

4. **Link to related docs** - Reference architecture.md and quality-strategy.md where appropriate

5. **Version in header** - Include fixture version format in documentation

## Dependencies
- TESTENV-1006 (Phase 1 complete)
- TESTENV-2001 (daemon service)
- TESTENV-2002 (E2E test helpers)

## Risk Assessment
- **Risk**: Documentation becomes stale
  - **Mitigation**: Keep docs minimal; reference code directly where possible
- **Risk**: Commands don't work as documented
  - **Mitigation**: Test all commands before committing

## Files/Packages Affected
- `packages/maproom-mcp/tests/README.md` (NEW)
- `packages/maproom-mcp/CLAUDE.md` (MODIFY - add Testing section)

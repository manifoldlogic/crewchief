# Architecture: Test Environment Infrastructure

## Overview

This architecture provides two complementary test infrastructure components:

1. **SQL Test Fixtures** - Pre-indexed data for fast, deterministic tests
2. **Dockerized Daemon** - Containerized Rust binary for E2E tests

## Target Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Test Environment (crewchief-dev-env)         │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                    Docker Network                         │   │
│  │                                                           │   │
│  │  ┌─────────────────┐        ┌─────────────────────────┐  │   │
│  │  │ postgres-test   │◄───────│  maproom-daemon         │  │   │
│  │  │ :5432           │        │  (profile: e2e)         │  │   │
│  │  │                 │        │                         │  │   │
│  │  │ Schema: maproom │        │ MAPROOM_DATABASE_URL=   │  │   │
│  │  │ + Fixtures      │        │ postgres-test:5432      │  │   │
│  │  └─────────────────┘        └─────────────────────────┘  │   │
│  │         ▲                             ▲                   │   │
│  │         │                             │                   │   │
│  └─────────┼─────────────────────────────┼───────────────────┘   │
│            │                             │                       │
│            │ host.docker.internal:5434   │ HTTP/JSON-RPC         │
│            │                             │                       │
│  ┌─────────┴─────────────────────────────┴───────────────────┐   │
│  │                    Vitest Tests                            │   │
│  │                                                            │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐  │   │
│  │  │ Unit Tests   │  │ Integration  │  │ E2E Tests       │  │   │
│  │  │ (mocked)     │  │ (fixtures)   │  │ (real daemon)   │  │   │
│  │  └──────────────┘  └──────────────┘  └─────────────────┘  │   │
│  │                                                            │   │
│  └────────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────┘
```

## Component 1: SQL Test Fixtures

### File Structure

```
packages/maproom-mcp/
├── tests/
│   ├── setup/
│   │   ├── ensure-test-db.ts      # Vitest globalSetup (existing)
│   │   ├── init-schema.sql        # Schema DDL (existing)
│   │   └── test-fixtures.sql      # Pre-indexed test data (NEW)
│   ├── corpus/                    # Test corpus source files (NEW)
│   │   ├── typescript/
│   │   │   ├── auth-service.ts    # AuthService with authenticate()
│   │   │   └── validate-token.ts  # validateToken() function
│   │   ├── python/
│   │   │   └── validate_token.py  # validate_token() function
│   │   ├── rust/
│   │   │   └── database.rs        # DatabaseConnection struct
│   │   └── README.md              # Corpus documentation
│   ├── fixtures/
│   │   ├── sample-typescript.ts   # Existing sample files
│   │   └── ...
│   └── helpers/
│       ├── database.ts            # Helper functions (modify)
│       └── daemon.ts              # E2E daemon helpers (NEW)
└── scripts/
    └── create-test-fixtures.sh    # Fixture generation (NEW)
```

### Fixture Content

The `test-fixtures.sql` file will contain:

```sql
-- Test Corpus Fixture
-- Provides deterministic data for search-quality tests

-- 1. Repository: test-corpus
INSERT INTO maproom.repos (id, name, root_path) VALUES
  (1000, 'test-corpus', '/tmp/semrank-test-corpus');

-- 2. Worktree: main
INSERT INTO maproom.worktrees (id, repo_id, name, abs_path) VALUES
  (1000, 1000, 'main', '/tmp/semrank-test-corpus');

-- 3. Commit
INSERT INTO maproom.commits (id, repo_id, sha, committed_at) VALUES
  (1000, 1000, 'fixture-commit-sha', NOW());

-- 4. Files (TypeScript, Python, Rust, Markdown)
INSERT INTO maproom.files (...) VALUES ...;

-- 5. Chunks with known search results
-- Key: chunks must produce predictable ranking for tests
INSERT INTO maproom.chunks (...) VALUES ...;

-- Reset sequences
SELECT setval('maproom.repos_id_seq', 1100);
-- ... etc
```

### Fixture Design Criteria

1. **Known Query Results** - Each test query has expected top results
2. **Diverse Content** - TypeScript, Python, Rust, Markdown chunks
3. **Ranking Scenarios** - Implementation vs test vs doc chunks
4. **Minimal Size** - ~100 chunks for fast loading (<50ms)
5. **Idempotent** - Safe to load multiple times

### Schema vs Fixtures: Important Distinction

**Schema initialization** and **fixture loading** are separate concerns:

| Aspect | Schema | Fixtures |
|--------|--------|----------|
| **What** | DDL (CREATE TABLE, INDEX) | DML (INSERT data) |
| **Source** | Rust migrations in `crates/maproom/migrations/` | SQL file in `tests/setup/test-fixtures.sql` |
| **CI Method** | `crewchief-maproom db migrate` | `psql < test-fixtures.sql` |
| **Local Method** | `init-schema.sql` (DDL export) | `psql < test-fixtures.sql` |
| **When to update** | New migration added | Corpus files changed |

**CI Workflow Alignment**: The existing CI workflow uses Rust migrations for schema. This project adds fixture loading *after* schema is ready. Both approaches work together:

```yaml
# CI Flow (conceptual)
1. Start postgres-test container
2. Run: crewchief-maproom db migrate  # Schema from Rust migrations
3. Run: psql < test-fixtures.sql       # Data from fixtures (NEW)
4. Run: pnpm test                       # Tests use pre-loaded data
```

### Integration with Test Setup

```typescript
// ensure-test-db.ts (enhanced)
export async function setup(): Promise<void> {
  // Step 1: Ensure container is running (existing)
  ensureContainerRunning()

  // Step 2: Initialize schema (existing - uses init-schema.sql locally)
  // In CI, schema comes from Rust migrations
  if (!isSchemaInitialized()) {
    initializeSchema()
  }

  // Step 3: Load fixtures (NEW)
  // Fixtures are DATA ONLY - no DDL
  if (!isTestCorpusLoaded()) {
    loadTestFixtures()
  }

  verifyTestCorpus()
}

function loadTestFixtures(): void {
  // Load pre-indexed test data (INSERT statements only)
  const fixtureSQL = readFileSync(FIXTURE_FILE, 'utf-8')
  execSync(`docker exec -i ${CONTAINER} psql -U maproom -d maproom_test`, {
    input: fixtureSQL
  })
}
```

## Component 2: Dockerized Daemon

### Existing Dockerfile

**Location**: `/workspace/Dockerfile.maproom` (already exists!)

The existing Dockerfile already implements all required features:
- Multi-stage build (rust:1.82-slim → debian:bookworm-slim)
- Non-root user (`maproom`, uid 1000)
- Health check on port 3000
- Stripped binary for minimal image size

### Docker Compose Addition

```yaml
# packages/vscode-maproom/config/docker-compose.yml
services:
  # ... existing postgres and postgres-test ...

  maproom-daemon:
    container_name: maproom-daemon
    profiles:
      - e2e
    build:
      context: ../../..           # Repository root
      dockerfile: Dockerfile.maproom  # Use existing Dockerfile
    environment:
      MAPROOM_DATABASE_URL: postgresql://maproom:maproom@postgres-test:5432/maproom_test
      MAPROOM_EMBEDDING_PROVIDER: ollama
      OLLAMA_HOST: http://host.docker.internal:11434
      RUST_LOG: info
    depends_on:
      postgres-test:
        condition: service_healthy
    networks:
      maproom-network:
        aliases:
          - maproom-daemon
    healthcheck:
      test: ["CMD", "crewchief-maproom", "status"]
      interval: 10s
      timeout: 5s
      retries: 3
      start_period: 30s
```

### E2E Test Integration

#### Environment Variable: `MAPROOM_DAEMON_URL`

The E2E skip logic uses `MAPROOM_DAEMON_URL` to detect daemon availability:

| Environment | `MAPROOM_DAEMON_URL` Value | E2E Tests |
|-------------|---------------------------|-----------|
| Local (no daemon) | *undefined* | Skipped |
| Local (with daemon) | `http://localhost:3000` | Run |
| CI (fixture tests) | *undefined* | Skipped |
| CI (E2E job) | `http://maproom-daemon:3000` | Run |

**How to enable E2E locally:**
```bash
# Start daemon with e2e profile
docker compose --profile e2e up -d

# Run tests with daemon URL set
MAPROOM_DAEMON_URL=http://localhost:3000 pnpm test
```

**CI Configuration:**
```yaml
# E2E job sets the variable
env:
  MAPROOM_DAEMON_URL: http://maproom-daemon:3000
```

#### Daemon Helper Functions

```typescript
// tests/helpers/daemon.ts

/**
 * Check if daemon is available based on environment variable.
 * Used for skipIf conditions in E2E tests.
 */
export function isDaemonAvailable(): boolean {
  return process.env.MAPROOM_DAEMON_URL !== undefined
}

/**
 * Get daemon URL, throwing if not available.
 * Use after checking isDaemonAvailable().
 */
export function getDaemonUrl(): string {
  const url = process.env.MAPROOM_DAEMON_URL
  if (!url) throw new Error('MAPROOM_DAEMON_URL not set')
  return url
}

/**
 * Wait for daemon to become healthy before running tests.
 * Called at E2E test suite setup.
 */
export async function waitForDaemon(): Promise<void> {
  const url = getDaemonUrl()
  const maxAttempts = 30

  for (let i = 0; i < maxAttempts; i++) {
    try {
      const response = await fetch(`${url}/health`)
      if (response.ok) return
    } catch {
      await sleep(1000)
    }
  }
  throw new Error(`Daemon at ${url} did not become healthy after ${maxAttempts}s`)
}
```

#### E2E Test Pattern

```typescript
// tests/e2e/indexing.test.ts
import { isDaemonAvailable, waitForDaemon, getDaemonUrl } from '../helpers/daemon'

describe.skipIf(!isDaemonAvailable())('E2E: Real Indexing', () => {
  beforeAll(async () => {
    await waitForDaemon()
  })

  it('indexes files via daemon', async () => {
    const daemonUrl = getDaemonUrl()
    // Test real indexing...
  })
})
```

## Test Categories

### Category 1: Schema Tests
- **Approach**: Fixtures
- **Setup**: Schema only
- **Example**: `004-worktree-tracking.test.ts`

### Category 2: Query Tests
- **Approach**: Fixtures
- **Setup**: Schema + test-fixtures.sql
- **Example**: `jsonb-queries.test.ts`

### Category 3: Search Quality Tests
- **Approach**: Fixtures with known results
- **Setup**: Schema + test-fixtures.sql
- **Example**: `search-quality.test.ts` (most tests)

### Category 4: E2E Indexing Tests
- **Approach**: Real daemon
- **Setup**: Schema + daemon container
- **Example**: Tests that verify actual indexing
- **Skip condition**: `describe.skipIf(!isDaemonAvailable())`

## Technology Choices

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Fixture format | SQL dump | Matches existing pattern, fast loading |
| Fixture size | ~100 chunks | Balance coverage vs load time |
| Daemon container | Multi-stage Dockerfile | Minimal image size |
| E2E activation | Docker profile | Opt-in for expensive tests |
| Network | Shared `maproom-network` | Container-to-container communication |

## Performance Targets

| Metric | Target | Rationale |
|--------|--------|-----------|
| Fixture load time | <50ms | Tests should start fast |
| Schema init | <100ms | Already achieved |
| Daemon startup | <5s | Acceptable for E2E only |
| Full test suite | <30s | Fast feedback loop |

## Migration Path

1. **Current**: 392 passing, 5 failing (daemon-dependent)
2. **After fixtures**: 397 passing (all fixture-compatible tests)
3. **After daemon**: E2E tests available when needed

## Dependencies

- PostgreSQL 16 with pgvector (existing)
- Docker Compose (existing)
- Rust toolchain (for daemon build)
- Node.js/Vitest (existing)

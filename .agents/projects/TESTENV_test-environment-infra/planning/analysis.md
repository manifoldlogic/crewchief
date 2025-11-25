# Analysis: Test Environment Infrastructure

## Problem Definition

### Current State

The MCP server (`@crewchief/maproom-mcp`) has integration tests that verify:
- Database schema correctness (migrations 0018-0020)
- JSONB query functionality (worktree_ids filtering)
- Search quality and ranking behavior
- End-to-end tool workflows

**What Works:**
- Schema initialization (MCPSIMP-4003) - test database gets proper schema
- Tests that only verify schema structure pass (37 tests)
- Simple database queries work correctly

**What Fails:**
- Tests requiring indexed data fail (5 tests in `search-quality.test.ts`)
- Rust daemon spawns but can't connect to database (network mismatch)
- No standardized test fixtures exist for the MCP package

### Root Cause Analysis

```
Test Environment Architecture (Current - Broken for E2E)
┌─────────────────────────────────────────────────────────┐
│                    DevContainer (Host)                   │
│  ┌─────────────────┐    ┌──────────────────────────┐   │
│  │   Vitest Tests  │    │  Rust Daemon (spawned)   │   │
│  │  (Node.js)      │◄───┤  crewchief-maproom       │   │
│  └────────┬────────┘    └───────────┬──────────────┘   │
│           │                         │                   │
│           │ host.docker.internal    │ ??? (can't reach) │
│           ▼                         ▼                   │
└───────────┼─────────────────────────┼───────────────────┘
            │                         │
┌───────────┼─────────────────────────┼───────────────────┐
│           │       Docker Network    │                   │
│  ┌────────▼────────┐                │                   │
│  │ postgres-test   │◄───────────────┘                   │
│  │ :5434 → :5432   │  (needs container network)        │
│  └─────────────────┘                                    │
└─────────────────────────────────────────────────────────┘
```

**Issue 1: Network Topology**
- Tests run in devcontainer, connect via `host.docker.internal:5434`
- Daemon spawned by tests runs on host, tries same URL
- But daemon is a native binary, not aware of Docker networking

**Issue 2: No Fixture System**
- Tests expect a `test-corpus` repository to be indexed
- `ensureTestCorpusIndexed()` tries to run daemon to index
- Without pre-existing fixtures, tests can't proceed

## Existing Solutions in Codebase

### Rust Fixture System
Location: `crates/maproom/scripts/create_fixture.sh`

Creates `mpembed_baseline_100.sql`:
- 100 representative chunks (TypeScript, Rust, Markdown)
- Stratified sampling for diverse coverage
- Preserves FK relationships
- ~192KB, loads in ~33ms

**Strengths:**
- Proven pattern
- Fast loading
- Deterministic

**Gaps for MCP:**
- Fixture is for Rust tests, not MCP package
- Doesn't include search-quality test corpus expectations
- Need MCP-specific fixture with known query results

### Test Corpus Location
The tests expect: `/tmp/semrank-test-corpus`

This corpus was created for SEMRANK project but:
- Not versioned in git
- Not automatically created
- Contents unknown without investigation

## Industry Best Practices

### 1. Database Fixtures (SQLite, Django, Rails)
- Pre-populated SQL dumps loaded at test start
- Deterministic, fast, version-controlled
- Trade-off: fixtures can drift from production schema

### 2. Docker Compose Test Services (Testcontainers)
- Spin up dependent services in containers
- Consistent networking
- Trade-off: slower startup, more resources

### 3. In-Memory Databases (H2, SQLite)
- Fastest possible tests
- Trade-off: not production-parity (PostgreSQL features)

### 4. Fixture Factories (FactoryBot, Faker)
- Generate test data programmatically
- Trade-off: less deterministic, slower

## Recommended Approach: Hybrid

### For 95% of Tests: SQL Fixtures
- Create `packages/maproom-mcp/tests/setup/test-corpus.sql`
- Pre-indexed chunks with known search results
- Loaded after schema in `ensure-test-db.ts`
- Fast, deterministic, works in CI

### For E2E Tests: Dockerized Daemon
- Add `maproom-daemon` service to docker-compose
- Profile-based activation (`--profile e2e`)
- Container-to-container networking
- Run real indexing when needed

## Test Classification

| Test Category | Count | Approach | Reason |
|--------------|-------|----------|--------|
| Schema validation | 37 | Fixtures | Only need tables to exist |
| JSONB queries | 11 | Fixtures | Need chunks with worktree_ids |
| Search ranking | 48 | Fixtures | Need predictable results |
| E2E indexing | 5 | Daemon | Test actual indexing flow |
| Debug mode | 1 | Daemon | Needs real daemon response |

## Key Insights

1. **Fixture-first is correct** - Most tests don't need real indexing
2. **Daemon is for E2E only** - Reserve for tests that specifically verify indexing
3. **Network isolation is the core problem** - Daemon needs container network
4. **Existing patterns work** - Adapt `create_fixture.sh` for MCP package
5. **Known query results** - Fixtures must include expected search outcomes
6. **Dockerfile already exists** - `/workspace/Dockerfile.maproom` is production-ready with multi-stage build, non-root user, and health checks

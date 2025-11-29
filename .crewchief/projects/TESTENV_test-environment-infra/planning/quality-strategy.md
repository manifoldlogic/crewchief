# Quality Strategy: Test Environment Infrastructure

## Testing Philosophy

This project is about **test infrastructure** - the code that enables other tests to run. Quality here means:

1. **Reliability** - Tests never fail due to infrastructure issues
2. **Speed** - Fast feedback loop for developers
3. **Determinism** - Same results every run
4. **Maintainability** - Easy to update when schema changes

## Test Pyramid for This Project

```
                    ┌───────────────────┐
                    │   Manual E2E      │  <- Run rarely
                    │   (CI pipeline)   │     (release validation)
                    └─────────┬─────────┘
                              │
              ┌───────────────┴───────────────┐
              │        Integration Tests       │  <- Run on PR
              │   (fixtures load correctly)    │     (fixture validation)
              └───────────────┬───────────────┘
                              │
      ┌───────────────────────┴───────────────────────┐
      │              Schema Validation Tests           │  <- Run always
      │   (DDL is correct, tables exist, indexes)     │     (fast, cheap)
      └────────────────────────────────────────────────┘
```

## Critical Test Paths

### Path 1: Fixture Loading
**What to test**: Fixtures load without errors, data is accessible

```typescript
// Test: Fixture creates expected data
it('fixture creates test-corpus repo', async () => {
  await loadFixtures()
  const { rows } = await client.query(
    "SELECT name FROM maproom.repos WHERE name = 'test-corpus'"
  )
  expect(rows).toHaveLength(1)
})

// Test: Fixture is idempotent
it('fixture can be loaded multiple times', async () => {
  await loadFixtures()
  await loadFixtures() // Should not throw
})
```

### Path 2: Search Result Determinism
**What to test**: Known queries return expected results

```typescript
// Test: Specific query returns known top result
it('authenticate query returns AuthService first', async () => {
  const results = await search(client, 'authenticate')
  expect(results[0].symbol_name).toBe('AuthService')
})
```

### Path 3: Daemon Container (E2E)
**What to test**: Container starts, responds, indexes

```typescript
// Test: Daemon health check passes
it.skipIf(!hasDaemon)('daemon responds to health check', async () => {
  const response = await fetch(`${DAEMON_URL}/health`)
  expect(response.ok).toBe(true)
})
```

## Test Data Design

### Fixture Requirements

The test fixture must include data that produces **known, documented results**.

#### Corpus Files (Versioned in Repository)

Location: `packages/maproom-mcp/tests/corpus/`

| File | Language | Key Symbols | Purpose |
|------|----------|-------------|---------|
| `typescript/auth-service.ts` | TypeScript | `AuthService`, `authenticate()`, `validateToken()` | Auth testing |
| `typescript/database-client.ts` | TypeScript | `DatabaseClient`, `connect()`, `query()` | DB operations |
| `python/validate_token.py` | Python | `validate_token()`, `TokenValidator` | Snake case matching |
| `python/user_service.py` | Python | `UserService`, `get_user()`, `create_user()` | CRUD operations |
| `rust/database.rs` | Rust | `DatabaseConnection`, `impl Connection` | Struct/impl testing |
| `rust/config.rs` | Rust | `Config`, `load_config()` | Configuration |
| `markdown/api-docs.md` | Markdown | Headings, code blocks | Documentation chunks |

#### Query→Result Matrix (Minimum 10 Documented Pairs)

| # | Query | Expected Top Result | Match Type | Ranking Reason |
|---|-------|-------------------|------------|----------------|
| 1 | `authenticate` | `AuthService.authenticate()` | Exact | Function name match |
| 2 | `user authentication` | `AuthService` class | Conceptual | Related to auth concept |
| 3 | `validate_token` | `validate_token()` Python | Exact (snake) | Snake case function |
| 4 | `validateToken` | `validateToken()` TS | Exact (camel) | Camel case function |
| 5 | `DatabaseConnection` | `DatabaseConnection` struct | Exact | Class/struct name |
| 6 | `connect to database` | `DatabaseClient.connect()` | Conceptual | DB connection concept |
| 7 | `query data` | `DatabaseClient.query()` | Conceptual | Query method |
| 8 | `user CRUD` | `UserService` class | Conceptual | User operations |
| 9 | `configuration loading` | `load_config()` | Conceptual | Config loading |
| 10 | `API documentation` | `api-docs.md` heading | Document | Markdown match |
| 11 | `get user by id` | `get_user()` | Conceptual | User retrieval |
| 12 | `impl Connection` | `impl Connection for DatabaseConnection` | Exact | Rust impl block |

#### Fixture Versioning

Fixtures include a version header for schema compatibility tracking:

```sql
-- Fixture Version: 1.0.0
-- Compatible Schema: migrations 0000-0020
-- Generated: 2025-01-XX
-- Generator: packages/maproom-mcp/scripts/create-test-fixtures.sh
--
-- When to regenerate:
-- 1. New migration added to crates/maproom/migrations/
-- 2. Schema columns changed (chunks, files, repos tables)
-- 3. Index definitions changed
-- 4. Test corpus files modified
```

### Fixture Verification Script

```bash
# scripts/verify-fixtures.sh
#!/bin/bash
# Run after fixture changes to verify expected results

psql $TEST_DB -c "
  SELECT symbol_name, kind, ts_rank(ts_doc, query) as rank
  FROM maproom.chunks, to_tsquery('authenticate') query
  WHERE ts_doc @@ query
  ORDER BY rank DESC
  LIMIT 5;
"
```

## Regression Prevention

### Schema Drift Detection (HIGH PRIORITY)

**Risk**: Fixtures become incompatible when schema migrations change column names, types, or constraints.

**Detection Strategy**: Multi-layer validation

#### Layer 1: Fixture Version Header Check

Every fixture file includes a version header:
```sql
-- Fixture Version: 1.0.0
-- Compatible Schema: migrations 0000-0020
-- Last Migration Hash: abc123
```

CI validates the header matches current migrations:
```bash
# scripts/check-fixture-version.sh
FIXTURE_MIGRATION=$(grep "Compatible Schema" test-fixtures.sql | sed 's/.*migrations //')
LATEST_MIGRATION=$(ls -1 crates/maproom/migrations/*.sql | tail -1 | xargs basename)

if [[ "$FIXTURE_MIGRATION" != *"$LATEST_MIGRATION"* ]]; then
  echo "ERROR: Fixtures may be stale. Latest migration: $LATEST_MIGRATION"
  echo "Run: ./scripts/regenerate-fixtures.sh"
  exit 1
fi
```

#### Layer 2: Load Test in CI

```yaml
# .github/workflows/test.yml
- name: Verify fixture schema compatibility
  run: |
    # Load schema via Rust migrations
    MAPROOM_DATABASE_URL=$TEST_DB ./target/release/crewchief-maproom db migrate

    # Load fixtures (will fail with constraint violations if schema changed)
    PGPASSWORD=maproom psql -h localhost -p 5434 -U maproom -d maproom_test \
      < packages/maproom-mcp/tests/setup/test-fixtures.sql

    # Verify expected data exists
    PGPASSWORD=maproom psql -h localhost -p 5434 -U maproom -d maproom_test \
      -c "SELECT COUNT(*) FROM maproom.chunks WHERE symbol_name = 'AuthService'" \
      | grep -q "1" || (echo "Fixture validation failed" && exit 1)
```

#### Layer 3: Query Result Verification

After loading fixtures, verify expected query results:
```typescript
// In test setup
async function verifyFixtureIntegrity(): Promise<void> {
  // Spot-check critical query results
  const authResult = await search(client, 'authenticate')
  if (authResult[0]?.symbol_name !== 'AuthService') {
    throw new Error('Fixture integrity check failed: authenticate query')
  }
}
```

#### When Fixtures Need Regeneration

| Trigger | Action | Automation |
|---------|--------|------------|
| New migration added | Regenerate fixtures | CI warns, manual action |
| Column renamed | Regenerate fixtures | CI fails with SQL error |
| Column type changed | Regenerate fixtures | CI fails with type error |
| Index changed | Usually no action | May affect query order |
| Corpus file modified | Regenerate fixtures | Manual action |

### Fixture Update Automation

```bash
# scripts/regenerate-fixtures.sh
# Called when schema changes

# 1. Start fresh test database
docker compose up -d postgres-test

# 2. Apply latest schema
psql $TEST_DB < init-schema.sql

# 3. Index test corpus with daemon
docker compose --profile e2e up -d maproom-daemon
crewchief-maproom scan --repo test-corpus ...

# 4. Export new fixtures
./create-mcp-fixture.sh > test-fixtures.sql

# 5. Commit updated fixtures
git add test-fixtures.sql
git commit -m "chore: regenerate test fixtures for schema update"
```

## CI Integration

### GitHub Actions Test Matrix

```yaml
jobs:
  test:
    strategy:
      matrix:
        test-type: [unit, integration, e2e]
    steps:
      - name: Unit Tests
        if: matrix.test-type == 'unit'
        run: pnpm test:unit

      - name: Integration Tests (Fixtures)
        if: matrix.test-type == 'integration'
        run: |
          docker compose --profile test up -d postgres-test
          pnpm test:vitest

      - name: E2E Tests (Daemon)
        if: matrix.test-type == 'e2e'
        run: |
          docker compose --profile test --profile e2e up -d
          pnpm test:e2e
```

## Quality Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Test reliability | 100% | No flaky tests in last 50 runs |
| Fixture load time | <50ms | Measured in test output |
| Test suite time | <30s | CI job duration |
| Coverage of test infra | >80% | Core paths covered |

## Risk Mitigation

### Risk: Fixtures Become Stale
**Mitigation**:
- CI job that loads fixtures on every PR
- Fixture version in comments, linked to schema version
- Regeneration script with clear instructions

### Risk: E2E Tests Flaky
**Mitigation**:
- Health check waits before running tests
- Retry logic for daemon startup
- Mark E2E tests as allow-failure in CI initially

### Risk: Different Results in CI vs Local
**Mitigation**:
- Use exact same Docker images
- Pin all dependency versions
- Document environment requirements

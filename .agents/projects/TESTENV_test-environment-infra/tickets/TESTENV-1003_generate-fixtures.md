# Ticket: TESTENV-1003: Generate initial test fixtures

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - fixture loads correctly and query results validated
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**: Validation via fixture load test and query verification.

## Agents
- database-engineer
- verify-ticket
- commit-ticket

## Summary
Execute the fixture generation script to create the initial `test-fixtures.sql` file and validate that the fixtures produce expected query results.

## Background
With the test corpus (TESTENV-1001) and generation script (TESTENV-1002) in place, this ticket executes the actual fixture generation and validates the output meets the documented query→result expectations.

Reference: [plan.md](../planning/plan.md) - Phase 1, Deliverable 3: "Test Fixtures SQL"

## Acceptance Criteria
- [ ] `test-fixtures.sql` generated at `packages/maproom-mcp/tests/setup/`
- [ ] Fixture file size is <500KB (target: ~100 chunks)
- [ ] Fixture loads successfully into fresh test database
- [ ] Fixture load time is <50ms
- [ ] At least 10 of 12 documented query→result pairs return expected top result
- [ ] Fixture is idempotent (can be loaded multiple times without error)

## Technical Requirements

### Fixture Generation Process
```bash
# 1. Ensure test database is running
docker compose -p crewchief-dev-env up -d postgres-test

# 2. Initialize schema
docker exec -i postgres-test-maproom psql -U maproom -d maproom_test \
  < packages/maproom-mcp/tests/setup/init-schema.sql

# 3. Start daemon for indexing
docker compose -p crewchief-dev-env --profile e2e up -d maproom-daemon
# OR run daemon locally:
# MAPROOM_DATABASE_URL=... cargo run --bin crewchief-maproom -- serve

# 4. Generate fixtures
cd packages/maproom-mcp
./scripts/create-test-fixtures.sh

# 5. Verify fixtures load
docker exec -i postgres-test-maproom psql -U maproom -d maproom_test \
  < tests/setup/test-fixtures.sql
```

### Validation Queries
After loading fixtures, verify key query results:
```sql
-- Query 1: "authenticate" should return AuthService
SELECT symbol_name, kind
FROM maproom.chunks
WHERE ts_doc @@ to_tsquery('authenticate')
ORDER BY ts_rank(ts_doc, to_tsquery('authenticate')) DESC
LIMIT 1;
-- Expected: AuthService (or authenticate method)

-- Query 5: "DatabaseConnection" should return struct
SELECT symbol_name, kind
FROM maproom.chunks
WHERE symbol_name ILIKE '%DatabaseConnection%'
LIMIT 1;
-- Expected: DatabaseConnection

-- Check chunk count
SELECT COUNT(*) FROM maproom.chunks;
-- Expected: 50-150 chunks
```

### Expected Fixture Statistics
| Metric | Target | Acceptable Range |
|--------|--------|-----------------|
| Total chunks | ~100 | 50-150 |
| TypeScript chunks | ~40 | 30-60 |
| Python chunks | ~30 | 20-40 |
| Rust chunks | ~20 | 10-30 |
| Markdown chunks | ~10 | 5-20 |
| File size | <200KB | <500KB |
| Load time | <50ms | <100ms |

## Implementation Notes

1. **Run generation in clean environment** - Start with fresh database to avoid stale data

2. **Verify query results manually** - Run the 12 documented queries and verify results before committing

3. **Adjust corpus if needed** - If query results don't match expectations, may need to adjust corpus files from TESTENV-1001

4. **Document any deviations** - If a query result differs from expected, update the README to reflect actual behavior

5. **Measure load time**:
   ```bash
   time docker exec -i postgres-test-maproom psql -U maproom -d maproom_test \
     < tests/setup/test-fixtures.sql
   ```

## Dependencies
- TESTENV-1001 (test corpus files)
- TESTENV-1002 (fixture generation script)

## Risk Assessment
- **Risk**: Query results don't match documented expectations
  - **Mitigation**: Adjust corpus files or update documentation to reflect actual ranking behavior
- **Risk**: Fixture file too large
  - **Mitigation**: Reduce corpus size; target 50-100 chunks
- **Risk**: Embedding generation fails/times out
  - **Mitigation**: Skip embeddings for MVP if needed; FTS works without them

## Files/Packages Affected
- `packages/maproom-mcp/tests/setup/test-fixtures.sql` (NEW)
- `packages/maproom-mcp/tests/corpus/README.md` (UPDATE if query results differ)

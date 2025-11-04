# Ticket: DBFALLBK-4001: End-to-End Scenario Testing for Connection Fallback

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- test-runner
- verify-ticket
- commit-ticket

## Summary
Execute manual end-to-end testing of all 4 user scenarios to verify the complete connection fallback system works correctly in real environments.

## Background
After implementing the connection fallback logic in Phases 1-3, we need comprehensive end-to-end testing to verify all scenarios work correctly in real-world usage:
- Devcontainer with DATABASE_URL set to maproom-postgres
- MCP user with no DATABASE_URL (auto-detection)
- Direct Rust binary usage with no DATABASE_URL
- MAPROOM_DB_HOST override

This implements the testing strategy from planning/quality-strategy.md Phase 4 (End-to-End Scenario Tests).

## Acceptance Criteria
- [x] All 4 scenarios tested and documented
- [x] Scenario 1 (Devcontainer): Uses maproom-postgres from DATABASE_URL
- [x] Scenario 2 (MCP user): Auto-detects maproom-postgres
- [x] Scenario 3 (Direct Rust): Auto-detects maproom-postgres
- [x] Scenario 4 (Override): Uses MAPROOM_DB_HOST value
- [x] All connection attempts succeed
- [x] Logging output is clear and helpful
- [x] No database connection errors

## Technical Requirements
Test the following 4 scenarios manually and document results:

**Scenario 1: Devcontainer** (DATABASE_URL set to maproom-postgres)
```bash
cd /workspace
echo $DATABASE_URL
# Expected: postgresql://maproom:maproom@maproom-postgres:5432/maproom

cargo run --bin crewchief-maproom -- db status
# Expected: Uses postgresql://maproom:***@maproom-postgres:5432/maproom

node packages/maproom-mcp/bin/cli.cjs scan /workspace
# Expected: Shows "Using explicit DATABASE_URL from environment"
```

**Scenario 2: MCP User** (no DATABASE_URL)
```bash
unset DATABASE_URL
docker compose -f ~/.maproom-mcp/docker-compose.yml ps
# Verify maproom-postgres is running

node packages/maproom-mcp/bin/cli.cjs scan /workspace
# Expected: Shows "Auto-detected database connection"
# Expected: Uses maproom-postgres
```

**Scenario 3: Direct Rust Binary** (no DATABASE_URL)
```bash
unset DATABASE_URL
cargo run --bin crewchief-maproom -- db status
# Expected: Shows "Auto-detected maproom-postgres hostname"
# Expected: Connects successfully
```

**Scenario 4: MAPROOM_DB_HOST Override**
```bash
export MAPROOM_DB_HOST=maproom-postgres
export MAPROOM_DB_PORT=5432
cargo run --bin crewchief-maproom -- db status
# Expected: Shows "Using MAPROOM_DB_HOST: maproom-postgres"
# Expected: Connects successfully
```

## Implementation Notes
For each scenario:
1. Execute the commands
2. Verify the connection succeeds
3. Check the log output shows correct connection method
4. Document any issues found
5. Verify database name/host matches expectations

Create a test results document with:
- Scenario name
- Commands run
- Expected output
- Actual output
- Pass/Fail status
- Any issues found

If any scenario fails, create follow-up tickets to fix the issues before marking this ticket complete.

## Dependencies
- DBFALLBK-1001 (Remove devcontainer postgres) must be complete
- DBFALLBK-2001 (Rust fallback logic) must be complete
- DBFALLBK-3001 (Node.js CLI updates) must be complete

## Risk Assessment
- **Risk**: Scenarios might fail due to environment issues
  - **Mitigation**: Document environment state, fix issues found
- **Risk**: maproom-postgres might not be running
  - **Mitigation**: Start it as part of test procedure

## Files/Packages Affected
- Create test results document (e.g., /workspace/.agents/projects/DBFALLBK_database-connection-fallback/test-results.md)

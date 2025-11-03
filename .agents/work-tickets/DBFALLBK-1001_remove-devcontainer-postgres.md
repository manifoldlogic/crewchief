# Ticket: DBFALLBK-1001: Remove Devcontainer Postgres Service

## Status
- [x] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- test-runner
- verify-ticket
- commit-ticket

## Summary
Remove the postgres service from the devcontainer docker-compose.yml and update all configuration to use only maproom-postgres, eliminating the confusing dual database setup.

## Background
CrewChief currently has two PostgreSQL databases:
1. Devcontainer postgres (postgres:5432/crewchief) - for local development
2. Maproom MCP postgres (maproom-postgres:5432/maproom) - for MCP service

This dual setup causes confusion because:
- Developers don't know which database they're using
- The Node.js CLI overrides DATABASE_URL and uses maproom-postgres even when devcontainer sets postgres
- Data can be inconsistent between the two databases
- Unnecessary complexity in the architecture

This ticket implements Phase 1 from planning/plan.md: eliminating the devcontainer postgres to use only maproom-postgres as the single source of truth.

## Acceptance Criteria
- [ ] postgres service completely removed from `.devcontainer/docker-compose.yml`
- [ ] postgres-data volume removed from docker-compose.yml
- [ ] DATABASE_URL environment variable points to maproom-postgres
- [ ] All unused CREWCHIEF_DB_* environment variables removed
- [ ] Devcontainer rebuilds successfully without errors
- [ ] Can connect to maproom-postgres from inside devcontainer

## Technical Requirements
- Remove postgres service definition from `.devcontainer/docker-compose.yml`
- Remove postgres-data volume from volumes section
- Update DATABASE_URL to: `postgresql://maproom:maproom@maproom-postgres:5432/maproom`
- Remove these environment variables: CREWCHIEF_DB_HOST, CREWCHIEF_DB_PORT, CREWCHIEF_DB_NAME, CREWCHIEF_DB_USER, CREWCHIEF_DB_PASSWORD
- Update depends_on to remove postgres dependency
- Ensure devcontainer can still access maproom-postgres (verify network connectivity)

## Implementation Notes
The devcontainer docker-compose.yml currently defines a postgres service that is separate from the maproom-postgres service. We need to:

1. Remove the postgres service block entirely
2. Remove the postgres-data volume (no longer needed)
3. Change DATABASE_URL from `postgresql://postgres:postgres@postgres:5432/crewchief` to `postgresql://maproom:maproom@maproom-postgres:5432/maproom`
4. Clean up the CREWCHIEF_DB_* variables which are completely unused in the codebase
5. Verify the devcontainer service can reach maproom-postgres on the shared network

Key file: `/workspace/.devcontainer/docker-compose.yml`

The maproom-postgres service should already be available via the crewchief-network or maproom-network. Verify network configuration allows devcontainer to reach it.

### Steps to Implement
1. Read current docker-compose.yml to understand structure
2. Remove postgres service definition
3. Remove postgres-data volume
4. Update DATABASE_URL environment variable
5. Remove unused CREWCHIEF_DB_* environment variables
6. Update depends_on if it references postgres
7. Verify network configuration allows access to maproom-postgres
8. Rebuild devcontainer to test changes
9. Verify database connectivity from inside devcontainer

### Verification Commands
```bash
# Test database connection from inside devcontainer
psql postgresql://maproom:maproom@maproom-postgres:5432/maproom -c "SELECT 1;"

# Verify environment variable
echo $DATABASE_URL

# List all tables to confirm connectivity
psql $DATABASE_URL -c "\dt maproom.*"
```

## Dependencies
- None (this is the first ticket in the DBFALLBK project)

## Risk Assessment

- **Risk**: Devcontainer rebuild might fail if maproom-postgres isn't accessible
  - **Mitigation**: Verify network configuration allows connectivity; maproom-postgres should be on a shared network or we need to add it

- **Risk**: Developers might have local data in postgres they want to preserve
  - **Mitigation**: This is development database only; data can be re-indexed from source if needed

- **Risk**: Network connectivity issues between devcontainer and maproom-postgres
  - **Mitigation**: Ensure both services are on the same Docker network; verify network configuration before removing postgres service

## Files/Packages Affected

### Files to Modify
- `/workspace/.devcontainer/docker-compose.yml` - Remove postgres service, update DATABASE_URL, remove unused env vars

### Configuration Changes
- Remove postgres service definition
- Remove postgres-data volume
- Update DATABASE_URL environment variable
- Remove CREWCHIEF_DB_* environment variables
- Update depends_on configuration

## Estimated Effort
30 minutes implementation + 15 minutes testing = 45 minutes total

## Priority
**High** - Eliminates architectural confusion and simplifies development environment

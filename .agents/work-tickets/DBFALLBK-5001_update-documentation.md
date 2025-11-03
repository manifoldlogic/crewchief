# Ticket: DBFALLBK-5001: Update Documentation for Single Database Architecture

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Update all project documentation to reflect the removal of devcontainer postgres and the new connection fallback behavior.

## Background
After implementing the database connection fallback system and removing the devcontainer postgres, we need to update all documentation to reflect the new architecture:
- CLAUDE.md needs updated database architecture section
- DATABASE_ARCHITECTURE.md needs devcontainer postgres removed
- maproom-mcp README needs connection fallback behavior documented
- Remove any references to the dual database setup

This implements Phase 5 from planning/plan.md: Documentation & Cleanup.

The previous tickets (DBFALLBK-1001 through DBFALLBK-4001) implemented the technical changes to move from a dual database setup to a single maproom-postgres database with intelligent connection fallback. Now we need to ensure all documentation accurately reflects this new architecture so developers understand the system correctly.

## Acceptance Criteria
- [ ] CLAUDE.md database architecture section updated to show single database (maproom-postgres)
- [ ] DATABASE_ARCHITECTURE.md updated to remove devcontainer postgres references
- [ ] packages/maproom-mcp/README.md documents connection fallback behavior
- [ ] No references to unused CREWCHIEF_DB_* environment variables remain
- [ ] No references to old devcontainer postgres service remain
- [ ] Troubleshooting guide added for connection issues
- [ ] All documentation examples use correct connection strings
- [ ] Documentation is consistent across all files

## Technical Requirements

### Update the following files:

**1. CLAUDE.md** (Database Architecture section):
- Remove description of "Dual PostgreSQL Setup"
- Update to show single database: maproom-postgres
- Document that devcontainer now uses maproom-postgres
- Update connection examples
- Remove "Quick Reference" section showing two databases
- Update "Why two instances?" section (remove it)

**2. docs/architecture/DATABASE_ARCHITECTURE.md**:
- Remove devcontainer PostgreSQL section
- Update architecture diagrams/descriptions
- Document single database architecture
- Update Quick Reference section
- Remove comparisons between two databases
- Update any network architecture diagrams

**3. packages/maproom-mcp/README.md**:
- Add "Database Connection" section explaining fallback behavior
- Document the 4-tier fallback hierarchy:
  1. DATABASE_URL (explicit config)
  2. MAPROOM_DB_HOST (component override)
  3. maproom-postgres (auto-detection)
  4. localhost:5433 (fallback)
- Add examples for each scenario
- Add troubleshooting section for connection issues

**4. Remove obsolete references**:
- Search for CREWCHIEF_DB_HOST, CREWCHIEF_DB_PORT, CREWCHIEF_DB_NAME, CREWCHIEF_DB_USER, CREWCHIEF_DB_PASSWORD and remove
- Search for "devcontainer postgres" and update/remove
- Search for "dual database" and update/remove
- Search for references to postgres:5432 (old devcontainer db) and update

## Implementation Notes

### For CLAUDE.md
Replace the "Database Architecture: Dual PostgreSQL Setup" section with:

```markdown
### Database Architecture: maproom-postgres

CrewChief uses a single PostgreSQL instance for all Maproom operations:

**Maproom PostgreSQL** (`maproom-postgres:5432/maproom`)
- Purpose: Semantic code search, MCP service, development
- Connection: `postgresql://maproom:maproom@maproom-postgres:5432/maproom`
- Network: Accessible from devcontainer and MCP containers
- Data: Persistent via Docker volumes

**Connection Fallback**:
The system automatically detects the database using this priority:
1. DATABASE_URL environment variable (explicit config)
2. MAPROOM_DB_HOST environment variable (component override)
3. maproom-postgres hostname resolution (auto-detection)
4. localhost:5433 (development fallback)

Both Rust binary and Node.js CLI use identical fallback logic for consistency.
```

Remove these sections entirely:
- "Why two instances?"
- "Quick Reference" showing two database commands
- Any mentions of "Development vs. production-like data"
- Any mentions of "Isolation: Development changes don't affect MCP service"

### For DATABASE_ARCHITECTURE.md
Update to reflect:
- Single database architecture
- Connection fallback behavior
- Removal of devcontainer postgres service
- Updated network diagrams (if any)

### For packages/maproom-mcp/README.md
Add a new "Database Connection" section:

```markdown
## Database Connection

The Maproom MCP server uses intelligent connection fallback to detect and connect to the PostgreSQL database:

### Connection Priority

1. **DATABASE_URL** (explicit config): If set, uses this connection string exactly
   ```bash
   export DATABASE_URL="postgresql://user:pass@host:port/dbname"
   ```

2. **MAPROOM_DB_HOST** (component override): If DATABASE_URL not set, constructs connection using MAPROOM_DB_*
   ```bash
   export MAPROOM_DB_HOST="custom-host"
   export MAPROOM_DB_PORT="5432"  # optional, defaults to 5432
   ```

3. **maproom-postgres** (auto-detection): Attempts to connect to maproom-postgres hostname
   - Works automatically in Docker environments
   - No configuration needed if maproom-postgres container is running

4. **localhost:5433** (fallback): Development fallback for local testing
   - Useful for local postgres instances on non-standard port

### Troubleshooting

**Can't connect to database:**
1. Verify maproom-postgres is running: `docker ps | grep maproom-postgres`
2. Start if needed: `cd config && docker compose up -d`
3. Check logs: `docker logs maproom-postgres`

**Connection refused:**
- Verify port 5432 is not blocked
- Check network connectivity: `docker network inspect maproom-network`

**Hostname not found:**
- Verify you're in correct Docker network
- Try setting DATABASE_URL explicitly
```

### Troubleshooting Guide
Add a troubleshooting section covering:
- maproom-postgres not running → docker compose up -d
- Can't resolve hostname → check network configuration
- Connection refused → verify port 5432 not blocked
- Wrong database → check DATABASE_URL environment variable
- Data missing → verify correct database, may need to re-index

### Search Terms for Obsolete References
Use grep to find and update:
- "CREWCHIEF_DB_HOST"
- "CREWCHIEF_DB_PORT"
- "CREWCHIEF_DB_NAME"
- "CREWCHIEF_DB_USER"
- "CREWCHIEF_DB_PASSWORD"
- "devcontainer postgres"
- "devcontainer PostgreSQL"
- "dual database"
- "two separate PostgreSQL instances"
- "postgres:5432" (when referring to old devcontainer db)
- "crewchief database" (may be confusing with maproom database)

### Verification Steps
1. Read all updated files to verify changes are complete
2. Search for obsolete terms to ensure none remain
3. Verify all connection strings point to maproom-postgres
4. Verify troubleshooting guides are accurate
5. Check that documentation is internally consistent

## Dependencies
- DBFALLBK-1001: Remove Devcontainer Postgres Service (should be complete)
- DBFALLBK-2001: Implement Rust Connection Fallback (should be complete)
- DBFALLBK-3001: Update Node.js CLI to Respect DATABASE_URL (should be complete)
- DBFALLBK-4001: End-to-End Scenario Testing (should be complete)

All technical implementation should be done before updating documentation.

## Risk Assessment

- **Risk**: Missing some documentation references to old dual database setup
  - **Mitigation**: Use comprehensive grep searches for multiple related terms; review all markdown files in docs/

- **Risk**: Breaking links in documentation
  - **Mitigation**: Test all internal documentation links after updates

- **Risk**: Documentation might contradict actual behavior if implementation changed
  - **Mitigation**: Verify actual connection behavior before documenting it; test connection fallback

- **Risk**: Users might have bookmarked old documentation sections
  - **Mitigation**: Not a major concern for internal documentation; benefits of clarity outweigh this risk

## Files/Packages Affected

### Files to Modify
- `/workspace/CLAUDE.md` - Update database architecture section
- `/workspace/docs/architecture/DATABASE_ARCHITECTURE.md` - Remove devcontainer postgres references
- `/workspace/packages/maproom-mcp/README.md` - Add connection fallback documentation

### Files to Search for Obsolete References
- All markdown files in `/workspace/docs/`
- `/workspace/README.md`
- `/workspace/packages/cli/README.md`
- Any other README or documentation files

## Estimated Effort
2 hours implementation + 1 hour verification and testing = 3 hours total

## Priority
**High** - Documentation must accurately reflect the implemented architecture to prevent developer confusion

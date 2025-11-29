# Ticket: MCPSIMP-2003: Update Extension docker-compose.yml

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - Tests pass - N/A (configuration file only)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- vscode-extension-specialist
- verify-ticket
- commit-ticket

## Summary
Update the VSCode extension's docker-compose.yml to remove the `ollama` and `maproom-mcp` services, keeping only the `postgres` service. This aligns with the new architecture where the MCP server runs on the host (not in a container).

## Background
The extension's `docker-compose.yml` currently defines three services: postgres, ollama, and maproom-mcp. In the simplified architecture:
- **postgres**: Still needed - the database must be running
- **ollama**: Remove - Ollama is unusably slow in containers; users manage their own Ollama
- **maproom-mcp**: Remove - MCP server now runs on host via `npx`, not in a container

This implements Phase 2.3 of the MCP Server Simplification plan.

## Acceptance Criteria
- [ ] `ollama` service definition removed from docker-compose.yml
- [ ] `maproom-mcp` service definition removed from docker-compose.yml
- [ ] `ollama-models` volume removed
- [ ] `maproom-logs` volume removed
- [ ] `postgres` service and `maproom-data` volume retained (unchanged)
- [ ] `maproom-network` network retained
- [ ] docker-compose.yml reduced from ~144 lines to ~50 lines
- [ ] Extension can still start PostgreSQL container successfully

## Technical Requirements
**Remove services:**
- Delete entire `ollama` service definition (was lines 49-84)
- Delete entire `maproom-mcp` service definition (was lines 86-131)

**Remove volumes:**
- Delete `ollama-models` volume declaration
- Delete `maproom-logs` volume declaration

**Keep unchanged:**
- `postgres` service (database container)
- `maproom-data` volume (database persistence)
- `maproom-network` network (container networking)

**Resulting structure:**
```yaml
version: '3.8'

services:
  postgres:
    # ... existing postgres config unchanged ...

volumes:
  maproom-data:

networks:
  maproom-network:
```

## Implementation Notes
- Read the current docker-compose.yml first to understand the structure
- The postgres service should remain completely unchanged
- After editing, validate YAML syntax (e.g., `docker compose config`)
- Test by running `docker compose up postgres` from the extension's config directory
- The file should be significantly smaller (~50 lines vs ~144 lines)

## Dependencies
- None (can be done in parallel with other Phase 2 tickets)

## Risk Assessment
- **Risk**: Accidentally modifying postgres service configuration
  - **Mitigation**: Only delete services/volumes, don't modify postgres section
- **Risk**: Extension code still references deleted services
  - **Mitigation**: MCPSIMP-2004 updates DockerManager to only use postgres

## Files/Packages Affected
- `packages/vscode-maproom/config/docker-compose.yml` (modify)

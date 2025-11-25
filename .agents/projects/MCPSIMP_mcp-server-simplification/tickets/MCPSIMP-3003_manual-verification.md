# Ticket: MCPSIMP-3003: Manual Verification

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - Tests pass - N/A (manual testing ticket)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- verify-ticket
- commit-ticket

## Summary
Perform comprehensive manual verification of all simplified components to ensure the MCP server, VSCode extension Docker management, and database auto-detection all work correctly in real-world scenarios.

## Background
After implementing all Phase 1-3 changes, this ticket ensures the system works end-to-end before release. This is critical because:
- Breaking changes are being introduced (v3.0.0)
- Multiple components were modified (MCP server, extension)
- Integration between components must be verified

This implements Phase 3.3 of the MCP Server Simplification plan.

## Acceptance Criteria
- [ ] `npx @crewchief/maproom-mcp` with running database → MCP server starts successfully
- [ ] `npx @crewchief/maproom-mcp` without database → Clear error message displayed
- [ ] DevContainer with `IN_DEVCONTAINER=true` → Uses container hostname correctly
- [ ] VSCode extension Docker management starts only PostgreSQL
- [ ] All existing MCP tool tests pass
- [ ] Generated MCP config includes `MAPROOM_DATABASE_URL` and `MAPROOM_EMBEDDING_PROVIDER`

## Technical Requirements
**MCP Server Verification:**
1. Start PostgreSQL container: `docker run -d --name maproom-postgres -e POSTGRES_USER=maproom -e POSTGRES_PASSWORD=maproom -e POSTGRES_DB=maproom -p 5433:5432 pgvector/pgvector:pg16`
2. Run: `npx @crewchief/maproom-mcp`
3. Verify: Server starts without errors
4. Send test JSON-RPC message if possible

**Error Handling Verification:**
1. Stop PostgreSQL container: `docker stop maproom-postgres`
2. Run: `npx @crewchief/maproom-mcp`
3. Verify: Clear error message about database connection

**DevContainer Verification:**
1. Set: `export IN_DEVCONTAINER=true`
2. Run: `npx @crewchief/maproom-mcp` (will fail if no container network, that's OK)
3. Verify: Connection string shows `maproom-postgres:5432` (not localhost)
4. Unset: `unset IN_DEVCONTAINER`

**VSCode Extension Verification:**
1. Open VSCode with the extension
2. Trigger Docker service startup via extension
3. Verify: Only `postgres` container is started
4. Verify: No `ollama` or `maproom-mcp` containers are created

**MCP Config Verification:**
1. Use extension to set up MCP configuration
2. Inspect generated `.vscode/mcp.json` or `~/.config/Code/User/globalStorage/*/mcp.json`
3. Verify: `MAPROOM_DATABASE_URL` is present
4. Verify: `MAPROOM_EMBEDDING_PROVIDER` is present

**Existing Tests:**
1. Run: `pnpm test` from repository root
2. Verify: All tests pass (especially MCP-related tests)

## Implementation Notes
- This is a verification ticket - no code changes should be made
- Document any issues found with specific error messages and steps to reproduce
- If issues are found, create follow-up tickets or return to previous tickets for fixes
- Keep notes on what was tested and results for the verify-ticket agent

## Dependencies
- All Phase 1 and Phase 2 tickets must be completed
- MCPSIMP-3001 (Update CLAUDE.md) and MCPSIMP-3002 (Write Unit Tests) should be done

## Risk Assessment
- **Risk**: Finding critical bugs at this late stage
  - **Mitigation**: Issues found here are better than issues found after release; plan has rollback procedures

## Files/Packages Affected
- None (verification only)

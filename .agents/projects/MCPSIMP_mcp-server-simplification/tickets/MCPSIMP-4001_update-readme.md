# Ticket: MCPSIMP-4001: Update README

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - Tests pass - N/A (documentation only)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Update the maproom-mcp package's README.md to document breaking changes, new usage patterns, and provide a migration guide for v3.0.0.

## Background
The v3.0.0 release is a breaking change that removes Docker orchestration and simplifies the MCP server to a single-purpose tool. Users need clear documentation about:
- What's changed (breaking changes)
- How to use the new version
- How to migrate from v2.x

This implements Phase 4.1 of the MCP Server Simplification plan.

## Acceptance Criteria
- [ ] Breaking changes section clearly documents removed functionality
- [ ] New usage pattern documented: `npx @crewchief/maproom-mcp` as single-purpose MCP server
- [ ] Migration guide for CLI users (non-VSCode) with step-by-step instructions
- [ ] MCP configuration examples for VS Code, Cursor, and Claude Code
- [ ] Environment variable documentation (MAPROOM_DATABASE_URL, IN_DEVCONTAINER, MAPROOM_EMBEDDING_PROVIDER)

## Technical Requirements
The README should include:

**Breaking Changes Section:**
- Version 3.0.0 removes setup, scan, watch subcommands
- Docker orchestration moved to VSCode extension
- Ollama container management removed
- Database must exist before MCP server starts

**New Usage:**
```bash
# Single command - runs MCP server directly
npx @crewchief/maproom-mcp
```

**Migration Guide for CLI Users:**
1. Start PostgreSQL manually:
   ```bash
   docker run -d --name maproom-postgres \
     -e POSTGRES_USER=maproom \
     -e POSTGRES_PASSWORD=maproom \
     -e POSTGRES_DB=maproom \
     -p 5433:5432 \
     pgvector/pgvector:pg16
   ```

2. Run database migrations:
   ```bash
   crewchief-maproom db migrate
   ```

3. Configure MCP client (with example JSON)

4. Index codebase:
   ```bash
   crewchief-maproom scan /path/to/your/repo
   ```

**MCP Configuration Examples:**
- VS Code / Cursor configuration
- Claude Code configuration
- DevContainer configuration (auto-detects database)

**Environment Variables:**
| Variable | Description | Default |
|----------|-------------|---------|
| MAPROOM_DATABASE_URL | Database connection string | Auto-detected |
| IN_DEVCONTAINER | Set to 'true' in devcontainers | Not set |
| MAPROOM_EMBEDDING_PROVIDER | openai, google, or ollama | Required |

## Implementation Notes
- Reference architecture.md for complete migration guide content
- Keep README focused and practical - link to detailed docs where needed
- Use the MCP configuration examples from architecture.md
- Ensure all code examples are copy-paste ready

## Dependencies
- All Phase 1-3 tickets should be completed so README accurately reflects final implementation

## Risk Assessment
- **Risk**: Incomplete migration guide frustrates users
  - **Mitigation**: Use detailed guide from architecture.md; test migration steps
- **Risk**: Examples don't work
  - **Mitigation**: All examples should be tested during MCPSIMP-3003

## Files/Packages Affected
- `packages/maproom-mcp/README.md` (modify)

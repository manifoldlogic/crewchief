# Ticket: MCPSIMP-3001: Update CLAUDE.md

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - Tests pass - N/A (documentation only)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Update the maproom-mcp package's CLAUDE.md to reflect the simplified architecture, removing documentation for deleted commands and updating usage instructions.

## Background
The CLAUDE.md file provides guidance for AI assistants working with the package. After simplification, the documentation needs to:
- Remove references to setup, scan, watch subcommands (deleted)
- Remove Docker orchestration documentation
- Update to show the single-purpose MCP server pattern
- Document the new database resolution hierarchy

This implements Phase 3.1 of the MCP Server Simplification plan.

## Acceptance Criteria
- [ ] References to `setup`, `scan`, `watch` subcommands removed
- [ ] Docker orchestration documentation removed
- [ ] New usage pattern documented: `npx @crewchief/maproom-mcp` runs MCP server directly
- [ ] Database resolution hierarchy documented (MAPROOM_DATABASE_URL > IN_DEVCONTAINER > localhost)
- [ ] File is accurate and helpful for AI assistants working with the package

## Technical Requirements
Update `packages/maproom-mcp/CLAUDE.md` to:
- Remove any sections about subcommands (setup, scan, watch)
- Remove Docker compose or container management instructions
- Add section explaining the simplified architecture:
  - Single-purpose: MCP server only
  - Database must be pre-existing
  - Auto-detection for database URL
- Document environment variables:
  - `MAPROOM_DATABASE_URL` - explicit database connection
  - `IN_DEVCONTAINER` - auto-detection for container environments
  - `MAPROOM_EMBEDDING_PROVIDER` - embedding provider selection
- Update any example commands to show new usage

## Implementation Notes
- Read the current CLAUDE.md first to understand what needs updating
- Keep information that's still relevant (MCP tools, daemon architecture)
- Remove information that's no longer applicable (Docker, subcommands)
- The goal is accuracy and helpfulness, not length - shorter is better if complete

## Dependencies
- **MCPSIMP-1001, 1002, 1003** (Phase 1) - Architecture must be finalized before documenting

## Risk Assessment
- **Risk**: Missing important information for AI assistants
  - **Mitigation**: Review the new architecture thoroughly; test MCP server behavior
- **Risk**: Outdated information remains
  - **Mitigation**: Search for references to deleted functionality

## Files/Packages Affected
- `packages/maproom-mcp/CLAUDE.md` (modify)

# Ticket: MCPSIMP-1001: Replace CLI Entry Point

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Replace the 1,971-line CLI entry point (`cli.cjs`) with a minimal ~50-line entry point that only runs the MCP server via stdio with auto-detected database configuration.

## Background
The current `packages/maproom-mcp/bin/cli.cjs` has grown to 1,971 lines containing Docker orchestration, Ollama management, and complex configuration that duplicates functionality in the VSCode extension. This ticket implements Phase 1.1 of the MCP Server Simplification plan.

**CRITICAL DEPENDENCY**: This task MUST complete before MCPSIMP-1002 (Delete Unused Files). The current cli.cjs imports `config-manager.js` and `docker-detection.js`. Deleting those files before replacing cli.cjs will break the package.

## Acceptance Criteria
- [ ] `packages/maproom-mcp/bin/cli.cjs` is replaced with ~50 line entry point
- [ ] Entry point implements three-tier database resolution: `MAPROOM_DATABASE_URL` > `IN_DEVCONTAINER` > localhost:5433
- [ ] Entry point successfully imports and runs `../dist/index.js`
- [ ] No subcommands (setup, scan, watch) remain - single-purpose MCP server only
- [ ] `npx @crewchief/maproom-mcp` starts MCP server (tested locally)

## Technical Requirements
- Use the exact implementation from plan.md Phase 1.1:
```javascript
#!/usr/bin/env node

/**
 * Maproom MCP Server
 *
 * Single-purpose: Run MCP server via stdio.
 * Expects database to exist (use VSCode extension or docker compose for setup).
 */

function resolveDatabase() {
  // 1. Explicit override
  if (process.env.MAPROOM_DATABASE_URL) {
    return process.env.MAPROOM_DATABASE_URL
  }

  // 2. DevContainer
  if (process.env.IN_DEVCONTAINER === 'true') {
    return 'postgresql://maproom:maproom@maproom-postgres:5432/maproom'
  }

  // 3. Default localhost
  return 'postgresql://maproom:maproom@localhost:5433/maproom'
}

async function main() {
  process.env.MAPROOM_DATABASE_URL = resolveDatabase()
  await import('../dist/index.js')
}

main().catch(error => {
  console.error('MCP server error:', error.message)
  process.exit(1)
})
```
- No new dependencies required
- Must maintain `#!/usr/bin/env node` shebang for npx compatibility

## Implementation Notes
- The existing cli.cjs is complex but the replacement is straightforward copy-paste
- Test the three database resolution paths manually:
  1. Set `MAPROOM_DATABASE_URL=postgresql://test@host:5432/db` and verify it's used
  2. Set `IN_DEVCONTAINER=true` (without MAPROOM_DATABASE_URL) and verify container hostname
  3. Unset both and verify localhost:5433 default
- Error handling via `.catch()` provides clear error messages on startup failure

## Dependencies
- None (this is the first ticket in the sequence)

## Risk Assessment
- **Risk**: Breaking existing users who rely on subcommands
  - **Mitigation**: This is a breaking change (v3.0.0). Migration guide in architecture.md documents the change.
- **Risk**: Import path `../dist/index.js` doesn't exist
  - **Mitigation**: Verify the build output exists; run `pnpm build` in maproom-mcp package first

## Files/Packages Affected
- `packages/maproom-mcp/bin/cli.cjs` (complete replacement)

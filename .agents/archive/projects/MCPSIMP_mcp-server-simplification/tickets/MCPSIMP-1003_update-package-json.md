# Ticket: MCPSIMP-1003: Update Package.json

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - Tests pass - N/A (configuration-only ticket)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Update `packages/maproom-mcp/package.json` to reflect the simplified architecture: bump version to 3.0.0, remove unused dependencies, simplify the files array, and remove obsolete scripts.

## Background
The MCP server has been simplified from a Docker orchestration tool to a single-purpose MCP server. The package.json needs to reflect this change with a major version bump (breaking change), removal of dependencies that were only needed for orchestration, and cleanup of scripts/files configuration. This implements Phase 1.3 of the MCP Server Simplification plan.

## Acceptance Criteria
- [ ] Version updated from `2.2.3` to `3.0.0`
- [ ] `chokidar` dependency removed (was used for file watching in orchestration)
- [ ] `files` array simplified to only include files that ship with the package
- [ ] `setup`, `scan`, `watch` scripts removed (if they exist)
- [ ] Description updated to reflect single-purpose MCP server
- [ ] `pnpm install` succeeds after changes

## Technical Requirements
- **Version change**: `"version": "2.2.3"` → `"version": "3.0.0"`
- **Remove dependency**: `chokidar` (check devDependencies and dependencies)
- **Update files array**: Remove references to deleted config files:
  - Remove `config/docker-compose.yml`
  - Remove `config/Dockerfile.*`
  - Remove `config/init.sql`
  - Remove other deleted config files
- **Update description**: Should reflect "MCP server for semantic code search" rather than Docker orchestration
- **Remove scripts** (if present):
  - `setup` script
  - `scan` script
  - `watch` script

## Implementation Notes
- Read the current package.json first to understand its structure
- The `files` array determines what gets published to npm - only include:
  - `bin/` (cli.cjs)
  - `dist/` (compiled TypeScript)
  - `README.md`
  - `LICENSE` (if exists)
- Keep all existing dependencies that are still needed (database, MCP SDK, etc.)
- Run `pnpm install` to verify lockfile updates correctly

## Dependencies
- **MCPSIMP-1002** (Delete Unused Files) - should be completed first so we can verify files array accuracy

## Risk Assessment
- **Risk**: Removing a dependency that's still needed
  - **Mitigation**: Only remove `chokidar` which was for file watching in orchestration; verify no imports remain
- **Risk**: Breaking npm publish with incorrect files array
  - **Mitigation**: Test with `npm pack --dry-run` to see what would be included

## Files/Packages Affected
- `packages/maproom-mcp/package.json` (modify)
- `pnpm-lock.yaml` (auto-updated by pnpm install)

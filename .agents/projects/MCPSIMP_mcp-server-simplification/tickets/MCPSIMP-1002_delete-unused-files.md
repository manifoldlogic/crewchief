# Ticket: MCPSIMP-1002: Delete Unused Files

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - Tests pass - N/A (deletion-only ticket)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Delete all source files, config files, and test files that are no longer needed after the CLI simplification. This removes ~1,920 lines of Docker orchestration code.

## Background
With the CLI entry point simplified (MCPSIMP-1001), numerous files are now unused and can be safely deleted. This includes the config manager, Docker detection utilities, Docker compose files, and related tests. This implements Phase 1.2 of the MCP Server Simplification plan.

**PREREQUISITE**: MCPSIMP-1001 (Replace CLI Entry Point) must be completed first. The old cli.cjs imports from files being deleted here.

## Acceptance Criteria
- [ ] All source files listed below are deleted
- [ ] All config files listed below are deleted
- [ ] Test file `workspace-path-detection.test.ts` is deleted
- [ ] Package builds successfully (`pnpm build` in maproom-mcp)
- [ ] No import errors when running the new CLI

## Technical Requirements

**Source files to delete** (no longer imported after cli.cjs replacement):
- `packages/maproom-mcp/src/config-manager.ts`
- `packages/maproom-mcp/src/utils/docker-detection.ts`

**Config files to delete** (orchestration removed):
- `packages/maproom-mcp/config/docker-compose.yml`
- `packages/maproom-mcp/config/Dockerfile.mcp-server`
- `packages/maproom-mcp/config/Dockerfile.combined`
- `packages/maproom-mcp/config/Dockerfile.maproom`
- `packages/maproom-mcp/config/init.sql`
- `packages/maproom-mcp/config/docker-compose.override.yml`
- `packages/maproom-mcp/config/docker-compose.env.example`
- `packages/maproom-mcp/config/docker-compose.test.yml`
- `packages/maproom-mcp/config/postgresql.conf`
- `packages/maproom-mcp/config/devcontainer-network-fix.sh`
- `packages/maproom-mcp/config/DEVCONTAINER_NETWORKING.md`

**Test files to delete** (reference deleted modules):
- `packages/maproom-mcp/tests/utils/workspace-path-detection.test.ts`

## Implementation Notes
- Before deleting, verify MCPSIMP-1001 is complete and the new cli.cjs is in place
- Delete files in any order (all are unused after cli.cjs replacement)
- After deletion, run `pnpm build` in maproom-mcp to verify no compilation errors
- If any files don't exist, skip them (they may have been deleted previously)
- The `config/` directory may be completely empty after this; can delete the directory itself

## Dependencies
- **MCPSIMP-1001** (Replace CLI Entry Point) - MUST be completed first

## Risk Assessment
- **Risk**: Accidentally deleting files still in use
  - **Mitigation**: All files verified as unused by checking imports in new cli.cjs
- **Risk**: Tests fail due to missing modules
  - **Mitigation**: Deleting `workspace-path-detection.test.ts` removes the test that would fail

## Files/Packages Affected
- `packages/maproom-mcp/src/config-manager.ts` (delete)
- `packages/maproom-mcp/src/utils/docker-detection.ts` (delete)
- `packages/maproom-mcp/config/` (delete all listed files)
- `packages/maproom-mcp/tests/utils/workspace-path-detection.test.ts` (delete)

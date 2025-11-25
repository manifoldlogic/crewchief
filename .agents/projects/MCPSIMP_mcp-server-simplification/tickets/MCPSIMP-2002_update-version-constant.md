# Ticket: MCPSIMP-2002: Update Version Constant

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - Tests pass - N/A (constant update only)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Update the `MAPROOM_MCP_VERSION` constant in the VSCode extension to `3.0.0` to match the new MCP server version.

## Background
The VSCode extension references the MCP server version in `packages/vscode-maproom/src/constants.ts`. This constant is used when the extension installs or references the MCP server package. With the MCP server being updated to v3.0.0 (breaking change), this constant must be updated to ensure compatibility. This implements Phase 2.2 of the MCP Server Simplification plan.

## Acceptance Criteria
- [ ] `MAPROOM_MCP_VERSION` constant updated from `'2.2.3'` to `'3.0.0'`
- [ ] Extension builds successfully (`pnpm build` in vscode-maproom)
- [ ] No other version references need updating in the extension

## Technical Requirements
- Locate and update: `packages/vscode-maproom/src/constants.ts`
- Change: `MAPROOM_MCP_VERSION = '2.2.3'` → `MAPROOM_MCP_VERSION = '3.0.0'`
- Search for any other hardcoded version references that might need updating

## Implementation Notes
- This is a simple string replacement
- Verify the constant exists and is used as expected by searching for its usage
- The version should match exactly what will be published to npm in Phase 4
- This ticket should be done AFTER Phase 1 is complete and the MCP package is ready

## Dependencies
- **MCPSIMP-1003** (Update Package.json) - Version should match MCP package version

## Risk Assessment
- **Risk**: Version mismatch between extension and published package
  - **Mitigation**: Coordinate publishing sequence per plan.md - publish MCP package first, then update extension
- **Risk**: Other version references exist that aren't updated
  - **Mitigation**: Search codebase for "2.2.3" and "maproom-mcp" to find all references

## Files/Packages Affected
- `packages/vscode-maproom/src/constants.ts` (modify)

# Ticket: AGENTOPT-1002: Implement Enhanced Tool Description in MCP Server

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Update the MCP server code to replace the current tool description with the empirically-validated enhanced description from AGENTOPT-1001.

## Background
This ticket implements Phase 1, Step 2 from the AGENTOPT project plan (planning/plan.md lines 52-69). After designing the enhanced description, this step integrates it into the production MCP server. This is a simple string replacement with validation checks.

## Acceptance Criteria
- [ ] Tool description updated in packages/maproom-mcp/src/index.ts (lines 117-155)
- [ ] Token count validated (<600 tokens)
- [ ] MCP schema validation passes
- [ ] Local build successful (pnpm build)
- [ ] Manual smoke test with sample queries

## Technical Requirements
- Replace string literal in packages/maproom-mcp/src/index.ts
- Maintain exact MCP tool schema structure:
  ```typescript
  {
    name: 'search',
    description: `[enhanced description here]`,
    inputSchema: { ... } // unchanged
  }
  ```
- No logic changes, only description string
- Token count check using tiktoken or similar
- MCP schema validation using existing validation tools

## Implementation Notes
1. Read enhanced description from AGENTOPT-1001 output
2. Open packages/maproom-mcp/src/index.ts
3. Locate tool description (lines 117-155)
4. Replace with enhanced description
5. Run token counter to verify <600 tokens
6. Run MCP schema validator
7. Build locally: `cd packages/maproom-mcp && pnpm build`
8. Test manually with 2-3 sample queries to verify MCP tool still works

## Dependencies
- AGENTOPT-1001 (enhanced description draft)

## Risk Assessment
- **Risk**: MCP schema becomes invalid
  - **Mitigation**: Automated schema validation before commit
- **Risk**: Description breaks MCP tool discovery
  - **Mitigation**: Manual testing with Claude Code before deployment

## Files/Packages Affected
- packages/maproom-mcp/src/index.ts (lines 117-155)

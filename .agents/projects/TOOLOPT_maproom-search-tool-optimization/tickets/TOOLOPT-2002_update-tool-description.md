# Ticket: TOOLOPT-2002: Update MCP tool description with variant-a-detailed

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Replace current maproom search tool description with variant-a-detailed content in MCP server source code.

## Background
variant-a-detailed demonstrated 19.6% performance vs 17.7% baseline through systematic transformation workflow teaching. Deploy this proven winner to production MCP server.

This ticket is part of Phase 2 (Production Deployment) of the TOOLOPT project, executing the core code change to deploy the optimized tool description.

## Acceptance Criteria
- [ ] `packages/maproom-mcp/src/tools/search.ts` updated with new description
- [ ] Description field replaced with variant-a-detailed content
- [ ] TypeScript compilation succeeds without errors
- [ ] `pnpm build` completes successfully
- [ ] No API changes (only description content)
- [ ] Git diff shows only description field change

## Technical Requirements
- File: `/workspace/packages/maproom-mcp/src/tools/search.ts`
- Source: `/workspace/packages/cli/.crewchief/genetic-iterations/ultra-run-1763154816350/variants/variant-a-detailed.json`
- Read JSON, extract description field, replace in TypeScript
- Maintain proper string escaping for multiline description
- No changes to tool parameters or implementation
- Build command: `cd /workspace/packages/maproom-mcp && pnpm build`

## Implementation Notes
- Locate current description in search.ts
- Extract description from variant-a-detailed.json
- Handle multiline string formatting (use template literals)
- Verify no unintended changes to tool schema
- Build verification ensures no syntax errors
- Only the description text should change - no other modifications to the tool definition

## Dependencies
- TOOLOPT-2001 (validation confirms deployment readiness)

## Risk Assessment
- **Risk**: String escaping issues causing syntax errors
  - **Mitigation**: Use template literals for multiline strings, verify build succeeds
- **Risk**: Build failures after update
  - **Mitigation**: Test compilation before committing, use `pnpm build` to verify

## Files/Packages Affected
- `/workspace/packages/maproom-mcp/src/tools/search.ts` (tool definition file)
- `/workspace/packages/maproom-mcp/dist/` (compiled output)

## Estimated Time
15 minutes

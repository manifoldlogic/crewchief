# Ticket: TOOLOPT-2003: Integration test MCP server with new description

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
Start MCP server with updated tool description and verify agents can successfully use the search tool with expected quality.

## Background
After code change, need to confirm MCP server starts correctly, tool description is properly exposed, and agents can interact with the tool successfully.

This ticket is part of Phase 2 (Production Deployment) of the TOOLOPT project, providing integration testing before PR creation.

## Acceptance Criteria
- [ ] MCP server starts without errors
- [ ] Tool list includes maproom search with new description
- [ ] Description matches variant-a-detailed content
- [ ] 2-3 sample agent searches complete successfully
- [ ] Search results quality meets expectations
- [ ] No regressions in tool functionality

## Technical Requirements
- Start server:
  ```bash
  cd /workspace/packages/maproom-mcp
  node dist/index.js
  ```
- Verify server startup logs
- Check tool description in tool list response
- Execute test searches via spawned agent or MCP inspector
- Confirm search→results flow works end-to-end

## Implementation Notes
- Use MCP inspector or spawn test agent
- Sample searches to test:
  - "Find worktree creation implementation"
  - "Search for message bus"
  - "Locate git operations"
- Verify agents follow transformation workflow
- Check for any error patterns
- Spot-check result relevance
- Document any unexpected behavior
- Verify tool description is properly formatted and readable

## Dependencies
- TOOLOPT-2002 (code updated and built)

## Risk Assessment
- **Risk**: Server startup issues
  - **Mitigation**: Check logs, verify build completed successfully
- **Risk**: Tool not exposed properly
  - **Mitigation**: Verify MCP server config, check tool list response
- **Risk**: Search quality regression
  - **Mitigation**: Compare with pre-change behavior, check result relevance

## Files/Packages Affected
- `/workspace/packages/maproom-mcp/dist/index.js` (server entry point)
- MCP server runtime configuration
- Test search results (ephemeral)

## Estimated Time
20 minutes

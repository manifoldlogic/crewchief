# Ticket: AGENTOPT-1005: Deploy Enhanced Description to Production

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
Deploy the enhanced tool description to production MCP server with proper versioning, smoke testing, and monitoring.

## Background
This ticket implements Phase 1, Step 5 from the AGENTOPT project plan (planning/plan.md lines 108-123). After code review approval from AGENTOPT-1004, deploy the enhancement to production with proper deployment hygiene (git tagging, artifact building, verification, and smoke testing).

## Acceptance Criteria
- [ ] Git tag created (v1.X.0-agent-opt) with proper version increment
- [ ] Production artifacts built successfully
- [ ] MCP server deployed to production
- [ ] Server restart verified
- [ ] Smoke tests pass with 3-5 real queries

## Technical Requirements
- Git tagging convention: v1.X.0-agent-opt (increment from current version)
- Build process:
  ```bash
  cd packages/maproom-mcp
  pnpm install
  pnpm build
  ```
- Deployment: MCP server restart (or let Claude Code restart on next use)
- Smoke testing: 3-5 real queries to verify tool works correctly
- Rollback plan: Git revert + rebuild (target <5 minutes)

## Implementation Notes

1. **Create git tag**:
   ```bash
   git tag -a v1.X.0-agent-opt -m "Phase 1: Enhanced tool description for AI agent query optimization"
   git push origin v1.X.0-agent-opt
   ```
   - Replace X.0 with current version number (check existing tags)
   - Example: if latest is v1.2.0, use v1.3.0-agent-opt

2. **Build production artifacts**:
   ```bash
   cd packages/maproom-mcp
   pnpm install
   pnpm build
   ```
   - Verify no build errors
   - Check dist/ directory contains compiled output

3. **Deploy to production**:
   - Option A: Restart MCP server manually if running separately
   - Option B: Let Claude Code auto-restart on next connection
   - Verify maproom-mcp process is running

4. **Verify deployment**:
   ```bash
   # Check MCP server health (if available)
   curl http://localhost:3000/health

   # Verify tool discovery works
   # Test that maproom-mcp tool is accessible to Claude Code
   ```

5. **Run smoke tests**:
   - Execute 3-5 semantic search queries with Claude Code
   - Verify tool description matches enhanced version from AGENTOPT-1003
   - Confirm results returned successfully
   - Check that queries are using new description text

6. **Monitor post-deployment** (30 minutes):
   - Watch for any error logs
   - Verify no increased latency
   - Confirm stable operation

## Dependencies
- AGENTOPT-1004 (code review approval and merge)
- AGENTOPT-1003 (enhanced description implementation)

## Risk Assessment
- **Risk**: Deployment breaks MCP server connectivity
  - **Mitigation**: Keep previous version available, immediate rollback to HEAD~1

- **Risk**: Enhanced description not served to Claude Code
  - **Mitigation**: Verify tool discovery, check MCP server logs, restart if needed

- **Risk**: Production build fails
  - **Mitigation**: Test build locally first, review build logs for errors

## Files/Packages Affected
- Git tags (create v1.X.0-agent-opt)
- packages/maproom-mcp/ (production build)
- packages/maproom-mcp/dist/ (build output)
- packages/maproom-mcp/src/ (source deployed)

## Planning References
- Implementation Plan: planning/plan.md lines 108-123 (Step 5: Deployment)
- Rollback procedure: planning/plan.md lines 677-703 (if section exists)
- Architecture context: planning/architecture.md

# Ticket: TOOLOPT-2005: Deploy approved changes to production

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
Merge approved PR, rebuild packages, restart MCP server, and verify deployment successful with post-deployment monitoring.

## Background
After PR approval, execute production deployment and confirm new tool description is live and functioning correctly.

This ticket is the final step of Phase 2 (Production Deployment) of the TOOLOPT project, completing the deployment cycle.

## Acceptance Criteria
- [ ] PR merged to main branch
- [ ] Packages rebuilt (`pnpm build` in maproom-mcp)
- [ ] MCP server restarted with new build
- [ ] Deployment verification completed:
  - [ ] Server started successfully
  - [ ] Tool description matches variant-a-detailed
  - [ ] Sample search executes correctly
- [ ] Post-deployment spot check passes
- [ ] No error rate increase observed

## Technical Requirements
- Merge PR (requires approval)
- Build commands:
  ```bash
  cd /workspace/packages/maproom-mcp
  pnpm build
  ```
- Restart MCP server (method depends on deployment)
- Verification:
  - Check server logs
  - Execute test search
  - Monitor for errors
- Document deployment timestamp

## Implementation Notes
- Coordinate restart timing if needed
- Keep backup of previous version
- Monitor logs for first 5-10 minutes
- Run quick smoke test (simple search)
- Document deployment timestamp
- Verify no regression in search functionality
- Confirm tool description is properly displayed

## Dependencies
- TOOLOPT-2004 (PR approved and ready to merge)

## Risk Assessment
- **Risk**: Production issues after deployment
  - **Mitigation**: Have rollback plan ready (see below)
- **Risk**: Service interruption during restart
  - **Mitigation**: Quick restart, minimal downtime
- **Risk**: Unexpected errors in production
  - **Mitigation**: Monitor logs, rollback if critical issues found

## Rollback Plan
```bash
git revert <commit-sha>
cd /workspace/packages/maproom-mcp
pnpm build
# Restart MCP server
```

## Files/Packages Affected
- `/workspace/packages/maproom-mcp/dist/` (production build)
- MCP server runtime (restarted)
- Production environment

## Estimated Time
15 minutes

## Post-Deployment Monitoring
- Monitor for 10-15 minutes after deployment
- Check error logs
- Verify search quality with sample queries
- Document any issues or anomalies

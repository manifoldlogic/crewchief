# Ticket: DINDFX-4001: Manual testing in devcontainer and host environments

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Manually verify the Docker workspace path detection solution works correctly in real devcontainer and host environments by running systematic manual tests of the setup command and MCP tools.

## Background
After all automated tests pass (Phases 1-3), we need to manually verify the solution works in actual environments. This phase focuses on the CRITICAL devcontainer scenario (the primary problem we're solving) and the important host scenario.

The devcontainer environment is where users were experiencing issues with workspace path detection, requiring manual configuration. This manual testing phase validates that the automated detection solution eliminates that pain point.

Optional tests for user override and error cases can be deferred to post-MVP as they are secondary scenarios.

**References**:
- `.agents/projects/DINDFX_docker-workspace-path-detection/planning/plan.md` - Phase 4
- `.agents/projects/DINDFX_docker-workspace-path-detection/planning/quality-strategy.md` - Manual testing checklist

## Acceptance Criteria
- [ ] MUST PASS: Setup works in devcontainer without manual config (Test 4.1)
- [ ] MUST PASS: Console shows correct host path in devcontainer (e.g., `✓ Workspace path: /host_mnt/...`)
- [ ] MUST PASS: maproom-mcp container can access workspace files
- [ ] MUST PASS: MCP tools (`mcp__maproom__open`, `mcp__maproom__context`) work after setup
- [ ] MUST PASS: Setup works on host machine (Test 4.2)
- [ ] MUST PASS: No errors or crashes during normal operation
- [ ] Document any issues found for follow-up

## Technical Requirements

### Test 4.1: Devcontainer Environment (CRITICAL - Must Pass)
1. Start fresh devcontainer
2. Destroy existing maproom containers: `docker compose down -v`
3. Run `npx @crewchief/maproom-mcp setup --provider=openai`
4. Verify console shows: `✓ Workspace path: /host_mnt/...` (actual host path)
5. Verify containers started successfully: `docker ps | grep maproom`
6. Verify volume mount: `docker inspect maproom-mcp | grep workspace`
7. Verify file access: `docker exec maproom-mcp ls /workspace/packages`
8. Reconnect to MCP: `/mcp` command
9. Test open tool: `mcp__maproom__open` with real file from workspace
10. Test context tool: `mcp__maproom__context` with real chunk

### Test 4.2: Host Environment (Important)
1. Exit devcontainer (run on host machine)
2. Run `npx @crewchief/maproom-mcp setup --provider=openai`
3. Verify console shows: `✓ Workspace path: /Users/.../crewchief` (current directory)
4. Verify containers started successfully
5. Verify volume mount points to current directory
6. Verify file access works

### Test Requirements
- Test in actual devcontainer environment (not simulated)
- Use fresh container state (destroy existing containers first)
- Use real MCP connection and tools
- Document exact commands and output
- Capture any error messages or unexpected behavior

## Implementation Notes

**Testing Approach:**
- Focus on CRITICAL path: devcontainer setup and file access
- Test 4.1 is the PRIMARY scenario this project solves
- Test 4.2 ensures we didn't break host execution
- Optional tests (4.3, 4.4) can be deferred - user override and error handling are secondary
- If Test 4.1 passes, the project is successful

**Expected Outcomes:**
- Devcontainer should automatically detect host workspace path via Docker inspection
- Host should use current working directory
- Volume mounts should work transparently in both environments
- MCP tools should have full file access

**Documentation:**
- Record all console output
- Document any unexpected behavior
- Note any edge cases discovered
- Create follow-up tickets if issues found

**Optional Tests (Post-MVP - Can Skip for Now):**
- Test 4.3: User override with custom WORKSPACE_HOST_PATH
- Test 4.4: Error cases (docker inspect failure, etc.)

## Dependencies
- **DINDFX-3001** must be complete (all automated tests passing)
- Must have access to devcontainer environment
- Must have Docker Desktop or Docker Engine running
- Must have MCP tools available

## Risk Assessment
- **Risk**: Environment-specific issues not caught by automated tests
  - **Mitigation**: Test in actual target environments (devcontainer and host)

- **Risk**: Docker socket permissions issues
  - **Mitigation**: Devcontainer already has Docker socket access configured

- **Risk**: Unexpected edge cases discovered during manual testing
  - **Mitigation**: Acceptable - document for follow-up tickets if needed

- **Risk**: Time-consuming debugging if issues found
  - **Mitigation**: Expected and planned for - 4-5 hours allocated

## Files/Packages Affected
**No code changes expected** - this is a validation phase.

**Files to Review:**
- Console output during setup
- `docker ps` output
- `docker inspect maproom-mcp` output
- MCP tool responses

**Estimated Effort:** 4-5 hours (realistic, includes debugging time if issues found)

# Ticket: DINDFX-2004: Integrate resolveWorkspacePath into runSetup()

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
Integrate the `resolveWorkspacePath()` function into the `runSetup()` flow by calling it after `setupConfigDirectory()` and before `startDockerCompose()`. This ensures the `WORKSPACE_HOST_PATH` environment variable is set before Docker Compose spawns, enabling correct volume mounting.

## Background
This ticket implements Phase 2 Step 2.4 from the DINDFX project plan. After implementing the three detection/resolution functions (DINDFX-2001, DINDFX-2002, DINDFX-2003), we now integrate them into the actual setup flow. The key is to call `resolveWorkspacePath()` BEFORE `startDockerCompose()` so that `WORKSPACE_HOST_PATH` is set in the environment when docker compose spawns. This is the final piece that makes the solution work end-to-end, ensuring the workspace path is correctly detected and propagated to the Docker container for volume mounting.

**Plan Reference**: `.agents/projects/DINDFX_docker-workspace-path-detection/planning/plan.md` - Phase 2 Step 2.4: Integrate with Setup Flow

## Acceptance Criteria
- [ ] `runSetup()` function modified to call `resolveWorkspacePath()`
- [ ] Call happens AFTER `setupConfigDirectory()` (docker-compose.yml is in place)
- [ ] Call happens BEFORE `startDockerCompose()` (env var set before spawn)
- [ ] `process.env.WORKSPACE_HOST_PATH` is set with resolved path
- [ ] Console shows: `✓ Workspace path: <path>` for user feedback
- [ ] Environment variable propagates to docker compose spawn (spread in `env`)
- [ ] All 3 integration tests pass
- [ ] Verification: `pnpm test integration/workspace-path-detection` shows 3/3 passing
- [ ] Verification: All 18 tests (15 unit + 3 integration) pass

## Technical Requirements
- Modify `runSetup()` function in `packages/maproom-mcp/bin/cli.cjs`
- Insert code after `setupConfigDirectory()` call (around line ~1788)
- Insert code before `await startDockerCompose()` call
- Call `resolveWorkspacePath()` to get the workspace path
- Set `process.env.WORKSPACE_HOST_PATH = workspacePath`
- Add console output: `console.error('✓ Workspace path:', workspacePath)`
- Use `console.error()` not `console.log()` to avoid polluting stdout (follows existing pattern)
- Ensure environment variable propagates to `startDockerCompose()` child process

## Implementation Notes

### Code to Add
```javascript
async function runSetup() {
  // ... existing code ...

  // Copy configs
  setupConfigDirectory();

  // 🆕 NEW: Detect and set workspace path for Docker volume mounting
  const workspacePath = resolveWorkspacePath();
  process.env.WORKSPACE_HOST_PATH = workspacePath;

  console.error('✓ Workspace path:', workspacePath);

  // Start Docker Compose (respects WORKSPACE_HOST_PATH)
  await startDockerCompose();

  // ... rest of setup ...
}
```

### Insertion Point
- **File**: `packages/maproom-mcp/bin/cli.cjs`
- **Function**: `runSetup()`
- **Location**: Around line ~1788, after `setupConfigDirectory()`, before `await startDockerCompose()`

### Key Details
- Setting `process.env.WORKSPACE_HOST_PATH` makes it available to child processes
- `startDockerCompose()` already spreads `process.env` into spawn env: `{ env: { ...process.env } }`
- docker-compose.yml will expand `${WORKSPACE_HOST_PATH}` to the set value
- Only 3 lines of new code required
- No other changes to existing setup flow

### Architecture References
- See `planning/architecture.md` Component Design section 4 (Integration Point)
- See `planning/architecture.md` Data Flow section for complete flow
- See `planning/quality-strategy.md` for integration test suite details

## Dependencies
- **DINDFX-1001** must be complete (integration tests written and failing)
- **DINDFX-2001** must be complete (isInsideDocker implemented)
- **DINDFX-2002** must be complete (getWorkspaceHostPath implemented)
- **DINDFX-2003** must be complete (resolveWorkspacePath implemented)

## Risk Assessment
- **Risk**: Insertion point incorrect (line number approximate)
  - **Mitigation**: Look for setupConfigDirectory call, insert immediately after

- **Risk**: Environment variable not propagating to Docker Compose
  - **Mitigation**: Existing pattern already spreads process.env in startDockerCompose

- **Risk**: Breaking existing setup flow
  - **Mitigation**: Only adds 3 lines, no other changes; integration tests verify end-to-end

## Files/Packages Affected
- `packages/maproom-mcp/bin/cli.cjs` (modify runSetup function)

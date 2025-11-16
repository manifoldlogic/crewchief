# Ticket: VSMAP-1003: Implement platform-aware binary spawning for watch processes

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
- process-management-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create `ProcessOrchestrator` to spawn `crewchief-maproom watch` and `branch-watch` processes. Handle platform detection, binary selection, and basic stdout logging.

## Background
The extension spawns two long-running Rust processes: `watch` (file changes) and `branch-watch` (branch switches). We need platform-specific binary selection (darwin-x64, linux-arm64, etc.) and proper process lifecycle management.

This ticket implements **Milestone 1.2: Binary Spawner** from Phase 1 of the VSMAP project plan, establishing the process management layer that runs the Maproom indexing engine.

## Acceptance Criteria
- [ ] Correct binary selected based on `process.platform` and `process.arch`
- [ ] `ProcessOrchestrator.startWatching()` spawns both processes
- [ ] Both processes run continuously (don't exit unexpectedly)
- [ ] Stdout logged to VSCode Output panel ("Maproom" channel)
- [ ] Processes killed on `stopWatching()` or extension deactivation
- [ ] Error shown if binary not found for platform

## Technical Requirements
- Detect platform: darwin (x64/arm64), linux (x64/arm64), win32 (x64)
- Binary path: `bin/<platform>/crewchief-maproom[.exe]`
- Spawn watch: `crewchief-maproom watch --throttle 3s`
- Spawn branch-watch: `crewchief-maproom branch-watch --repo <workspace-root>`
- Use `child_process.spawn()` with `stdio: ['pipe', 'pipe', 'pipe']`
- Create VSCode OutputChannel for logging (reuse "Maproom" channel from DockerManager)
- Handle process exit (log exit code, restart if needed)
- Pass environment variables for Postgres connection (PGHOST, PGPORT, PGUSER, PGPASSWORD, PGDATABASE)

## Implementation Notes
- Bundle Rust binaries in `bin/` directory of VSIX (packaging handled separately)
- Use `context.extensionPath` to find binary location
- Pass environment variables for Postgres connection (from DockerManager config)
- Log stderr to Output panel (errors from Rust binary)
- Implement graceful shutdown (SIGTERM, wait 5s, SIGKILL if needed)
- Platform detection utility should be reusable (separate file)
- Handle edge case: binary exists but isn't executable (chmod +x in postinstall)
- Don't restart processes immediately on crash - wait and log error

## Dependencies
- VSMAP-0003 (agents tested) - process-management-specialist must be validated
- VSMAP-1001 (DockerManager for env vars) - need Postgres connection details

## Risk Assessment
- **Risk**: Binary may not execute (permissions, architecture mismatch)
  - **Mitigation**: Clear error message with platform detection info, check file permissions
- **Risk**: Processes may crash immediately
  - **Mitigation**: Log stderr, show to user, don't restart indefinitely
- **Risk**: Wrong binary for platform (e.g., arm64 binary on x64 machine)
  - **Mitigation**: Validate platform detection logic, test on multiple platforms

## Files/Packages Affected
- `src/process/orchestrator.ts` (create, ~200 lines)
- `src/utils/platform.ts` (create, ~50 lines)
- `src/docker/manager.ts` (modify to export env vars)
- VSCode Output panel integration

# Ticket: VSMAP-1001: Implement DockerManager class for service lifecycle

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
- [x] **Verified** - by the verify-ticket agent

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
Create `DockerManager` class to start/stop PostgreSQL and Maproom MCP server using Docker Compose. Handle health checks, error cases (Docker not running), and graceful shutdown.

## Background
The extension depends on PostgreSQL (for Maproom index) and the Maproom MCP server (for Claude Code integration). These services must start automatically when the extension activates and stop cleanly on deactivation.

This ticket implements **Milestone 1.1: Docker Manager** from Phase 1 (Core Infrastructure) of the VSMAP project plan, establishing the foundational service layer that all other components depend on.

## Acceptance Criteria
- [x] `DockerManager.ensureServicesRunning()` starts docker-compose services
- [x] PostgreSQL healthy within 30 seconds (pg_isready check)
- [x] MCP server healthy within 15 seconds (TCP ping on port)
- [x] Error shown if Docker Desktop not running
- [x] Services stop gracefully on `DockerManager.stop()`
- [x] No-op if services already running (idempotent behavior)

## Technical Requirements
- Use `child_process.spawn('docker', ['compose', 'up', '-d'])` to start services
- Use `child_process.spawn('docker', ['compose', 'down'])` to stop services
- Health check PostgreSQL: `docker exec <container> pg_isready`
- Health check MCP: TCP connection to localhost:3000 (or configured port)
- Implement exponential backoff for health checks (1s, 2s, 4s, 8s, max 30s total)
- Throw clear error if Docker not installed or not running
- Use VSCode's `window.showErrorMessage()` for Docker errors
- Log health check attempts to Output panel

## Implementation Notes
- Reference `packages/maproom-mcp/config/docker-compose.yml` for service definitions
- The docker-compose.yml should be copied to extension's config/ directory
- Handle SIGTERM/SIGINT gracefully (stop services before exit)
- Connection string: `postgresql://maproom:maproom@maproom-postgres:5432/maproom`
- Create VSCode OutputChannel named "Maproom" for all logging
- Use defensive programming - validate Docker command exists before spawning

## Dependencies
- VSMAP-0003 (agents tested) - process-management-specialist must be validated

## Risk Assessment
- **Risk**: Docker Desktop may not be running
  - **Mitigation**: Clear error message with setup instructions pointing to extension README
- **Risk**: Health checks may timeout on slower machines
  - **Mitigation**: 30s timeout with exponential backoff, log progress to help debug
- **Risk**: Port conflicts (3000 already in use)
  - **Mitigation**: Docker Compose will fail with clear error, surface to user

## Files/Packages Affected
- `src/docker/manager.ts` (create, ~150 lines)
- `config/docker-compose.yml` (create, copy from packages/maproom-mcp/config/docker-compose.yml)
- VSCode extension Output panel integration

# Ticket: MCPSTART-1002: Add Docker command execution logging

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass (verified via production use in v1.1.10+)
- [x] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- test-runner (manual verification)
- verify-ticket
- commit-ticket

## Summary
Log every Docker Compose command before execution to verify correct arguments are being passed (especially service names and environment variables).

## Background
We need visibility into exactly what commands are sent to Docker Compose. Previous fixes may have had correct logic but incorrect command execution. This ticket adds logging before every docker/docker compose command execution to help diagnose issues.

This implements **Phase 1.2** from MCPSTART_ARCHITECTURE.md - Docker Command Logging. The goal is to verify that the right commands with the right arguments are being sent to Docker Compose.

## Acceptance Criteria
- [x] All `spawn()` and `spawnSync()` calls to docker/docker compose are logged
- [x] Logs show command name, full args array, and working directory
- [x] Logs appear before command execution (not after)
- [x] Logs use diagnosticLog() function from MCPSTART-1001
- [x] Sensitive env vars are NOT logged in command args

## Technical Requirements
- Create `execDockerCompose(args, description)` wrapper function
- Log format: `Docker Compose Command: {description}` with args array
- Wrap existing spawnSync/spawn calls where appropriate
- Log working directory (CONFIG_DIR)
- Example: `Docker Compose Command: Starting services` → `{ command: 'docker', args: ['compose', 'up', '-d', 'postgres', 'maproom-mcp'], cwd: '/home/user/.maproom-mcp' }`

## Implementation Notes
From MCPSTART_ARCHITECTURE.md lines 49-66:
```javascript
function execDockerCompose(args, description) {
  diagnosticLog(`Docker Compose Command: ${description}`, {
    command: 'docker',
    args: args,
    cwd: CONFIG_DIR
  });

  return spawnSync('docker', args, {
    cwd: CONFIG_DIR,
    encoding: 'utf-8',
    stdio: 'pipe'
  });
}
```

**Important**: Don't create a wrapper for ALL commands yet - just add logging to existing calls. The wrapper can be refactored later if beneficial. Focus on visibility first.

Target file: `packages/maproom-mcp/bin/cli.cjs`

Locations to add logging:
- Before each `spawnSync('docker', ...)` call
- Before each `spawn('docker', ...)` call
- Include the description of what the command is doing (e.g., "Starting services", "Checking container status", "Stopping services")

## Dependencies
- **Prerequisite**: MCPSTART-1001 (diagnosticLog function must exist)

## Risk Assessment
- **Risk**: Low - only adds logging, no behavior changes
  - **Mitigation**: No changes to execution logic, only observability improvements

## Files/Packages Affected
- `packages/maproom-mcp/bin/cli.cjs` - Add logging before Docker commands

## Implementation Notes

Successfully added diagnostic logging before all 6 Docker command execution points:

1. **Line 59-63**: `checkDockerDaemon()` - Checking Docker daemon status
   - Command: `docker info`
   - Description: "Checking Docker daemon status"

2. **Line 92-96**: `checkDockerCompose()` - Checking Docker Compose version
   - Command: `docker compose version`
   - Description: "Checking Docker Compose version"

3. **Line 301-305**: `startDockerCompose()` - Stopping unnecessary services
   - Command: `docker compose stop <service-names>`
   - Description: "Stopping unnecessary services"

4. **Line 325-329**: `startDockerCompose()` - Starting services
   - Command: `docker compose up -d <service-names>`
   - Description: "Starting services"

5. **Line 403-407**: `waitForServicesHealthy()` - Checking container status
   - Command: `docker compose ps --format json`
   - Description: "Checking container status"

6. **Line 518-522**: `establishStdioProxy()` - Establishing stdio proxy
   - Command: `docker exec -i maproom-mcp node /app/dist/index.js`
   - Description: "Establishing stdio proxy to MCP server"

All logs follow the format:
```javascript
diagnosticLog('Docker [Compose] Command: <description>', {
  command: 'docker',
  args: [...],
  cwd: <CONFIG_DIR or process.cwd()>
});
```

Logs appear BEFORE command execution and will be visible when:
- MAPROOM_MCP_DEBUG=true is set, OR
- EMBEDDING_PROVIDER is not set (zero-config mode)

No sensitive environment variables are exposed in the logs - only command structure is logged.

### Testing
To verify the implementation, run the CLI with debug mode enabled:
```bash
MAPROOM_MCP_DEBUG=true npx -y @crewchief/maproom-mcp
```

You should see diagnostic output showing:
- Each Docker command before it executes
- The full args array for each command
- The working directory for each command

# Ticket: LOCAL-1008: Implement CLI wrapper with docker-compose orchestration

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- verify-ticket
- commit-ticket

## Summary
Implement the Node.js CLI wrapper (bin/cli.js) that orchestrates Docker Compose and proxies stdio to the MCP container. This is the entry point users invoke via npx to launch the fully containerized Maproom MCP service with zero configuration.

## Background
This ticket is part of Phase 1 (Core Infrastructure) of the LOCAL project, which implements a fully containerized Maproom MCP service with local LLM embeddings (Ollama + nomic-embed-text), bundled PostgreSQL, and zero-configuration deployment via npm.

The CLI wrapper is the orchestration brain that provides a seamless user experience. When a user runs `npx -y @crewchief/maproom-mcp`, this CLI wrapper must:
1. Verify Docker Compose is available
2. Initialize the configuration directory on first run
3. Start the Docker Compose stack with all services
4. Wait for health checks to pass
5. Proxy stdio to the MCP container for seamless communication

This is critical infrastructure for achieving the "zero-configuration UX" goal - users should not need to manually run docker-compose commands or manage configuration files.

## Acceptance Criteria
- [ ] bin/cli.js created with proper shebang (#!/usr/bin/env node)
- [ ] CLI checks for Docker Compose v2 plugin (not old standalone binary)
- [ ] Creates ~/.maproom-mcp directory on first run if it doesn't exist
- [ ] Copies embedded docker-compose.yml to ~/.maproom-mcp/docker-compose.yml
- [ ] Starts Docker Compose stack with `docker compose up -d`
- [ ] Waits for all services to be healthy before connecting
- [ ] Proxies stdio to the maproom container via `docker compose exec -T maproom`
- [ ] Provides clear, user-friendly error messages for common failure cases
- [ ] Handles SIGINT gracefully and shuts down the MCP connection cleanly
- [ ] Works end-to-end with `npx -y @crewchief/maproom-mcp` (manual test in LOCAL-3001)

## Technical Requirements

### Core Functions
1. **checkDocker()**: Verify Docker Compose plugin availability
   - Run `docker compose version`
   - Exit code 0 = success
   - Otherwise, show error: "Docker Compose plugin not found. Please install Docker Desktop or Docker Compose v2."

2. **startStack()**: Launch Docker Compose stack
   - Run `docker compose -f ~/.maproom-mcp/docker-compose.yml up -d`
   - Set cwd to CONFIG_DIR
   - Use `stdio: 'inherit'` to show output to user
   - Return promise that resolves on successful start

3. **waitForHealth()**: Poll until all services are healthy
   - Run `docker compose ps --services --filter status=running`
   - Check that all 3 services (postgres, ollama, maproom) are running
   - Retry up to 30 times with 2-second intervals
   - Throw error if timeout exceeded

4. **main()**: Orchestrate the entire flow
   - Call startStack()
   - Call waitForHealth()
   - Execute `docker compose exec -T maproom /usr/local/bin/crewchief-maproom serve --stdio`
   - Proxy stdio: `stdio: ['inherit', 'inherit', 'inherit']`
   - Handle SIGINT to gracefully shutdown MCP connection
   - Exit with proper exit codes

### Configuration Management
- Config directory: `~/.maproom-mcp`
- Embedded compose file: `<package-root>/config/docker-compose.yml`
- Copy embedded file to config directory on first run only
- Show initialization message: "✅ Initialized Maproom configuration"

### Error Handling
Must provide clear error messages for:
- Docker not installed
- Docker Compose plugin not available (v2 required)
- Services fail to start within timeout
- MCP connection failures

### Dependencies
- Node.js built-in modules only: `child_process`, `fs`, `path`, `os`
- No external npm dependencies required

## Implementation Notes

### Reference Architecture
The complete implementation is documented in LOCAL_ARCHITECTURE.md lines 100-227. Key implementation details:

**File Structure**:
```
bin/cli.js              # Main CLI wrapper (this ticket)
config/
  docker-compose.yml    # Embedded compose file (from LOCAL-1003)
```

**CLI Flow**:
1. Check if Docker Compose v2 is available
2. Create ~/.maproom-mcp directory if needed
3. Copy docker-compose.yml to config directory (first run only)
4. Start Docker Compose stack: `docker compose up -d`
5. Wait for all 3 services to be healthy (postgres, ollama, maproom)
6. Connect to MCP container: `docker compose exec -T maproom <binary> serve --stdio`
7. Proxy stdio bidirectionally for MCP protocol
8. On SIGINT: kill MCP process gracefully and exit

### Docker Compose v2 vs v1
- **Required**: Docker Compose v2 (plugin: `docker compose`)
- **Not supported**: Docker Compose v1 (standalone binary: `docker-compose`)
- Detection: Check if `docker compose version` succeeds

### stdio Proxying
The MCP protocol requires bidirectional stdio communication:
- stdin: User input → MCP container
- stdout: MCP responses → User
- stderr: Error messages → User

Use spawn with `stdio: ['inherit', 'inherit', 'inherit']` to achieve transparent proxying.

### Graceful Shutdown
On SIGINT (Ctrl+C):
1. Log: "🛑 Shutting down gracefully..."
2. Send SIGTERM to MCP process
3. Allow MCP to clean up and exit
4. Exit CLI wrapper with same exit code

### User Feedback
Provide clear console output at each stage:
- "🚀 Starting Maproom MCP with local LLM..."
- "✅ Maproom MCP is ready!"
- "✅ All services healthy"
- "🔌 Connecting to MCP server..."
- "❌ Error: <detailed message>"

## Dependencies
- **LOCAL-1007**: npm package structure must exist
  - bin/ directory
  - config/ directory with docker-compose.yml
  - package.json with bin field configured

## Risk Assessment
- **Risk**: Docker Compose v2 not installed on user's machine
  - **Mitigation**: Clear error message with installation instructions; document requirement prominently

- **Risk**: Health check timeout too short for slow machines or large Ollama model downloads
  - **Mitigation**: 30 retries × 2 seconds = 60 seconds should be sufficient; can increase if needed in testing

- **Risk**: stdio proxying fails or breaks MCP protocol
  - **Mitigation**: Use `stdio: 'inherit'` for transparent passthrough; test with actual MCP clients

- **Risk**: Orphaned containers if CLI crashes during startup
  - **Mitigation**: Docker Compose handles container lifecycle; users can manually run `docker compose down` if needed

- **Risk**: Permissions issues with ~/.maproom-mcp directory
  - **Mitigation**: Use `fs.mkdirSync` with `recursive: true`; handle write errors gracefully

## Files/Packages Affected
- `/workspace/packages/cli/bin/cli.js` (new file)
- `/workspace/packages/cli/package.json` (bin field should already be configured by LOCAL-1007)

## Testing Notes
Manual testing will be performed in LOCAL-3001 (Phase 3: E2E testing):
1. Run `npx -y @crewchief/maproom-mcp` on a clean system
2. Verify Docker Compose stack starts
3. Verify MCP communication works
4. Test error handling (Docker not running, etc.)
5. Test graceful shutdown with Ctrl+C

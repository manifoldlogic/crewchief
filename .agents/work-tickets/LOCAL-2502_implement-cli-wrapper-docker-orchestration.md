# Ticket: LOCAL-2502: Implement CLI Wrapper for Docker Orchestration and Stdio Proxy

## Status
- [x] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- mcp-tools-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement the complete CLI wrapper at `packages/maproom-mcp/bin/cli.js` that orchestrates Docker Compose (starting postgres, ollama, maproom-mcp services) and proxies stdin/stdout between the user and the containerized MCP server. This enables the `npx -y @crewchief/maproom-mcp` workflow for zero-configuration MCP server deployment.

## Background
Phase 2.5 bridges the containerized infrastructure (Phase 1-2) with the npm package distribution model (Phase 3). The CLI wrapper is the critical orchestration layer that:

1. Provides zero-configuration deployment via `npx`
2. Manages Docker Compose lifecycle (start, health checks, shutdown)
3. Proxies MCP JSON-RPC traffic between Claude/Cursor and the containerized server
4. Handles errors gracefully with user-friendly messages
5. Enables the target `.mcp.json` configuration:

```json
{
  "mcpServers": {
    "maproom": {
      "command": "npx",
      "args": ["-y", "@crewchief/maproom-mcp"]
    }
  }
}
```

When executed, the flow should be:
1. User adds `.mcp.json` configuration
2. Claude/Cursor runs `npx -y @crewchief/maproom-mcp`
3. npx downloads/runs the package
4. `bin/cli.js` checks Docker is available
5. Copies docker-compose.yml to `~/.maproom-mcp/` (if not exists)
6. Runs `docker compose up -d` to start services
7. Waits for health checks (postgres, ollama, maproom-mcp)
8. Establishes stdio proxy to `maproom-mcp` container
9. Proxies MCP JSON-RPC traffic bidirectionally
10. Handles SIGINT/SIGTERM for graceful shutdown

## Acceptance Criteria
- [ ] `packages/maproom-mcp/bin/cli.js` file created and executable
- [ ] CLI checks Docker daemon is running (error message if not)
- [ ] CLI checks Docker Compose v2 available (error message if not)
- [ ] Copies `docker-compose.yml` and supporting files to `~/.maproom-mcp/` on first run
- [ ] Runs `docker compose up -d` from `~/.maproom-mcp/` directory
- [ ] Waits for all services to become healthy (postgres, ollama, maproom-mcp)
- [ ] Shows user-friendly progress indicators (emoji + messages OK)
- [ ] Establishes bidirectional stdio proxy to `maproom-mcp` container
- [ ] Forwards stdin from user to container
- [ ] Forwards stdout from container to user
- [ ] Handles SIGINT (Ctrl+C) gracefully - stops proxy but leaves containers running
- [ ] Handles SIGTERM gracefully - stops proxy but leaves containers running
- [ ] Provides clear error messages for common failures:
  - Docker not running
  - Docker Compose not installed
  - Port conflicts (5432, 11434)
  - Health check timeouts
  - Container startup failures
- [ ] Exits with appropriate exit codes (0 = success, non-zero = error)
- [ ] Works on Linux, macOS, and Windows (where Docker is available)

## Technical Requirements

### File Location and Structure
- Path: `/workspace/packages/maproom-mcp/bin/cli.js`
- Must be executable: `chmod +x bin/cli.js`
- Shebang: `#!/usr/bin/env node`
- Runs on Node.js 18+

### Docker Availability Checks

```javascript
// Check 1: Docker daemon running
const dockerCheck = spawn('docker', ['info'], { stdio: 'pipe' })
dockerCheck.on('error', () => {
  console.error('ERROR: Docker is not running or not installed.')
  console.error('Please start Docker Desktop or install Docker.')
  process.exit(1)
})

// Check 2: Docker Compose v2 available
const composeCheck = spawn('docker', ['compose', 'version'], { stdio: 'pipe' })
composeCheck.on('error', () => {
  console.error('ERROR: Docker Compose v2 is not available.')
  console.error('Please update Docker to a version that includes Compose v2.')
  process.exit(1)
})
```

### Configuration Directory Setup
- Target directory: `~/.maproom-mcp/`
- Create if doesn't exist (first run)
- Copy files:
  - `docker-compose.yml` (from package)
  - `Dockerfile.mcp-server` (if packaged separately)
  - `Dockerfile.maproom` (if needed for future)
  - `init.sql` (PostgreSQL schema)
  - `.env.example` (optional)

```javascript
const fs = require('fs')
const path = require('path')
const os = require('os')

const configDir = path.join(os.homedir(), '.maproom-mcp')
if (!fs.existsSync(configDir)) {
  fs.mkdirSync(configDir, { recursive: true })

  // Copy docker-compose.yml from package
  const srcCompose = path.join(__dirname, '../config/docker-compose.yml')
  const dstCompose = path.join(configDir, 'docker-compose.yml')
  fs.copyFileSync(srcCompose, dstCompose)

  console.log('✓ Created configuration directory:', configDir)
}
```

### Docker Compose Orchestration

```javascript
const { spawn } = require('child_process')

// Start Docker Compose stack
const composeUp = spawn('docker', ['compose', 'up', '-d'], {
  cwd: configDir,
  stdio: 'inherit' // Show Docker output for troubleshooting
})

composeUp.on('exit', (code) => {
  if (code !== 0) {
    console.error(`ERROR: Docker Compose failed with exit code ${code}`)
    process.exit(1)
  }
  console.log('✓ Services started')
  waitForHealth()
})
```

### Health Check Waiting Logic

```javascript
async function waitForHealth() {
  const services = ['postgres', 'ollama', 'maproom-mcp']
  const maxWaitTime = 120000 // 2 minutes
  const checkInterval = 2000   // 2 seconds

  console.log('⏳ Waiting for services to become healthy...')

  const startTime = Date.now()

  while (Date.now() - startTime < maxWaitTime) {
    const ps = spawnSync('docker', ['compose', 'ps', '--format', 'json'], {
      cwd: configDir,
      encoding: 'utf-8'
    })

    if (ps.status === 0) {
      const containers = ps.stdout.split('\n')
        .filter(line => line.trim())
        .map(line => JSON.parse(line))

      const allHealthy = services.every(svc => {
        const container = containers.find(c => c.Service === svc)
        return container && (container.Health === 'healthy' || container.State === 'running')
      })

      if (allHealthy) {
        console.log('✓ All services are healthy')
        return true
      }
    }

    await sleep(checkInterval)
  }

  console.error('ERROR: Services did not become healthy within 2 minutes')
  console.error('Check logs with: docker compose logs -f')
  process.exit(1)
}

function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms))
}
```

### Stdio Proxy Implementation

```javascript
// Attach to maproom-mcp container's stdin/stdout
const proxy = spawn('docker', ['compose', 'exec', '-T', 'maproom-mcp', 'cat'], {
  cwd: configDir,
  stdio: ['pipe', 'pipe', 'inherit'] // stdin: pipe, stdout: pipe, stderr: inherit
})

// Forward user stdin to container
process.stdin.pipe(proxy.stdin)

// Forward container stdout to user
proxy.stdout.pipe(process.stdout)

// Handle proxy errors
proxy.on('error', (err) => {
  console.error('ERROR: Failed to connect to MCP server:', err.message)
  process.exit(1)
})

// Handle proxy exit
proxy.on('exit', (code, signal) => {
  if (code !== 0 && signal !== 'SIGTERM') {
    console.error(`ERROR: MCP server exited unexpectedly (code: ${code}, signal: ${signal})`)
    process.exit(code || 1)
  }
  process.exit(0)
})

// Handle user interrupts (Ctrl+C)
process.on('SIGINT', () => {
  console.log('\n⏸  Disconnecting from MCP server (services still running)...')
  proxy.kill('SIGTERM')
  process.exit(0)
})

process.on('SIGTERM', () => {
  proxy.kill('SIGTERM')
  process.exit(0)
})
```

### Error Handling - Common Scenarios

#### Docker Not Running
```
❌ ERROR: Docker is not running or not installed.

Please start Docker Desktop or install Docker:
  • macOS: https://docs.docker.com/desktop/install/mac-install/
  • Linux: https://docs.docker.com/engine/install/
  • Windows: https://docs.docker.com/desktop/install/windows-install/
```

#### Port Conflicts
```
❌ ERROR: Port 5432 is already in use.

PostgreSQL requires port 5432. Please either:
  1. Stop the service using port 5432
  2. Edit ~/.maproom-mcp/docker-compose.yml to use a different port

To find the conflicting process:
  lsof -i :5432
```

#### Health Check Timeout
```
❌ ERROR: Services did not become healthy within 2 minutes.

Check logs for errors:
  cd ~/.maproom-mcp
  docker compose logs postgres
  docker compose logs ollama
  docker compose logs maproom-mcp

Try restarting:
  docker compose down && docker compose up -d
```

### Progress Indicators
Use simple, informative messages:

```
🚀 Starting Maproom MCP Server...
✓ Docker is available
✓ Docker Compose v2 detected
✓ Configuration ready: ~/.maproom-mcp
✓ Services started
⏳ Waiting for services to become healthy...
  ⏳ postgres: starting...
  ✓ postgres: healthy
  ⏳ ollama: pulling model (this may take a few minutes)...
  ✓ ollama: healthy
  ⏳ maproom-mcp: connecting to database...
  ✓ maproom-mcp: ready
✓ All services are healthy
🔗 Connected to MCP server (stdio mode)
📝 Logs: docker compose logs -f (in ~/.maproom-mcp)
```

### Package.json bin Entry
Ensure `packages/maproom-mcp/package.json` has:

```json
{
  "bin": {
    "maproom-mcp": "./bin/cli.js"
  }
}
```

When published to npm, this enables `npx -y @crewchief/maproom-mcp` to execute `bin/cli.js`.

## Implementation Notes

### Alternative Stdio Proxy Approach
Instead of `docker compose exec -T ... cat`, could use:

```javascript
// Direct docker exec (more robust)
const containerName = 'maproom-mcp'
const proxy = spawn('docker', ['exec', '-i', containerName, 'node', '/app/dist/index.js'], {
  stdio: ['pipe', 'pipe', 'inherit']
})
```

This depends on container naming convention. Docker Compose generates names like `maproom-mcp` or `maproom_maproom-mcp_1`.

### Testing the CLI Wrapper
Before publishing, test locally:

```bash
# Make executable
chmod +x packages/maproom-mcp/bin/cli.js

# Test directly (not via npm)
node packages/maproom-mcp/bin/cli.js

# Test via npx local package
cd packages/maproom-mcp
npm pack
# Creates @crewchief-maproom-mcp-1.0.0.tgz

# Install and test
npx /path/to/@crewchief-maproom-mcp-1.0.0.tgz
```

### Integration with Claude/Cursor
After publishing, users configure `.mcp.json`:

```json
{
  "mcpServers": {
    "maproom": {
      "command": "npx",
      "args": ["-y", "@crewchief/maproom-mcp"]
    }
  }
}
```

Claude/Cursor will:
1. Execute: `npx -y @crewchief/maproom-mcp`
2. npx downloads package (if not cached)
3. Runs `bin/cli.js`
4. CLI starts Docker services and establishes stdio proxy
5. Claude/Cursor sends MCP JSON-RPC messages to CLI stdin
6. CLI forwards to container, forwards responses back

### Configuration Persistence
The `~/.maproom-mcp/` directory persists across runs:
- Docker Compose state
- PostgreSQL data volume
- Ollama models volume
- User customizations to docker-compose.yml

Users can customize:
```bash
# Edit configuration
code ~/.maproom-mcp/docker-compose.yml

# View logs
cd ~/.maproom-mcp && docker compose logs -f

# Stop services manually
cd ~/.maproom-mcp && docker compose down

# Clean everything (including data)
cd ~/.maproom-mcp && docker compose down -v
```

### Windows Compatibility
Windows requires:
- Docker Desktop for Windows
- Git Bash or PowerShell (for `npx`)
- Home directory detection: `os.homedir()` works cross-platform

Potential issue: Windows path separators. Use `path.join()` consistently.

## Dependencies
- **Blocks**: None - can start immediately
- **Blocked by**: LOCAL-2501 (Containerize TypeScript MCP server) - MUST complete first
- **Blocks**: LOCAL-2503 (npm package finalization)
- **Blocks**: LOCAL-3001 (test npx startup flow)

## Risk Assessment

### Risk: Docker Compose CLI format differences across versions
- **Impact**: Medium - health check parsing may fail
- **Mitigation**: Test on Docker Compose v2.20+ (JSON format stabilized)
- **Mitigation**: Fallback to simpler health check (just check containers are running)

### Risk: Stdio proxy drops messages or corrupts JSON-RPC
- **Impact**: High - MCP protocol will fail
- **Mitigation**: Avoid buffering, use direct piping
- **Mitigation**: Test with large JSON-RPC messages (search results with many chunks)
- **Mitigation**: Ensure no encoding issues (use raw binary pipes)

### Risk: Port conflicts prevent services from starting
- **Impact**: Medium - user cannot start services
- **Mitigation**: Clear error messages with diagnostic commands
- **Mitigation**: Document how to change ports in docker-compose.yml

### Risk: Health check timeout on slow machines or networks
- **Impact**: Medium - false negative, services actually healthy
- **Mitigation**: 2-minute timeout should be sufficient for most cases
- **Mitigation**: Show progress (which service is still starting)
- **Mitigation**: Document manual verification steps

### Risk: Container name conflicts if user has other containers
- **Impact**: Low - Docker Compose handles namespacing
- **Mitigation**: Use explicit container names in docker-compose.yml
- **Mitigation**: Document cleanup procedures

### Risk: npx caching issues with package updates
- **Impact**: Medium - users may run old version
- **Mitigation**: Use `npx -y` to always use latest
- **Mitigation**: Document cache clearing: `npx clear-npx-cache`

### Risk: Zombie containers left running after crash
- **Impact**: Low - benign but confusing
- **Mitigation**: Document cleanup: `docker compose down` in ~/.maproom-mcp
- **Mitigation**: Consider `--abort-on-container-exit` for tighter coupling

## Files/Packages Affected
- `/workspace/packages/maproom-mcp/bin/cli.js` (new file)
- `/workspace/packages/maproom-mcp/package.json` (verify bin entry exists)
- `/workspace/packages/maproom-mcp/config/docker-compose.yml` (packaged configuration)
- `/workspace/packages/maproom-mcp/config/init.sql` (packaged PostgreSQL schema)
- `/workspace/packages/maproom-mcp/README.md` (usage documentation - LOCAL-3002)

## Success Metrics
After implementation:
1. `node bin/cli.js` successfully starts all services
2. Health checks pass within 2 minutes on typical machine
3. Stdio proxy successfully forwards MCP JSON-RPC messages
4. Claude/Cursor can communicate with MCP server via npx
5. User interrupts (Ctrl+C) exit cleanly without orphaning containers
6. Error messages are clear and actionable
7. Works on macOS (primary platform) and Linux (CI/production)
8. Windows compatibility verified (best-effort)

---

## Implementation Notes (mcp-tools-engineer)

Successfully implemented the complete CLI wrapper with all specified features:

### Key Implementation Details:

1. **Pre-flight Checks** - Implemented comprehensive Docker availability checks:
   - `checkDockerDaemon()`: Verifies Docker daemon is running via `docker info`
   - `checkDockerCompose()`: Verifies Docker Compose v2 via `docker compose version`
   - Clear error messages with installation links for macOS/Linux/Windows

2. **Configuration Setup** - `setupConfigDirectory()` creates `~/.maproom-mcp/`:
   - Copies `docker-compose.yml` from package `config/` directory
   - Copies `init.sql` for PostgreSQL schema initialization
   - Uses cross-platform paths with `os.homedir()` and `path.join()`

3. **Docker Orchestration** - `startDockerCompose()`:
   - Runs `docker compose up -d` in CONFIG_DIR
   - Shows progress for image pulls (Downloading/Extracting messages)
   - Detects port conflicts and provides diagnostic commands
   - Returns promise that resolves when stack is running

4. **Health Check Waiting** - `waitForServicesHealthy()`:
   - Polls `docker compose ps --format json` every 2 seconds
   - Tracks status of postgres, ollama, maproom services
   - Shows per-service status updates (starting, healthy, unhealthy)
   - 2-minute timeout with helpful error messages
   - Handles containers with and without health checks gracefully

5. **Stdio Proxy** - `establishStdioProxy()`:
   - Uses `docker exec -i maproom-mcp node /app/dist/index.js`
   - Pipes `process.stdin` → `proxy.stdin` (user input to container)
   - Pipes `proxy.stdout` → `process.stdout` (container output to user)
   - Inherits stderr for debugging logs
   - Bidirectional JSON-RPC communication for MCP protocol

6. **Graceful Shutdown** - Signal handlers:
   - SIGINT (Ctrl+C): Kills proxy, displays helpful message, exits cleanly
   - SIGTERM: Kills proxy and exits
   - Containers continue running after CLI exits (intended behavior)
   - User can manually stop with `docker compose down`

7. **Error Handling** - User-friendly messages for:
   - Docker not installed or not running
   - Docker Compose v2 not available
   - Port conflicts (5432, 11434) with diagnostic commands
   - Service startup failures with log inspection commands
   - Health check timeouts with troubleshooting steps

### Technical Decisions:

- **All output to stderr**: Used `console.error()` for status/progress messages so stdout is reserved for MCP JSON-RPC (protocol requirement)
- **Synchronous checks**: Pre-flight checks use `spawnSync` for immediate feedback
- **Asynchronous orchestration**: Stack startup and health checks use promises/async-await
- **No external dependencies**: Uses only Node.js built-ins (child_process, fs, path, os)
- **Cross-platform**: All paths use `path.join()`, home directory via `os.homedir()`

### Files Modified:

- `/workspace/packages/maproom-mcp/bin/cli.js`: Complete rewrite (388 lines)
- Made executable with `chmod +x`
- package.json already had correct bin entry: `"maproom-mcp": "./bin/cli.js"`

### Testing Recommendations:

The implementation should be tested with:
```bash
# Direct execution
node /workspace/packages/maproom-mcp/bin/cli.js

# Via npx (after packing)
cd /workspace/packages/maproom-mcp
npm pack
npx /path/to/@crewchief-maproom-mcp-1.0.0.tgz
```

Expected behavior:
1. Checks pass (Docker running, Compose available)
2. Config directory created at `~/.maproom-mcp/`
3. Docker Compose stack starts (postgres, ollama, maproom)
4. Health checks pass within 2 minutes
5. Stdio proxy established
6. Can send MCP JSON-RPC messages via stdin
7. Ctrl+C exits gracefully, containers keep running

### Known Considerations:

- **Container name**: Uses `maproom-mcp` from docker-compose.yml (line 69 of config/docker-compose.yml)
- **Service name**: docker-compose.yml defines service as `maproom` but container_name as `maproom-mcp`
- **Health check logic**: Looks for service names: postgres, ollama, maproom (matches docker-compose.yml)
- **First run**: May take 2+ minutes to download Ollama model (nomic-embed-text)
- **Volume persistence**: Data persists in Docker volumes between runs

All acceptance criteria have been implemented and the CLI is ready for testing.

# MCPSTART: MCP Provider Startup Fix - Analysis

## Problem Statement

Despite two previous fix attempts (MCP-008 and MCP-011), the Maproom MCP server **still starts the Ollama container** when explicitly configured to use Google Vertex AI embeddings via `.mcp.json`. This indicates a fundamental issue in how environment variables flow from the MCP client configuration through to Docker Compose service selection.

## Current Situation Assessment

### What We've Tried

**MCP-008 (commit 5b7f1e4)**:
- Changed `docker-compose.yml` to use `${EMBEDDING_PROVIDER:-ollama}` instead of hardcoded `EMBEDDING_PROVIDER: ollama`
- Removed Ollama health check dependency from maproom-mcp service
- **Result**: Ollama still started

**MCP-011 (commit 3bb0071)**:
- Added auto-detection of outdated `docker-compose.yml` files
- Added automatic config file replacement
- Added explicit `docker compose stop` for unnecessary services
- **Result**: Ollama still started

### Why Previous Fixes Failed

The core issue is that **environment variables from `.mcp.json` are not reaching the Docker Compose stack**. When the MCP client (Claude, Cursor, etc.) invokes:

```json
{
  "command": "npx",
  "args": ["-y", "@crewchief/maproom-mcp@latest"],
  "env": {
    "EMBEDDING_PROVIDER": "google",
    "GOOGLE_PROJECT_ID": "crewchief-476600"
  }
}
```

The environment variables should flow:
1. MCP Client → Node.js process (`npx`)
2. Node.js process → CLI script (`bin/cli.cjs`)
3. CLI script → Docker Compose (`docker compose up`)
4. Docker Compose → Container environment

**The break is likely between steps 1→2 or 3→4.**

## Root Cause Hypotheses

### Hypothesis 1: npx Environment Isolation
`npx` may not pass through environment variables when executing packages. When it downloads and runs `@crewchief/maproom-mcp@latest`, it might create a fresh environment without the parent's env vars.

**Evidence**:
- Direct execution with `export EMBEDDING_PROVIDER=google && node cli.cjs` works
- Execution via `npx` with env vars in `.mcp.json` doesn't work

**Test**: Run `npx` with explicit environment and add debug logging at the start of `cli.cjs`

### Hypothesis 2: Docker Compose Env Var Inheritance
Docker Compose might not be inheriting environment variables from the spawning Node.js process, especially when using `spawn()` or `spawnSync()`.

**Evidence**:
- The CLI shows correct messages ("Starting with Google Vertex AI...")
- But Docker Compose still starts Ollama

**Test**: Add explicit `env` parameter to `spawn()` calls that include all needed env vars

### Hypothesis 3: Service Dependency Graph
Even though we removed the health check dependency, Docker Compose might still start Ollama because:
- It's defined in the `docker-compose.yml`
- The default behavior is to start all services unless explicitly told otherwise

**Evidence**:
- `docker compose up postgres maproom-mcp` should skip Ollama
- But the existing code might not be passing service names correctly

**Test**: Verify the exact `docker compose up` command being executed

### Hypothesis 4: Stale Container State
Previous Ollama containers might still be running from earlier sessions, giving the appearance that the fix isn't working.

**Evidence**:
- User reports "it's still not working" after fixes
- Might not have fully stopped and removed old containers

**Test**: Add `docker compose down` before `docker compose up` in the startup flow

## Industry Standards & Best Practices

### Environment Variable Propagation
**Node.js Child Processes**: By default, `spawn()` inherits parent environment. However, explicit `env` option **replaces** rather than extends. Best practice:

```javascript
spawn('docker', args, {
  env: {
    ...process.env,  // Include parent env
    CUSTOM_VAR: 'value'  // Add/override specific vars
  }
})
```

### Docker Compose Service Selection
**Selective Service Startup**: Docker Compose supports explicit service selection:

```bash
# Start only specific services
docker compose up -d postgres maproom-mcp

# This WILL NOT start ollama, even if it's defined in the file
```

**Profile-Based Configuration**: Modern Docker Compose supports profiles for optional services:

```yaml
services:
  ollama:
    profiles: ["ollama"]  # Only starts if --profile ollama
```

### MCP Server Configuration
**MCP Specification**: Environment variables in `mcpServers` config should be available to the command's process. The MCP SDK should pass these through.

**Common Issues**:
- Some MCP clients might not pass env vars correctly (client bug)
- `npx` might not preserve env vars when downloading packages
- Container isolation might prevent env var inheritance

## Existing Codebase State

### Current Flow (packages/maproom-mcp/bin/cli.cjs)

```
1. CLI starts → reads process.env.EMBEDDING_PROVIDER
2. getRequiredServices() → determines which services to start
3. startDockerCompose() → runs docker compose up [services...]
4. waitForHealthy() → waits for services to be ready
```

### Critical Code Sections

**Environment Variable Check** (lines ~200-230):
```javascript
function getRequiredServices() {
  const provider = process.env.EMBEDDING_PROVIDER?.toLowerCase();

  const services = {
    postgres: true,
    ollama: false,
    'maproom-mcp': true
  };

  if (!provider || provider === 'ollama') {
    services.ollama = true;
  }

  return Object.entries(services)
    .filter(([_, needed]) => needed)
    .map(([service, _]) => service);
}
```

**Docker Compose Execution** (lines ~240-270):
```javascript
function startDockerCompose() {
  const requiredServices = getRequiredServices();

  // Stop unnecessary services
  const unnecessaryServices = ['postgres', 'ollama', 'maproom-mcp']
    .filter(s => !requiredServices.includes(s));

  if (unnecessaryServices.length > 0) {
    spawnSync('docker', ['compose', 'stop', ...unnecessaryServices], {
      cwd: CONFIG_DIR,
      stdio: 'pipe'
    });
  }

  // Start required services
  const args = ['compose', 'up', '-d', ...requiredServices];
  spawn('docker', args, {
    cwd: CONFIG_DIR,
    stdio: ['ignore', 'pipe', 'pipe']
  });
}
```

### Potential Issues in Current Code

1. **No explicit env propagation**: The `spawn()` call doesn't explicitly pass `env` with `process.env`
2. **Config file auto-update timing**: Updates happen during setup, but containers might already be running
3. **No verification logging**: We don't log what `process.env.EMBEDDING_PROVIDER` actually contains
4. **No graceful shutdown**: Should do `docker compose down` first for clean state

## Gap Analysis

### What's Missing

1. **Diagnostic Logging**:
   - No log of actual env vars received at CLI startup
   - No log of exact docker compose command executed
   - No verification that env vars reach the Node.js process

2. **Environment Propagation**:
   - `spawn()` calls don't explicitly pass environment
   - No documentation of how npx handles env vars
   - No testing of MCP client → CLI env var flow

3. **Clean State Management**:
   - No guarantee containers are stopped before reconfiguration
   - No `docker compose down` to ensure clean slate
   - Auto-update might happen while containers are running

4. **Verification Testing**:
   - No automated test that verifies Ollama doesn't start
   - No integration test with actual MCP client
   - All testing has been manual

### What's Working

1. **Conditional Logic**: `getRequiredServices()` correctly determines what should start
2. **Service Selection**: Passing service names to `docker compose up` works
3. **Config Auto-Update**: Detects and updates outdated configs
4. **Manual Testing**: Works when env vars are explicitly exported in shell

## Research Insights

### npx Behavior with Environment Variables

From npm documentation and testing:
- `npx` **does** pass environment variables to executed packages
- However, it creates a temporary directory and may have different working directory
- Environment variables should be available via `process.env`

**Key Finding**: The issue is likely not with `npx` itself, but with how we're testing or how the MCP client invokes the command.

### Docker Compose Environment Variable Inheritance

From Docker documentation:
- Child processes inherit parent environment by default
- `docker compose` reads `.env` file from working directory
- Environment variables can be passed via `--env-file` or `-e` flags
- Service-level `environment` in `docker-compose.yml` takes precedence

**Key Finding**: The `environment:` section in `docker-compose.yml` **overrides** any env vars passed from the CLI, even with `${VAR:-default}` syntax, unless the outer process has that var set.

### MCP Client Implementation

From MCP specification:
- Clients should spawn server process with stdio transport
- Environment variables from `mcpServers.{name}.env` should be set on the process
- This is a MUST requirement in the spec

**Key Finding**: If the MCP client doesn't set env vars correctly, that's a client bug, not our bug. But we should verify this is happening.

## Critical Questions to Answer

1. **Is EMBEDDING_PROVIDER reaching cli.cjs?**
   - Add debug logging at the very start of the file
   - Log `process.env.EMBEDDING_PROVIDER` before any logic

2. **Is the docker compose command correct?**
   - Log the exact args array passed to `spawn()`
   - Verify service names are in the args

3. **Are containers actually stopped?**
   - Run `docker ps` before and after the fix
   - Verify Ollama container is not in the list

4. **Is this a published vs local issue?**
   - Does the fix work with local testing but not with published package?
   - Is there a version mismatch?

## Success Criteria

This project succeeds when:

1. ✅ **Environment variables are verified present**: Debug logs show `EMBEDDING_PROVIDER=google` at CLI startup
2. ✅ **Ollama does not start**: `docker ps` shows no `maproom-ollama` container when using Google
3. ✅ **Ollama does start**: When EMBEDDING_PROVIDER is not set or is 'ollama', Ollama starts correctly
4. ✅ **Published package works**: The fix works via `npx @crewchief/maproom-mcp@latest` with `.mcp.json` config
5. ✅ **Automated testing exists**: Integration test verifies correct startup behavior

## Constraints

- **Must preserve zero-config behavior**: Default (no env vars) should still start Ollama
- **Must work with published npm package**: Fix must work via `npx`, not just local testing
- **Must work with all MCP clients**: Should work with Claude Desktop, Cursor, Zed, etc.
- **Must not break existing users**: Ollama-based users must continue working
- **Must be verifiable**: We need definitive proof of what's actually happening

## Next Steps

The architecture document will propose a comprehensive solution that:
1. Adds robust diagnostic logging
2. Ensures environment variable propagation at every step
3. Implements clean container state management
4. Creates automated verification testing
5. Provides a clear debugging path for users

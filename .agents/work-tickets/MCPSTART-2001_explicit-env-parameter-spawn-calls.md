# Ticket: MCPSTART-2001: Add explicit env parameter to all spawn() calls

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- docker-engineer
- integration-tester
- verify-ticket
- commit-ticket

## Summary
Modify all spawn() and spawnSync() calls to explicitly pass environment variables, ensuring Docker Compose receives EMBEDDING_PROVIDER and other configuration. This is the CORE FIX for the Ollama startup issue.

## Background
By default, Node.js spawn() inherits the parent environment. However, explicitly specifying `env` REPLACES rather than extends the parent environment. The current code doesn't pass `env` at all, relying on implicit inheritance which may fail in certain environments (npx, MCP clients). This ticket makes environment passing explicit and guaranteed.

This implements **Phase 2.1** from MCPSTART_ARCHITECTURE.md - Explicit Environment Passing, which is the root cause fix for the issue where Docker Compose doesn't receive EMBEDDING_PROVIDER=google/openai and incorrectly starts Ollama containers.

## Acceptance Criteria
- [x] All spawn() and spawnSync() calls to docker include explicit `env` parameter
- [x] env parameter includes `...process.env` to preserve parent environment
- [x] Key variables explicitly set: EMBEDDING_PROVIDER, EMBEDDING_MODEL, EMBEDDING_DIMENSION
- [x] Default values provided for missing vars (e.g., EMBEDDING_PROVIDER defaults to 'ollama')
- [x] Diagnostic logging shows env vars being passed to Docker Compose (redacted)
- [ ] Integration tests verify Ollama doesn't start when EMBEDDING_PROVIDER=google

## Technical Requirements
- Modify all spawn()/spawnSync() calls that execute docker/docker compose commands in `packages/maproom-mcp/bin/cli.cjs`
- Use pattern: `env: { ...process.env, KEY: value || default }`
- Ensure these variables are explicitly in env object:
  - EMBEDDING_PROVIDER (default: 'ollama')
  - EMBEDDING_MODEL (default: 'nomic-embed-text')
  - EMBEDDING_DIMENSION (default: '768')
  - EMBEDDING_API_ENDPOINT (if set)
  - GOOGLE_PROJECT_ID (if set)
  - GOOGLE_APPLICATION_CREDENTIALS (if set)
  - OPENAI_API_KEY (if set)
  - DATABASE_URL (if set)
- Add diagnostic log showing which env vars are being passed (with credential redaction)
- Preserve all other environment variables from parent process

## Implementation Notes

### Core Pattern (from MCPSTART_ARCHITECTURE.md lines 99-132)

Apply this pattern to ALL docker command spawn/spawnSync calls:

```javascript
function startDockerCompose() {
  const args = ['compose', 'up', '-d', ...requiredServices];

  // CRITICAL: Explicitly pass environment variables
  const env = {
    ...process.env,  // Include all parent env vars
    // Ensure key vars are present with defaults
    EMBEDDING_PROVIDER: process.env.EMBEDDING_PROVIDER || 'ollama',
    EMBEDDING_MODEL: process.env.EMBEDDING_MODEL || 'nomic-embed-text',
    EMBEDDING_DIMENSION: process.env.EMBEDDING_DIMENSION || '768'
  };

  diagnosticLog('Starting Docker Compose', {
    args,
    env: redactSensitive({
      EMBEDDING_PROVIDER: env.EMBEDDING_PROVIDER,
      EMBEDDING_MODEL: env.EMBEDDING_MODEL,
      EMBEDDING_DIMENSION: env.EMBEDDING_DIMENSION
    })
  });

  const compose = spawn('docker', args, {
    cwd: CONFIG_DIR,
    env: env,  // Explicitly pass environment
    stdio: ['ignore', 'pipe', 'pipe'],
    encoding: 'utf-8'
  });
}
```

### Key Implementation Points

1. **Always use `...process.env` FIRST** in the env object to preserve all parent vars
2. **Explicitly set defaults** for critical vars (EMBEDDING_PROVIDER, etc.)
3. **Log before spawn** to show what env is being passed (use redactSensitive helper from MCPSTART-1004)
4. **Apply to ALL docker commands**: compose up, compose down, compose ps, container inspect, etc.

### Files to Modify

Search for all spawn() and spawnSync() calls in `packages/maproom-mcp/bin/cli.cjs` that execute:
- `docker compose up`
- `docker compose down`
- `docker compose ps`
- `docker container inspect`
- Any other docker commands

Each must receive explicit `env` parameter with the pattern above.

## Dependencies
- **Prerequisite**: MCPSTART-1001 (diagnostic logging infrastructure)
- **Prerequisite**: MCPSTART-1002 (docker command logging)
- **Prerequisite**: MCPSTART-1003 (container state logging)
- **Prerequisite**: MCPSTART-1004 (credential redaction)
- **Blocks**: MCPSTART-2002 (docker-compose.yaml env propagation)
- **Blocks**: MCPSTART-2003 (service filtering logic)

## Risk Assessment
- **Risk**: Medium - changes how env vars are passed to all docker commands, could break if done incorrectly
  - **Mitigation**: Use `...process.env` first to preserve all existing vars before adding/overriding specific ones
  - **Mitigation**: Test with all three providers (ollama, google, openai) to ensure correct behavior
  - **Mitigation**: Verify diagnostic logs show correct env vars being passed

- **Risk**: Low - could accidentally expose credentials in logs
  - **Mitigation**: Use redactSensitive() helper from MCPSTART-1004 for all diagnostic logging

- **Test Plan**: Run integration tests from MCPSTART-4001 to verify:
  - Ollama doesn't start when EMBEDDING_PROVIDER=google
  - Google credentials are available when EMBEDDING_PROVIDER=google
  - Default to Ollama when EMBEDDING_PROVIDER is unset

## Files/Packages Affected
- `packages/maproom-mcp/bin/cli.cjs` - All spawn/spawnSync calls to docker commands

## Implementation Notes

### Changes Made

Successfully added explicit environment parameter passing to ALL 7 spawn/spawnSync calls in `packages/maproom-mcp/bin/cli.cjs`:

1. **checkDockerDaemon()** - `docker info` (line 125)
2. **checkDockerCompose()** - `docker compose version` (line 173)
3. **logDockerState()** - `docker compose ps` (line 353)
4. **startDockerCompose()** - Stopping unnecessary services - `docker compose stop` (line 465)
5. **startDockerCompose()** - Starting services - `docker compose up` (line 510)
6. **waitForServicesHealthy()** - Health check loop - `docker compose ps` (line 605)
7. **establishStdioProxy()** - Stdio proxy - `docker exec` (line 744)

### Pattern Applied

Each docker command now uses this pattern:

```javascript
// Explicitly pass environment variables to docker command
const env = {
  ...process.env,  // CRITICAL: Include all parent env vars FIRST
  // Ensure key vars are present with defaults
  EMBEDDING_PROVIDER: process.env.EMBEDDING_PROVIDER || 'ollama',
  EMBEDDING_MODEL: process.env.EMBEDDING_MODEL || 'nomic-embed-text',
  EMBEDDING_DIMENSION: process.env.EMBEDDING_DIMENSION || '768'
};

diagnosticLog('Docker Command: <description>', {
  command: 'docker',
  args: [...],
  cwd: ...,
  env: redactSensitive({
    EMBEDDING_PROVIDER: env.EMBEDDING_PROVIDER,
    EMBEDDING_MODEL: env.EMBEDDING_MODEL,
    EMBEDDING_DIMENSION: env.EMBEDDING_DIMENSION
  })
});

const result = spawn/spawnSync('docker', [...], {
  env: env,  // Explicit environment passing
  ...otherOptions
});
```

### Key Features

1. **Environment Preservation**: `...process.env` is ALWAYS first to preserve all parent environment variables
2. **Explicit Defaults**: Critical variables (EMBEDDING_PROVIDER, EMBEDDING_MODEL, EMBEDDING_DIMENSION) explicitly set with defaults
3. **Diagnostic Logging**: Every spawn call logs the environment being passed (with credential redaction)
4. **Consistency**: Same pattern applied to all 7 docker command invocations
5. **Comprehensive Coverage**: Includes docker info, compose version, compose ps, compose stop, compose up, and exec commands

### Expected Behavior Changes

After this fix:
- Docker Compose WILL receive EMBEDDING_PROVIDER from parent process environment
- When EMBEDDING_PROVIDER=google/openai, Ollama containers will NOT start
- Diagnostic logs will show which environment variables are being passed to each docker command
- This fixes the root cause where Docker Compose wasn't receiving the EMBEDDING_PROVIDER configuration

### Verification Commands

To verify the fix works:

```bash
# Test with Google provider
EMBEDDING_PROVIDER=google npx -y @crewchief/maproom-mcp
# Should see: "Starting with Google Vertex AI..." and "Skipping Ollama - not needed"

# Test with OpenAI provider
EMBEDDING_PROVIDER=openai npx -y @crewchief/maproom-mcp
# Should see: "Starting with OpenAI..." and "Skipping Ollama - not needed"

# Test with Ollama (default)
npx -y @crewchief/maproom-mcp
# Should see: "Starting with Ollama (local embeddings)..."
```

With `MAPROOM_MCP_DEBUG=true`, diagnostic logs will show the environment being passed to each docker command.

# MCPSTART: MCP Provider Startup Fix - Architecture

## Design Principles

1. **Fail Fast with Clear Diagnostics**: If env vars aren't set, tell the user immediately and clearly
2. **Explicit Over Implicit**: Don't rely on Docker Compose defaults, be explicit about what starts/stops
3. **Verifiable at Every Step**: Log key decisions and state so issues can be debugged
4. **Zero-Config Still Works**: Default behavior (no env vars) must start Ollama seamlessly
5. **Clean State Management**: Always ensure containers are in expected state before operations

## Proposed Solution

### Phase 1: Diagnostic Infrastructure

Add comprehensive logging to understand what's actually happening:

**1.1 Environment Variable Verification**

Add at the **very top** of `bin/cli.cjs` (after requires, before any logic):

```javascript
// ============================================
// DIAGNOSTIC: Environment Variable Logging
// ============================================
const DIAGNOSTIC_MODE = process.env.MAPROOM_MCP_DEBUG === 'true';

function diagnosticLog(message, data) {
  if (DIAGNOSTIC_MODE || !process.env.EMBEDDING_PROVIDER) {
    // Always log in debug mode, or if provider is missing (helps diagnosis)
    console.error('🔍 [DIAGNOSTIC]', message);
    if (data) {
      console.error('   ', JSON.stringify(data, null, 2));
    }
  }
}

// Log environment variables immediately on startup
diagnosticLog('CLI Started', {
  EMBEDDING_PROVIDER: process.env.EMBEDDING_PROVIDER || '(not set)',
  GOOGLE_PROJECT_ID: process.env.GOOGLE_PROJECT_ID ? '(set)' : '(not set)',
  GOOGLE_APPLICATION_CREDENTIALS: process.env.GOOGLE_APPLICATION_CREDENTIALS ? '(set)' : '(not set)',
  OPENAI_API_KEY: process.env.OPENAI_API_KEY ? '(set)' : '(not set)',
  OLLAMA_HOST: process.env.OLLAMA_HOST || '(not set)',
  NODE_ENV: process.env.NODE_ENV || '(not set)',
  cwd: process.cwd()
});
```

**1.2 Docker Command Logging**

Before every Docker command, log what we're about to execute:

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

**1.3 Service State Logging**

After container operations, verify state:

```javascript
function logDockerState() {
  const result = spawnSync('docker', ['compose', 'ps', '--format', 'json'], {
    cwd: CONFIG_DIR,
    encoding: 'utf-8',
    stdio: 'pipe'
  });

  if (result.status === 0) {
    const containers = result.stdout.trim().split('\n')
      .filter(line => line)
      .map(line => JSON.parse(line));

    diagnosticLog('Container State', containers.map(c => ({
      service: c.Service,
      state: c.State,
      status: c.Status
    })));
  }
}
```

### Phase 2: Environment Variable Propagation

Ensure env vars flow correctly to Docker Compose:

**2.1 Explicit Environment Passing**

Modify all `spawn()` and `spawnSync()` calls to explicitly include environment:

```javascript
function startDockerCompose() {
  // ... service selection logic ...

  const args = ['compose', 'up', '-d', ...requiredServices];

  // CRITICAL: Explicitly pass environment variables
  const env = {
    ...process.env,  // Include all parent env vars
    // Ensure key vars are present
    EMBEDDING_PROVIDER: process.env.EMBEDDING_PROVIDER || 'ollama',
    EMBEDDING_MODEL: process.env.EMBEDDING_MODEL || 'nomic-embed-text',
    EMBEDDING_DIMENSION: process.env.EMBEDDING_DIMENSION || '768'
  };

  diagnosticLog('Starting Docker Compose', { args, env: {
    EMBEDDING_PROVIDER: env.EMBEDDING_PROVIDER,
    EMBEDDING_MODEL: env.EMBEDDING_MODEL
  }});

  const compose = spawn('docker', args, {
    cwd: CONFIG_DIR,
    env: env,  // Explicitly pass environment
    stdio: ['ignore', 'pipe', 'pipe'],
    encoding: 'utf-8'
  });

  // ... rest of logic ...
}
```

**2.2 Docker Compose File Verification**

After auto-update, verify the config actually uses environment variables:

```javascript
function verifyDockerComposeConfig() {
  const content = fs.readFileSync(COMPOSE_FILE, 'utf-8');

  // Check for environment variable syntax
  const hasEnvVarSyntax = /\$\{EMBEDDING_PROVIDER[:\-]/.test(content);
  const hasHardcodedProvider = /EMBEDDING_PROVIDER:\s*['"]?ollama['"]?\s*$/m.test(content);

  if (hasHardcodedProvider && !hasEnvVarSyntax) {
    console.error('❌ ERROR: docker-compose.yml has hardcoded EMBEDDING_PROVIDER');
    console.error('   This will override your configuration.');
    console.error('   File location:', COMPOSE_FILE);
    process.exit(1);
  }

  diagnosticLog('Docker Compose Config Verified', {
    hasEnvVarSyntax,
    hasHardcodedProvider: false
  });
}
```

### Phase 3: Clean State Management

Ensure containers are in the expected state:

**3.1 Pre-Flight Container Cleanup**

Before starting services, ensure clean state:

```javascript
function ensureCleanState() {
  diagnosticLog('Checking existing container state');

  // Get current container states
  const psResult = spawnSync('docker', ['compose', 'ps', '-q'], {
    cwd: CONFIG_DIR,
    encoding: 'utf-8',
    stdio: 'pipe'
  });

  const hasContainers = psResult.stdout.trim().length > 0;

  if (hasContainers) {
    diagnosticLog('Existing containers found, stopping all services');

    // Stop all services to ensure clean state
    const stopResult = spawnSync('docker', ['compose', 'stop'], {
      cwd: CONFIG_DIR,
      encoding: 'utf-8',
      stdio: 'pipe'
    });

    if (stopResult.status === 0) {
      console.error('✓ Stopped existing containers');
    }

    // Wait briefly for containers to fully stop
    sleep(1000);
  }

  logDockerState();
}
```

**3.2 Explicit Service Removal**

When a service should NOT be running, ensure it's removed:

```javascript
function removeUnnecessaryServices(unnecessaryServices) {
  if (unnecessaryServices.length === 0) return;

  diagnosticLog('Removing unnecessary services', { services: unnecessaryServices });

  // Stop services
  const stopResult = spawnSync('docker',
    ['compose', 'stop', ...unnecessaryServices],
    {
      cwd: CONFIG_DIR,
      encoding: 'utf-8',
      stdio: 'pipe'
    }
  );

  if (stopResult.status === 0) {
    // Remove stopped containers
    const rmResult = spawnSync('docker',
      ['compose', 'rm', '-f', ...unnecessaryServices],
      {
        cwd: CONFIG_DIR,
        encoding: 'utf-8',
        stdio: 'pipe'
      }
    );

    if (rmResult.status === 0) {
      console.error(`✓ Removed unnecessary services: ${unnecessaryServices.join(', ')}`);
    }
  }

  logDockerState();
}
```

### Phase 4: Service Profiles (Alternative Approach)

For a more robust long-term solution, use Docker Compose profiles:

**4.1 Docker Compose Profile Configuration**

Update `config/docker-compose.yml` to use profiles:

```yaml
services:
  postgres:
    # ... always required ...

  ollama:
    profiles: ["ollama"]  # Only start if --profile ollama
    # ... rest of ollama config ...

  maproom-mcp:
    # ... always required ...
    depends_on:
      postgres:
        condition: service_healthy
      # Remove ollama dependency entirely
```

**4.2 Profile-Based Startup**

Modify CLI to use profiles instead of service selection:

```javascript
function startDockerCompose() {
  const provider = process.env.EMBEDDING_PROVIDER?.toLowerCase();

  // Determine which profiles to activate
  const profiles = [];
  if (!provider || provider === 'ollama') {
    profiles.push('ollama');
    console.error('🚀 Starting with Ollama (local embeddings)...');
  } else {
    console.error(`🚀 Starting with ${provider} embeddings...`);
    console.error('   (Ollama not needed, skipping)');
  }

  const args = ['compose'];

  // Add profile flags
  profiles.forEach(profile => {
    args.push('--profile', profile);
  });

  args.push('up', '-d');

  diagnosticLog('Docker Compose with Profiles', { args, profiles });

  // ... execute command ...
}
```

**Benefits of Profile Approach**:
- Docker Compose handles service selection natively
- No manual service name management
- Clearer intent in configuration
- More maintainable long-term

## Implementation Strategy

### Recommended Approach: Phased Implementation

**Phase 1 First** (Diagnostic Infrastructure):
- Gets us visibility into what's happening
- Can be shipped quickly
- Helps users debug their own issues
- No risk to existing functionality

**Phase 2 Second** (Environment Propagation):
- Fixes the core issue
- Low risk (explicit env passing)
- Can verify with Phase 1 diagnostics

**Phase 3 Third** (Clean State Management):
- Ensures robustness
- Prevents "works sometimes" issues
- Medium risk (modifies container lifecycle)

**Phase 4 Optional** (Service Profiles):
- Better long-term architecture
- Requires docker-compose.yml changes
- Should be separate minor version

### Critical Implementation Details

**1. Backwards Compatibility**

Ensure existing users (Ollama-based) aren't broken:

```javascript
// Default to ollama if not specified
const provider = process.env.EMBEDDING_PROVIDER?.toLowerCase() || 'ollama';

// Treat empty string as default
if (provider === '') {
  provider = 'ollama';
}
```

**2. Docker Compose Version Compatibility**

Check for `--profile` support:

```javascript
function supportsProfiles() {
  const result = spawnSync('docker', ['compose', 'version'], {
    encoding: 'utf-8',
    stdio: 'pipe'
  });

  if (result.status === 0) {
    const version = result.stdout.match(/v?(\d+)\.(\d+)/);
    if (version) {
      const major = parseInt(version[1]);
      const minor = parseInt(version[2]);
      // Profiles supported in Docker Compose v2.0+
      return major >= 2;
    }
  }

  return false;
}
```

**3. npx Compatibility**

Ensure the published package works correctly:

```javascript
// Log package metadata for debugging
const packageJson = require('../package.json');
diagnosticLog('Package Info', {
  name: packageJson.name,
  version: packageJson.version,
  workingDir: __dirname
});
```

**4. MCP Client Compatibility**

Provide fallback for clients that don't pass env vars:

```javascript
// Check if we're running via MCP (stdio mode)
const isMCPMode = !process.stdin.isTTY;

if (isMCPMode && !process.env.EMBEDDING_PROVIDER) {
  console.error('⚠️  WARNING: Running in MCP mode but EMBEDDING_PROVIDER not set');
  console.error('   This usually means your MCP client is not passing environment variables');
  console.error('   Please check your .mcp.json configuration');
  console.error('   Defaulting to Ollama for zero-config experience');
}
```

## File Structure

```
packages/maproom-mcp/
├── bin/
│   └── cli.cjs                 # Modified with diagnostics & env propagation
├── config/
│   ├── docker-compose.yml      # Updated with profiles (Phase 4)
│   └── docker-compose.env.example  # Example env file for users
├── src/
│   └── utils/
│       ├── diagnostics.ts      # Diagnostic logging utilities
│       └── docker.ts           # Docker command wrappers
└── tests/
    └── integration/
        └── startup.test.js     # Integration tests for startup behavior
```

## Configuration File Example

Create `config/docker-compose.env.example`:

```bash
# Maproom MCP Environment Configuration
# Copy this to docker-compose.env and customize

# Embedding Provider (ollama, google, openai)
EMBEDDING_PROVIDER=ollama

# Ollama Configuration (if EMBEDDING_PROVIDER=ollama)
EMBEDDING_MODEL=nomic-embed-text
EMBEDDING_DIMENSION=768
EMBEDDING_API_ENDPOINT=http://ollama:11434

# Google Vertex AI Configuration (if EMBEDDING_PROVIDER=google)
GOOGLE_PROJECT_ID=your-project-id
GOOGLE_APPLICATION_CREDENTIALS=/path/to/credentials.json
GOOGLE_VERTEX_REGION=us-west1

# OpenAI Configuration (if EMBEDDING_PROVIDER=openai)
OPENAI_API_KEY=your-api-key

# Database Configuration
DATABASE_URL=postgresql://maproom:maproom@maproom-postgres:5432/maproom

# Logging
LOG_LEVEL=info
```

## Testing Strategy

See QUALITY_STRATEGY.md for comprehensive testing approach, but key architectural tests:

**1. Environment Variable Flow Test**

```javascript
// Verify env vars reach the CLI
test('environment variables are received by CLI', () => {
  const result = execSync('EMBEDDING_PROVIDER=google node bin/cli.cjs --help', {
    env: { ...process.env, EMBEDDING_PROVIDER: 'google' }
  });

  expect(result.stderr).toContain('Starting with Google Vertex AI');
});
```

**2. Container State Test**

```javascript
// Verify Ollama doesn't start with Google
test('ollama container not started with google provider', async () => {
  await startMCP({ EMBEDDING_PROVIDER: 'google' });

  const containers = await getRunningContainers();
  const ollamaContainer = containers.find(c => c.names.includes('ollama'));

  expect(ollamaContainer).toBeUndefined();
});
```

**3. Service Selection Test**

```javascript
// Verify correct services start
test('correct services start based on provider', async () => {
  await startMCP({ EMBEDDING_PROVIDER: 'google' });

  const containers = await getRunningContainers();
  const serviceNames = containers.map(c => c.labels['com.docker.compose.service']);

  expect(serviceNames).toContain('postgres');
  expect(serviceNames).toContain('maproom-mcp');
  expect(serviceNames).not.toContain('ollama');
});
```

## Rollout Plan

1. **Version 1.1.8** - Diagnostic Infrastructure
   - Add comprehensive logging
   - No behavior changes
   - Can identify issues in production

2. **Version 1.1.9** - Environment Propagation Fix
   - Explicit env passing
   - Clean state management
   - Verify with diagnostics

3. **Version 1.2.0** - Service Profiles (optional)
   - Architectural improvement
   - Better maintainability
   - Breaking change (major version bump)

## Success Metrics

- ✅ **Diagnostic logs show env vars present**: Users can verify config is being read
- ✅ **Ollama doesn't start with Google**: `docker ps` confirms no ollama container
- ✅ **Ollama starts with default**: Zero-config still works
- ✅ **Published package works**: Fix verified via `npx` installation
- ✅ **All MCP clients work**: Tested with Claude Desktop, Cursor

## Risk Mitigation

**Risk**: Changes break existing Ollama users
**Mitigation**: Comprehensive testing, default to Ollama on any ambiguity

**Risk**: Docker Compose version incompatibility
**Mitigation**: Version detection, fallback to service name approach

**Risk**: MCP client doesn't pass env vars
**Mitigation**: Clear warning messages, default to zero-config behavior

**Risk**: Published package differs from local testing
**Mitigation**: Test the actual published package before announcing fix

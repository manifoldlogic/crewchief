# Quality Strategy: MCP Extension Initialization

## Philosophy

**Build confidence, not coverage.** Tests should prevent rework by catching issues before they reach users. For this MVP, focus on critical paths and integration points. Avoid ceremonial testing that doesn't reduce risk.

## Risk Assessment

### High-Risk Areas (Must Test)

1. **MCP Configuration Writing**
   - **Risk**: Malformed JSON, incorrect environment variable syntax, overwriting user config
   - **Impact**: MCP server fails to start, user loses existing configuration
   - **Mitigation**: Unit tests for config generation, integration tests for file writes

2. **Configuration Merging**
   - **Risk**: Overwriting existing MCP servers in user's config
   - **Impact**: User loses other MCP server configurations
   - **Mitigation**: Unit tests for merge logic, integration tests with existing configs

3. **Workspace Validation**
   - **Risk**: Writing config files outside workspace (path traversal)
   - **Impact**: Security vulnerability, file system corruption
   - **Mitigation**: Unit tests for path validation logic

### Medium-Risk Areas (Should Test)

4. **UI Integration**
   - **Risk**: Extension activation prompt not showing correctly
   - **Impact**: Users don't discover setup wizard
   - **Mitigation**: Manual testing on first activation

5. **Provider Selection**
   - **Risk**: Incorrect environment variable names for providers
   - **Impact**: MCP server fails with wrong credentials
   - **Mitigation**: Unit tests for each provider's config

### Low-Risk Areas (Manual Testing Only)

6. **UI/UX Flow**
   - **Risk**: Confusing wizard steps, unclear error messages
   - **Impact**: Users don't complete setup
   - **Mitigation**: Manual testing with fresh installs

## Test Strategy

### Unit Tests

**Purpose**: Validate individual components in isolation

**Scope**:
- MCP configuration generation
- Provider-specific environment variables
- Configuration merging (preserve existing MCP servers)
- Workspace path validation
- Extension activation logic

**Technology**: Vitest (already in project)

**Example Tests**:

```typescript
// src/config/mcp-writer.test.ts
describe('MCPConfigWriter', () => {
  it('should preserve existing MCP servers when adding maproom', () => {
    const existing = {
      mcpServers: {
        'other-server': { command: 'foo', args: ['bar'] }
      }
    }

    const writer = new MCPConfigWriter()
    const updated = writer.mergeMaproomServer(existing, 'openai')

    expect(updated.mcpServers).toHaveProperty('other-server')
    expect(updated.mcpServers).toHaveProperty('maproom')
  })

  it('should use env variable syntax for API keys', () => {
    const writer = new MCPConfigWriter()
    const config = writer.buildMaproomConfig({ provider: 'openai' })

    expect(config.env?.OPENAI_API_KEY).toBe('${env:OPENAI_API_KEY}')
  })

  it('should reject paths outside workspace', () => {
    const writer = new MCPConfigWriter()

    expect(() => {
      writer.registerMCPServer('/workspace', '../../../etc/passwd')
    }).toThrow('Invalid path')
  })

  it('should generate correct config for each provider', () => {
    const writer = new MCPConfigWriter()

    const openai = writer.buildEnvironment('openai')
    expect(openai.OPENAI_API_KEY).toBe('${env:OPENAI_API_KEY}')

    const google = writer.buildEnvironment('google')
    expect(google.GOOGLE_APPLICATION_CREDENTIALS).toBe('${env:GOOGLE_APPLICATION_CREDENTIALS}')

    const ollama = writer.buildEnvironment('ollama')
    expect(Object.keys(ollama)).toHaveLength(0) // No env vars needed
  })
})
```

**Coverage Goal**: 70% for new code. Focus on happy path + critical error cases.

### Integration Tests

**Purpose**: Validate component interactions and external dependencies

**Scope**:
- Setup wizard flow (without real Docker)
- File system operations (temp directory)
- PostgreSQL connectivity checks
- Process spawning (with mock CLI)

**Technology**: Vitest + temporary directories

**Example Tests**:

```typescript
// test/integration/setup-flow.test.ts
describe('Setup Flow Integration', () => {
  let tempDir: string

  beforeEach(() => {
    tempDir = mkdtempSync(path.join(os.tmpdir(), 'maproom-test-'))
  })

  afterEach(() => {
    rmSync(tempDir, { recursive: true })
  })

  it('should write MCP config to workspace .vscode directory', async () => {
    const configPath = path.join(tempDir, '.vscode', 'mcp.json')

    const writer = new MCPConfigWriter()
    await writer.registerMCPServer({
      workspaceRoot: tempDir,
      provider: 'ollama'
    })

    expect(existsSync(configPath)).toBe(true)

    const config = JSON.parse(readFileSync(configPath, 'utf-8'))
    expect(config.mcpServers.maproom).toBeDefined()
  })

  it('should handle missing .vscode directory', async () => {
    const writer = new MCPConfigWriter()

    // Should not throw
    await writer.registerMCPServer({
      workspaceRoot: tempDir,
      provider: 'openai'
    })

    const configPath = path.join(tempDir, '.vscode', 'mcp.json')
    expect(existsSync(configPath)).toBe(true)
  })
})

// test/integration/postgres-check.test.ts
describe('PostgreSQL Connection Check', () => {
  it('should detect when postgres is not running', async () => {
    const config = {
      host: 'nonexistent-host',
      port: 9999,
      user: 'test',
      password: 'test',
      database: 'test'
    }

    const available = await checkPostgresAvailable(config, 500)
    expect(available).toBe(false)
  })

  // Note: Test with running postgres would require Docker
  // Skip for unit test suite, verify manually
})
```

### Manual Testing Checklist

**Purpose**: Validate end-to-end user experience

#### First-Time Setup

- [ ] Install extension from VSIX
- [ ] Open workspace without `.vscode/mcp.json`
- [ ] Should see "Maproom requires initial setup" message
- [ ] Click "Run Setup"
- [ ] Select provider (OpenAI/Google/Ollama)
- [ ] Enter credentials
- [ ] Observe progress notification (shows meaningful progress)
- [ ] Wait for completion (2-5 minutes)
- [ ] Verify `.vscode/mcp.json` created with correct config
- [ ] Check status bar shows "$(check) Maproom: Ready"
- [ ] Test MCP search command works

#### Setup Without Docker

- [ ] Uninstall Docker Desktop
- [ ] Install extension
- [ ] Try to run setup
- [ ] Should see clear error: "Docker not found. Install Docker Desktop: https://..."
- [ ] Error should be actionable, not cryptic

#### Setup Cancellation

- [ ] Start setup
- [ ] Wait for Docker Compose to start
- [ ] Click "Cancel" on progress notification
- [ ] Verify:
  - Process terminates
  - Containers stop (not left running)
  - Can retry setup successfully

#### Error Recovery

- [ ] Stop Docker containers: `docker compose down`
- [ ] Check status bar shows "$(warning) Maproom: Setup Required"
- [ ] Click status bar
- [ ] Should see options:
  - "Run Setup"
  - "Restart Services"
  - "View Logs"
- [ ] Test each option works

#### Configuration Persistence

- [ ] Complete setup
- [ ] Close VS Code
- [ ] Reopen workspace
- [ ] Should not prompt for setup again
- [ ] Status bar should show ready state
- [ ] MCP commands should work immediately

#### Multi-Workspace

- [ ] Open Workspace A, complete setup
- [ ] Open Workspace B (new window)
- [ ] Should prompt for setup again (workspace-scoped)
- [ ] Complete setup in B
- [ ] Both workspaces should work independently

#### Provider Switching

- [ ] Complete setup with Ollama
- [ ] Run setup again, choose OpenAI
- [ ] Should update `.vscode/mcp.json` with new provider
- [ ] Restart containers
- [ ] Verify OpenAI embeddings work

### Acceptance Criteria

Before merging to main:

1. **No Regression**: Existing features (scan, watch, search) continue working
2. **Setup Success**: Can complete setup with each provider (OpenAI, Google, Ollama)
3. **Error Clarity**: Error messages guide users to resolution
4. **Clean Cancellation**: No orphaned processes or containers
5. **Configuration Safety**: Existing MCP servers not overwritten
6. **Status Accuracy**: Status bar reflects true service state

## Test Data

### Mock CLI Output

Store example CLI output for testing progress parsing:

```typescript
// test/fixtures/cli-output.ts
export const SETUP_OUTPUT_SAMPLES = {
  dockerDownload: `
Downloading Docker images...
Pulling postgres:15 ... done
Pulling pgvector:0.5.1 ... done
  `.trim(),

  ollamaModel: `
Downloading Ollama model...
pulling manifest
pulling 6a0746a1ec1a... 100%
pulling 15ad3a82de0f... 100%
success
  `.trim(),

  serviceStartup: `
Creating network "maproom_default" ... done
Creating maproom-postgres ... done
Waiting for PostgreSQL to be ready...
PostgreSQL is ready
  `.trim(),
}
```

### Test Configurations

```typescript
// test/fixtures/mcp-configs.ts
export const EXISTING_MCP_CONFIG = {
  mcpServers: {
    'github': {
      command: 'npx',
      args: ['@modelcontextprotocol/server-github'],
      env: { GITHUB_TOKEN: '${env:GITHUB_TOKEN}' }
    }
  }
}

export const EXPECTED_MERGED_CONFIG = {
  mcpServers: {
    'github': { /* unchanged */ },
    'maproom': {
      command: 'npx',
      args: ['@crewchief/maproom-mcp'],
      env: { OPENAI_API_KEY: '${env:OPENAI_API_KEY}' }
    }
  }
}
```

## CI/CD Integration

### What to Automate

**Unit Tests**: Run on every commit
```yaml
- name: Unit Tests
  run: |
    cd packages/vscode-maproom
    pnpm test
```

**Lint & Format**: Enforce code quality
```yaml
- name: Lint
  run: pnpm lint:check
```

**Package Size Check**: Prevent regressions
```yaml
- name: Check VSIX Size
  run: |
    SIZE=$(stat -f%z vscode-maproom-*.vsix)
    if [ $SIZE -gt 5242880 ]; then  # 5MB
      echo "VSIX too large: ${SIZE} bytes"
      exit 1
    fi
```

### What to Manual Test

- First-time setup experience
- Error recovery flows
- Cross-platform behavior (Windows/macOS/Linux)
- Docker version compatibility

## Performance Criteria

### Extension Activation

**Goal**: Activate in <100ms

**Measurement**:
```typescript
const activationStart = Date.now()
export async function activate(context: vscode.ExtensionContext) {
  // ... activation logic ...
  const activationTime = Date.now() - activationStart
  if (activationTime > 100) {
    console.warn(`Slow activation: ${activationTime}ms`)
  }
}
```

**Strategy**: Defer all heavy operations (setup check, status monitoring) until after activation completes

### Setup Time

**Expected**: 2-5 minutes (acceptable for one-time operation)

**Breakdown**:
- Docker image download: 1-3 minutes (first time only)
- Ollama model download: 1-2 minutes (Ollama provider only)
- Container startup: 10-30 seconds
- Service validation: 5-10 seconds

**Not a Performance Issue**: This is infrastructure setup, one-time cost is acceptable if progress is visible

### Status Check Overhead

**Goal**: <10ms per check

**Frequency**: Every 30 seconds

**Cost**: Single TCP connection attempt to PostgreSQL

**Acceptable**: 10ms every 30s = 0.03% CPU impact (negligible)

## Security Testing

### Credential Handling

**Test Cases**:
- [ ] API keys never logged to output channel
- [ ] Secrets stored in VS Code SecretStorage, not plaintext files
- [ ] Environment variable syntax used in `.vscode/mcp.json`
- [ ] Credentials passed to CLI via environment variables, not command line args

**Validation**:
```typescript
it('should not log API keys', () => {
  const logger = createTestLogger()

  const manager = new SetupManager(logger)
  manager.runSetup({ provider: 'openai', apiKey: 'sk-secret123' })

  const logs = logger.getAllLogs()
  expect(logs.join('\n')).not.toContain('sk-secret123')
})
```

### File System Safety

**Test Cases**:
- [ ] Only writes to workspace `.vscode/` directory
- [ ] Creates parent directories with correct permissions
- [ ] Handles read-only file systems gracefully
- [ ] Does not follow symlinks outside workspace

## Monitoring and Observability

### Telemetry Events (Future Consideration)

If we add telemetry in future versions:
- Setup started (provider)
- Setup completed (duration, success/failure)
- Setup cancelled (stage)
- Error encountered (type, provider)

### Output Channel Logging

Already have output channel. Ensure it logs:
- Setup start/complete
- CLI stdout/stderr
- Error details
- Configuration paths

**Example**:
```typescript
outputChannel.appendLine(`[Setup] Starting with provider: ${provider}`)
outputChannel.appendLine(`[Setup] Running: npx @crewchief/maproom-mcp setup --provider=${provider}`)
// ... stream CLI output ...
outputChannel.appendLine(`[Setup] ✓ Complete in ${duration}ms`)
outputChannel.appendLine(`[Setup] MCP config written to: ${configPath}`)
```

## Known Limitations

### MVP Scope

**Explicitly Out of Scope**:
- Automatic Docker installation
- Container lifecycle management (start/stop individual services)
- Custom Docker Compose configurations
- Multi-workspace orchestration
- Remote development (SSH, WSL, Remote-Containers)

**Why**: These add complexity without clear user value. Ship simple version first, iterate based on real usage.

### Future Test Needs

**If We Add These Features**:
- **Container Management**: Test start/stop/restart of individual services
- **Custom Configs**: Validate user-provided docker-compose.yml files
- **Remote Dev**: Test SSH/WSL/Remote-Container scenarios
- **Resource Limits**: Validate memory/CPU constraints

## Definition of Done

A ticket is "done" when:

1. **Unit Tests Pass**: All new code has unit tests covering happy path + critical errors
2. **Integration Tests Pass**: Component interactions validated
3. **Manual Testing Complete**: Checklist items verified for the feature
4. **No Regressions**: Existing functionality still works
5. **Documentation Updated**: README reflects new capabilities
6. **Code Reviewed**: At least one other person (or AI agent) has reviewed
7. **Linting Passes**: No new lint violations
8. **Performance Acceptable**: No slowdown in activation or operation

## Testing Timeline

### During Implementation

- Write unit tests alongside code (TDD-style)
- Run tests locally before committing
- Fix failures immediately (don't accumulate test debt)

### Before PR

- Complete manual testing checklist
- Run full test suite
- Verify CI passes

### Before Release

- Complete acceptance criteria
- Test VSIX installation from scratch
- Verify on clean machine if possible

## Conclusion

This quality strategy focuses on **confidence over coverage**. We test the critical paths that could cause user pain (process management, configuration, cancellation) while accepting manual testing for UI/UX flows that are harder to automate.

The goal is not 100% test coverage - it's zero surprises for users. Tests are a tool to achieve that, not an end in themselves.

By keeping the test suite focused and fast, we encourage running it frequently, which is more valuable than a comprehensive but slow suite that developers avoid running.

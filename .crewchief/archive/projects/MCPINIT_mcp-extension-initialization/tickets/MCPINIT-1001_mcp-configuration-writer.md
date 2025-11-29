# Ticket: MCPINIT-1001: Implement MCP Configuration Writer for Maproom MCP Server Registration

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- Test file existence alone does NOT satisfy this requirement

## Agents
- vscode-extension-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary

Create a utility class (`MCPConfigWriter`) that writes `.vscode/mcp.json` to register the Maproom MCP server with VS Code's MCP client. This utility will:
- Write proper MCP server configuration using versioned CLI reference (`@crewchief/maproom-mcp@2.2.1`)
- Merge with existing MCP servers (preserve user's other configurations)
- Use environment variable syntax for credentials (no plaintext secrets)
- Validate workspace paths to prevent security issues
- Support all three embedding providers (OpenAI, Google, Ollama)

This is the foundation component for the setup wizard integration (MCPINIT-1002).

## Background

The VSCode Maproom extension currently requires users to manually run `npx @crewchief/maproom-mcp setup --provider=<provider>` and manually create `.vscode/mcp.json` configuration. This creates significant friction during onboarding.

The existing CLI at `packages/maproom-mcp/bin/cli.cjs` (1,972 lines) already handles ALL Docker orchestration, health checking, and service management. The extension should NOT duplicate this - instead, it should follow the MCP best practice pattern: **register the MCP server, don't manage it**.

**Key Discovery**: The extension's ONLY job is to write `.vscode/mcp.json` pointing to the CLI. VS Code's MCP client handles invoking the CLI and managing its lifecycle. This is analogous to language server protocol - extensions register servers, they don't manage them.

This ticket implements the configuration writer component from Phase 1 of the MCP Extension Initialization project plan.

## Acceptance Criteria

### Core Functionality
- [ ] `MCPConfigWriter` class writes `.vscode/mcp.json` to workspace root
- [ ] Uses versioned reference: `@crewchief/maproom-mcp@2.2.1` (from `MAPROOM_MCP_VERSION` constant)
- [ ] Preserves existing `mcpServers` entries (merges, doesn't overwrite)
- [ ] Uses environment variable syntax: `${env:OPENAI_API_KEY}`, `${env:GOOGLE_APPLICATION_CREDENTIALS}`
- [ ] Supports all three providers: `openai`, `google`, `ollama`
- [ ] Creates `.vscode/` directory if missing
- [ ] Returns clear error if no workspace folder is open

### Security
- [ ] Validates all file paths are within workspace (no path traversal)
- [ ] Never writes plaintext credentials to config files
- [ ] All paths use `path.join()` for cross-platform compatibility

### Testing
- [ ] Unit tests: Config generation for each provider (openai, google, ollama)
- [ ] Unit tests: Merging preserves existing MCP servers
- [ ] Unit tests: Path validation rejects traversal attempts
- [ ] Integration tests: Writes to temp directory successfully
- [ ] All tests pass in `packages/vscode-maproom/`

## Technical Requirements

### File Creation

**File**: `packages/vscode-maproom/src/config/mcp-writer.ts` (~80 lines)

**Interface**:
```typescript
export interface MCPConfig {
  mcpServers: Record<string, MCPServerConfig>
}

export interface MCPServerConfig {
  command: string
  args: string[]
  env?: Record<string, string>
}

export class MCPConfigWriter {
  /**
   * Register Maproom MCP server in workspace .vscode/mcp.json
   * @param workspaceRoot Absolute path to workspace root
   * @param provider Embedding provider (openai, google, ollama)
   * @throws Error if workspace path invalid or outside workspace
   */
  async registerMCPServer(workspaceRoot: string, provider: string): Promise<void>

  /**
   * Build environment variables for provider
   * @param provider Embedding provider
   * @returns Environment variable object with ${env:...} syntax
   */
  private buildEnvironment(provider: string): Record<string, string>
}
```

### Version Constant

**File**: `packages/vscode-maproom/src/constants.ts` (create if not exists)

```typescript
/**
 * MCP server version pinned to match extension compatibility.
 * Update this when coordinating releases between vscode-maproom and maproom-mcp.
 */
export const MAPROOM_MCP_VERSION = '2.2.1'
```

### Configuration Format

The writer must generate this exact format:

```json
{
  "mcpServers": {
    "maproom": {
      "command": "npx",
      "args": ["-y", "@crewchief/maproom-mcp@2.2.1"],
      "env": {
        "OPENAI_API_KEY": "${env:OPENAI_API_KEY}"
      }
    }
  }
}
```

**Environment variables by provider**:
- **OpenAI**: `{ "OPENAI_API_KEY": "${env:OPENAI_API_KEY}" }`
- **Google**: `{ "GOOGLE_APPLICATION_CREDENTIALS": "${env:GOOGLE_APPLICATION_CREDENTIALS}" }`
- **Ollama**: `{}` (no environment variables needed)

### Path Validation

```typescript
import * as path from 'path'

// REQUIRED: Validate path is within workspace
const resolvedConfig = path.resolve(configPath)
const resolvedWorkspace = path.resolve(workspaceRoot)

if (!resolvedConfig.startsWith(resolvedWorkspace)) {
  throw new Error('Invalid path: configuration file must be within workspace')
}
```

### Merge Logic

```typescript
// Read existing config or create new
let config: MCPConfig = { mcpServers: {} }
if (fs.existsSync(configPath)) {
  const content = await fs.promises.readFile(configPath, 'utf-8')
  config = JSON.parse(content)
}

// Ensure mcpServers object exists
config.mcpServers = config.mcpServers || {}

// Add/update maproom server (preserves other servers)
config.mcpServers.maproom = {
  command: 'npx',
  args: ['-y', `@crewchief/maproom-mcp@${MAPROOM_MCP_VERSION}`],
  env: this.buildEnvironment(provider)
}
```

## Implementation Notes

### Design Pattern: Configuration Writer Only

This component is DELIBERATELY simple - it only writes configuration. It does NOT:
- ❌ Spawn processes or manage CLI lifecycle
- ❌ Monitor service health
- ❌ Handle Docker orchestration
- ❌ Validate credentials
- ❌ Check if services are running

**Why?** Those responsibilities belong to:
- **CLI** (`@crewchief/maproom-mcp`): Handles Docker, health checking, service management
- **VS Code MCP Client**: Invokes CLI based on `.vscode/mcp.json`
- **Setup Wizard** (MCPINIT-1002): Collects user input

This ticket creates the "registration utility" that the wizard will use.

### Testing Strategy

Focus on configuration correctness and security:

**High-Priority Tests**:
1. Config generation produces valid JSON
2. Environment variable syntax is correct (no plaintext secrets)
3. Merging preserves existing MCP servers
4. Path validation prevents traversal

**Medium-Priority Tests**:
5. Provider-specific environment variables
6. Directory creation
7. Error messages are clear

**Low-Priority** (manual testing):
8. VS Code actually reads the config correctly
9. CLI invocation works (verified in MCPINIT-1002)

### Cross-Platform Considerations

- Use `path.join()` for all path operations (Windows compatibility)
- Use `fs.promises` for async file operations
- Use `recursive: true` for `mkdir` (idempotent)

### Documentation

Add JSDoc comments explaining:
- Purpose of environment variable syntax
- Why we merge instead of overwrite
- Security considerations for path validation

## Dependencies

**None** - This ticket has no dependencies and can be implemented first.

## Risk Assessment

### Risk 1: Path Traversal Vulnerability
- **Impact**: Malicious input could write config outside workspace
- **Mitigation**: Path validation with `path.resolve()` and `startsWith()` check
- **Test Coverage**: Unit test for `../../etc/passwd` attack

### Risk 2: Overwriting User's MCP Config
- **Impact**: User loses other MCP server configurations
- **Mitigation**: Always read existing config and merge
- **Test Coverage**: Unit test verifies preservation of existing servers

### Risk 3: Plaintext Credentials in Config
- **Impact**: API keys exposed in `.vscode/mcp.json`
- **Mitigation**: Use `${env:VAR_NAME}` syntax, never write actual secrets
- **Test Coverage**: Unit test verifies no actual credentials in output

## Files/Packages Affected

### Files to Create
- `packages/vscode-maproom/src/config/mcp-writer.ts` (~80 lines) - Main implementation
- `packages/vscode-maproom/src/constants.ts` (~10 lines) - Version constant [if not exists]
- `packages/vscode-maproom/src/config/mcp-writer.test.ts` (~150 lines) - Test suite

### Files to Read (for context)
- `packages/vscode-maproom/src/ui/setupWizard.ts` - See how providers are currently handled
- `packages/vscode-maproom/package.json` - Verify test script configuration
- `.crewchief/projects/MCPINIT_mcp-extension-initialization/planning/version-strategy.md` - Version pinning strategy
- `.crewchief/projects/MCPINIT_mcp-extension-initialization/planning/security-review.md` - Security requirements

### Package Context
- **Package**: `packages/vscode-maproom`
- **Test Command**: `pnpm test` (runs Vitest)
- **Build Command**: `pnpm build`
- **Target Version**: 0.2.0 (feature addition)

## Related Documentation

- [Planning: Architecture](../planning/architecture.md) - System design showing config writer role
- [Planning: Version Strategy](../planning/version-strategy.md) - Version pinning approach
- [Planning: Security Review](../planning/security-review.md) - File system security
- [Planning: Quality Strategy](../planning/quality-strategy.md) - Testing approach
- [VS Code MCP Documentation](https://code.visualstudio.com/docs/copilot/customization/mcp-servers) - Official MCP config format

## Definition of Done

- [ ] `MCPConfigWriter` class implemented in `src/config/mcp-writer.ts`
- [ ] `MAPROOM_MCP_VERSION` constant defined in `src/constants.ts`
- [ ] All 15 acceptance criteria met
- [ ] Unit tests written and passing (70%+ coverage target)
- [ ] Integration tests written and passing
- [ ] Code follows TypeScript best practices
- [ ] JSDoc comments added to public methods
- [ ] No lint violations
- [ ] Files use ESM modules (import/export)
- [ ] Ready for integration with setup wizard (MCPINIT-1002)

---

**Project**: MCPINIT - MCP Extension Initialization
**Phase**: 1 (Foundation)
**Estimated Complexity**: Low
**Estimated Time**: 2-3 hours
**Next Ticket**: MCPINIT-1002 (Setup Wizard Integration)

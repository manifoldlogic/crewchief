# Architecture: MCP Extension Initialization

## Design Principles

1. **Reuse Over Rebuild**: Invoke the proven CLI instead of reimplementing Docker orchestration
2. **Progressive Enhancement**: Work without setup, but offer setup when needed
3. **Transparent Operations**: Show users what's happening, don't hide complexity
4. **Fail Gracefully**: Provide recovery paths when things go wrong
5. **MVP Focus**: Ship value quickly, iterate based on real usage

## System Overview

```
┌─────────────────────────────────────────────────────────┐
│                    VSCode Extension                      │
│                                                          │
│  ┌──────────────┐          ┌────────────────────────┐  │
│  │   Existing   │          │    NEW: MCP Config     │  │
│  │   Setup      │─────────▶│    Writer              │  │
│  │   Wizard     │          │    (80 lines)          │  │
│  └──────────────┘          └────────────────────────┘  │
│                                      │                   │
│                                      ↓                   │
│                             Writes .vscode/mcp.json     │
│                                                          │
└──────────────────────────────────────────────────────────┘

Extension's job complete.

Later, when VS Code needs MCP server:

                VS Code MCP Client
                        │
                        │ reads config
                        ▼
                  .vscode/mcp.json
                        │
                        │ invokes
                        ▼
┌────────────────────────────────────────────────────────┐
│   npx @crewchief/maproom-mcp@2.2.1                    │
│                                                         │
│   CLI handles entire lifecycle:                        │
│   - Docker Compose orchestration                       │
│   - PostgreSQL + pgvector setup                        │
│   - Ollama/OpenAI configuration                        │
│   - Health checking                                     │
│   - MCP server communication                           │
└────────────────────────────────────────────────────────┘
```

**Key Principle**: Extension registers MCP server, doesn't manage it. CLI is self-contained.

## Core Components

### 1. Setup Wizard (`src/ui/setupWizard.ts`)

**Responsibility**: Guide users through initial configuration

**Existing Foundation**: We already have `setupWizard.ts` with provider selection UI

**Enhancement Strategy**:
```typescript
interface SetupWizardFlow {
  // Step 1: Welcome & check prerequisites
  checkDocker(): Promise<boolean>

  // Step 2: Provider selection (existing)
  selectProvider(): Promise<'openai' | 'google' | 'ollama'>

  // Step 3: Credentials (existing)
  collectCredentials(provider): Promise<ProviderConfig>

  // Step 4: Run setup (NEW)
  runSetup(provider, config): Promise<SetupResult>

  // Step 5: Register MCP (NEW)
  registerMCPServer(config): Promise<void>

  // Step 6: Success confirmation
  showSuccess(): Promise<void>
}
```

**UI Pattern**: Multi-step QuickPick with progress notifications

### 2. MCP Configuration Writer (NEW: `src/config/mcp-writer.ts`)

**Responsibility**: Write `.vscode/mcp.json` with Maproom MCP server registration

**Simplified Implementation** (~80 lines):
```typescript
import { MAPROOM_MCP_VERSION } from '../constants'

class MCPConfigWriter {
  async registerMCPServer(workspaceRoot: string, provider: string): Promise<void> {
    const configPath = path.join(workspaceRoot, '.vscode', 'mcp.json')

    // Read existing config or create new
    let config: MCPConfig = { mcpServers: {} }
    if (fs.existsSync(configPath)) {
      config = JSON.parse(fs.readFileSync(configPath, 'utf-8'))
    }

    // Add/update maproom server
    config.mcpServers.maproom = {
      command: 'npx',
      args: ['-y', `@crewchief/maproom-mcp@${MAPROOM_MCP_VERSION}`],
      env: this.buildEnvironment(provider)
    }

    // Write back
    await fs.promises.mkdir(path.dirname(configPath), { recursive: true })
    await fs.promises.writeFile(configPath, JSON.stringify(config, null, 2))
  }

  private buildEnvironment(provider: string): Record<string, string> {
    switch (provider) {
      case 'openai': return { OPENAI_API_KEY: '${env:OPENAI_API_KEY}' }
      case 'google': return { GOOGLE_APPLICATION_CREDENTIALS: '${env:GOOGLE_APPLICATION_CREDENTIALS}' }
      case 'ollama': return {}
    }
  }
}
```

**Key Points**:
- Merges with existing MCP servers (doesn't overwrite)
- Uses environment variable syntax for VS Code to resolve
- Creates `.vscode/` directory if missing
- Validates workspace exists before writing

### 3. Extension Activation Flow (MODIFY: `src/extension.ts`)

**Simplified Flow**:
```typescript
export async function activate(context: vscode.ExtensionContext) {
  // Register commands
  registerSetupCommand(context)

  // Check if MCP config exists
  const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath
  if (!workspaceRoot) return // No workspace, skip setup prompt

  const mcpConfigPath = path.join(workspaceRoot, '.vscode', 'mcp.json')
  const configExists = fs.existsSync(mcpConfigPath)

  if (!configExists) {
    // First time - prompt for setup
    const action = await vscode.window.showInformationMessage(
      'Maproom MCP server not configured. Run setup?',
      'Run Setup',
      'Remind Me Later'
    )

    if (action === 'Run Setup') {
      await vscode.commands.executeCommand('maproom.setup')
    }
  }
}
```

**Key Changes**:
- No subprocess management
- No status monitoring
- Just check for config file and prompt if missing
- Extension delegates everything to MCP CLI

## Version Strategy

### Pinned MCP Version

**Challenge**: Extension and MCP server must use compatible versions to avoid schema mismatches.

**Solution**: Pin exact MCP version in extension code:

```typescript
// src/constants.ts
/**
 * MCP server version compatible with this extension
 * Update when maproom-mcp has breaking changes or new features
 */
export const MAPROOM_MCP_VERSION = '2.2.1'
```

**Benefits**:
- ✅ Guaranteed compatibility between extension and MCP server
- ✅ Predictable behavior (no surprise updates)
- ✅ Easy to test specific version combinations
- ✅ Users automatically get correct version via npx

**Maintenance**:
- Sync script: `pnpm sync:versions` updates constant from maproom-mcp package.json
- CI check ensures versions stay aligned
- Extension version bump when updating MCP version

See `version-strategy.md` for complete version management approach.

## Technology Choices

### Use Existing Tools

| Capability | Technology | Rationale |
|------------|------------|-----------|
| **Docker Orchestration** | Docker Compose (via CLI) | Already proven, handles complexity |
| **Process Management** | Node.js `child_process.spawn` | Built-in, handles streams well |
| **Progress UI** | `vscode.window.withProgress` | Native, cancellable, familiar |
| **Output Streaming** | `vscode.OutputChannel` | Standard pattern for CLI tools |
| **Status Display** | `vscode.StatusBarItem` | Persistent, non-intrusive |
| **MCP Registration** | `.vscode/mcp.json` | VS Code native format |

### Dependencies

**New Dependencies**: NONE

**Leverage Existing**:
- `@crewchief/maproom-mcp` package (already a dependency)
- VS Code Extension API (already available)
- Node.js built-ins (`child_process`, `fs`, `path`)

## File Structure

**New Files** (~150 lines total):
```
src/
├── extension.ts           # Modified: Add setup prompt (~20 lines)
├── constants.ts           # NEW: MAPROOM_MCP_VERSION = '2.2.1'
├── config/
│   └── mcp-writer.ts     # NEW: Write .vscode/mcp.json (80 lines)
└── ui/
    └── setupWizard.ts    # Enhanced: Call mcp-writer after provider selection (+50 lines)
```

**Existing Files** (unchanged):
```
src/
├── config/
│   └── secrets.ts        # Already handles credentials
└── docker/
    └── manager.ts        # Already manages Docker (unused in new approach)
```

## Data Flow

**Simplified Setup Flow**:

```
User activates extension
  ↓
No .vscode/mcp.json found
  ↓
Prompt: "Run Setup?"
  ↓
User clicks "Run Setup"
  ↓
Existing Setup Wizard
  ├─ Select Provider (existing)
  └─ Enter Credentials (existing)
  ↓
NEW: MCPConfigWriter
  └─ Write .vscode/mcp.json
  ↓
Show: "Restart VS Code to activate"
  ↓
User restarts VS Code
  ↓
VS Code MCP Client reads .vscode/mcp.json
  ↓
VS Code invokes: npx @crewchief/maproom-mcp@2.2.1
  ↓
CLI manages entire Docker stack
```

**Key Insight**: Extension's job ends at writing config. VS Code + CLI handle the rest.

## Anti-Pattern: Why Not Wrap the CLI?

### The Temptation

It's tempting to have the extension spawn `npx @crewchief/maproom-mcp setup` as a subprocess
and parse its output to show progress notifications.

### Why This Is Wrong

1. **Coupling**: Extension becomes tightly coupled to CLI output format
   - CLI output changes break extension
   - Need to maintain two parsers (CLI's own + extension's)
   - Version skew causes failures

2. **Duplication**: Extension reimplements CLI error handling
   - CLI already has comprehensive error handling
   - Extension adds second layer with different semantics
   - Maintenance burden multiplied

3. **Complexity**: Need to manage child process lifecycle
   - Spawn, monitor, kill, timeout handling
   - Cross-platform process management differences
   - Zombie process prevention
   - Signal handling (SIGTERM, SIGKILL)

4. **Unnecessary Abstraction**: Adds layer that provides no value
   - User can run `npx @crewchief/maproom-mcp setup` directly
   - Extension wrapper doesn't improve experience
   - Just adds potential failure points

### The Correct Pattern

**MCP servers are self-contained executables.** Think of them like language servers.

The extension's job is to:
1. Help users configure the server (provider selection, credentials)
2. Register the server in `.vscode/mcp.json`
3. **That's it.**

VS Code's MCP client invokes the server directly when needed.
The server manages its own lifecycle (including Docker containers).

### Analogy: Language Servers

VS Code extensions for TypeScript, Python, etc. follow this pattern:

```
Extension:
- Provides configuration UI
- Writes settings.json
- Registers language server location

VS Code:
- Invokes language server when needed
- Manages communication

Language Server:
- Self-contained executable
- Manages own lifecycle
- Handles requests independently
```

The Maproom MCP server follows the exact same pattern. The CLI is the executable.
Extension just tells VS Code where to find it.

## Performance Considerations

### Activation Time

**Goal**: Extension activates in <100ms

**Strategy**:
- Don't block on service checks during activation
- Start status monitoring asynchronously
- Defer setup wizard until user interaction

**Measurement**:
```typescript
const activationStart = Date.now()
export async function activate(context: vscode.ExtensionContext) {
  // ... activation code ...
  const activationTime = Date.now() - activationStart
  console.log(`Maproom activated in ${activationTime}ms`)
}
```

### Setup Time

**Expected**: 2-5 minutes (unchanged from CLI)

**Breakdown**:
- Docker image downloads: 1-3 minutes (first time only)
- Ollama model download: 1-2 minutes (Ollama only)
- Container startup: 10-30 seconds
- Service validation: 5-10 seconds

**Optimization**: Progress reporting keeps user informed, cancel button provides escape hatch

### Status Monitoring

**Check Interval**: 30 seconds (configurable)

**Cost**: Single TCP connection attempt to PostgreSQL (~5ms)

**Optimization**: Only check when workspace is active (pause when VS Code minimized)

## Constraints and Trade-offs

### Constraints

1. **Docker Required**: Users must have Docker Desktop installed
2. **Network Access**: Required for downloading images (Ollama especially)
3. **Disk Space**: ~2GB for images + models
4. **Platform Support**: Same as Docker Desktop (Windows, macOS, Linux)

### Trade-offs

| Decision | Pro | Con | Rationale |
|----------|-----|-----|-----------|
| **Invoke CLI vs Reimplement** | Reuses proven code, simpler | Extra `npx` overhead (~1s) | Simplicity wins, 1s is acceptable |
| **Workspace config vs Global** | Team can share setup | Per-workspace setup needed | Better security, team benefit |
| **Auto-setup vs Manual** | Better onboarding | Surprising behavior | Prompt with clear choice |
| **Status polling vs Events** | Simple, reliable | Slightly wasteful | 30s interval is negligible |

## MVP Scope

**In Scope**:
- ✅ Setup wizard that invokes CLI
- ✅ Progress notification during setup
- ✅ MCP server registration
- ✅ Status bar monitoring
- ✅ Error recovery commands
- ✅ Output channel for logs

**Out of Scope (Intentionally Deferred)**:
- ❌ Automatic dependency installation (Docker, npx)
- ❌ Container lifecycle management (start/stop/restart individual services)
- ❌ Custom Docker Compose configurations
- ❌ Multi-workspace support
- ❌ Remote development scenarios (SSH, WSL)

## Future Considerations

### Extensibility

The architecture allows future enhancements without breaking changes:

1. **Container Management** (if user demand warrants)
   - Add `docker compose` command wrappers
   - Provide granular control (restart just PostgreSQL)
   - Show container logs in Output channel

2. **Advanced Configuration** (if user demand warrants)
   - Custom ports and hostnames
   - Multiple embedding providers simultaneously
   - Resource limits and performance tuning

3. **Remote Development** (if user demand warrants)
   - Detect SSH/WSL/Remote-Containers
   - Handle Docker-in-Docker scenarios
   - Forward ports automatically

**Priority**: Evaluate based on user feedback after MVP release. Ship simple version first.

### Maintainability

**Coupling Points**:
- CLI output format (for progress parsing)
- MCP configuration schema
- Docker Compose service names

**Mitigation**:
- Parse CLI output defensively (regex with fallbacks)
- Version check CLI before setup
- Document assumptions in code comments

**Testing Strategy**: See `quality-strategy.md`

## Conclusion

This architecture achieves simplicity by **delegating complexity to proven components**. The CLI already handles Docker orchestration perfectly - we just wrap it in VS Code-native UI patterns. This approach:

- **Ships value quickly**: Weeks, not months
- **Leverages existing code**: No duplication
- **Follows VS Code patterns**: Feels native
- **Provides recovery paths**: Users never stuck
- **Enables future enhancements**: Clear extension points

The "complexity" from previous attempts was likely architectural - trying to own orchestration rather than delegate it. This design inverts that relationship: the extension is the thin UI layer, the CLI is the orchestration engine.

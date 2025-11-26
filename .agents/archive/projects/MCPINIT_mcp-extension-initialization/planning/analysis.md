# Analysis: MCP Extension Initialization

## Problem Definition

The VSCode Maproom extension currently requires users to manually run `npx @crewchief/maproom-mcp setup --provider=<provider>` before using the extension. This creates friction in the onboarding experience and leads to confusion when the extension fails to connect to services that haven't been initialized.

**User's Observation**: "Since I can do it with a single command line command, I don't understand why it can't be simple."

The core challenge is bridging the gap between:
- **What works**: Single CLI command (`npx @crewchief/maproom-mcp setup --provider=openai`)
- **What's needed**: Seamless VSCode extension initialization that orchestrates Docker containers

### Previous Attempts

We attempted container orchestration before and found it "challenging." However, the existence of a working CLI command suggests the complexity may have been in how we approached it, not the task itself.

## Discovery: Existing Infrastructure

During detailed code review, critical infrastructure was discovered that eliminates the need for complex container orchestration:

### MCP CLI Already Complete (`packages/maproom-mcp/bin/cli.cjs`)

The MCP CLI (1,972 lines) already implements everything needed:

**Docker Orchestration** (lines 231-824):
- `checkDockerDaemon()` - Validates Docker is running
- `checkDockerCompose()` - Verifies docker compose available
- `setupConfigDirectory()` - Creates `~/.maproom-mcp/` config
- `startDockerCompose()` - Orchestrates PostgreSQL, Ollama, MCP containers
- `ensureCleanState()` - Handles container cleanup

**Provider Management** (lines 1156-1296):
- `validateProviderConfig()` - Validates OpenAI/Google/Ollama config
- `validateProviderRequirements()` - Checks credentials present
- Provider-specific setup for each embedding service

**Health Checking** (lines 936-1070):
- `waitForServicesHealthy()` - Exponential backoff retry logic
- `verifyFinalState()` - Validates all services running
- Comprehensive diagnostic logging

**Complete Setup Command** (line 1786):
- `runSetup()` - Orchestrates entire Docker stack
- Database schema initialization
- Service validation
- Configuration persistence

### VSCode Extension Already Has UI (`packages/vscode-maproom/src/`)

**Setup Wizard** (`src/ui/setupWizard.ts` - 285 lines):
- Provider selection QuickPick UI
- Ollama auto-detection
- Password-masked credential input
- Saves to workspace state

**Secrets Management** (`src/config/secrets.ts` - 222 lines):
- Complete SecretStorage wrapper
- Provider-specific environment variables
- Secure credential storage/retrieval

**Docker Management** (`src/docker/manager.ts` - 500+ lines):
- Docker Compose lifecycle management
- Health checking
- Error handling

### Revised Approach

**Original Plan**: Build SetupManager to spawn CLI, parse output, manage Docker
**Problem**: This creates wrapper around wrapper - CLI already manages Docker!

**Correct Approach**:
1. Use EXISTING setupWizard.ts for provider selection
2. Use EXISTING SecretsManager for credential storage
3. NEW: Write `.vscode/mcp.json` pointing to `@crewchief/maproom-mcp@{VERSION}`
4. Done. VS Code invokes CLI when needed, CLI manages Docker

**Work Reduction**: 700 lines → 150 lines (78% reduction)

## Current State

### What Works Today

1. **CLI Tool** (`@crewchief/maproom-mcp`):
   - Single command setup: `npx @crewchief/maproom-mcp setup --provider=<provider>`
   - Orchestrates Docker Compose for PostgreSQL + pgvector
   - Handles provider-specific configuration (OpenAI, Google, Ollama)
   - Validates services are running
   - Takes 2-5 minutes on first run

2. **Extension Configuration** (v0.1.1):
   - Database connection settings
   - Auto-detects devcontainer environment
   - Defaults to `localhost` or `maproom-postgres`

### What Doesn't Work

1. **Manual Setup Required**: Users must know to run the CLI command
2. **No Automated MCP Registration**: Extension doesn't register itself as MCP server
3. **No Container Lifecycle Management**: Extension doesn't start/stop containers
4. **Poor Error Messages**: "PostgreSQL not available" doesn't guide users to setup

## Industry Solutions

Based on research of VSCode extension and MCP best practices:

### MCP Server Installation Patterns (2025)

**Official VSCode MCP Integration** ([source](https://code.visualstudio.com/api/extension-guides/ai/mcp)):
- Extensions can programmatically register MCP servers via VS Code API
- Configuration stored in `.vscode/mcp.json` (workspace) or user profile (global)
- VS Code provides `@mcp` search in Extensions view
- "Install in VS Code" buttons on websites using `vscode://` URLs
- Auto-discovery from Claude Desktop configurations

**Security Best Practices** ([source](https://code.visualstudio.com/docs/copilot/customization/mcp-servers)):
- MCP servers run arbitrary code - only add from trusted sources
- Prefer workspace-level over global configuration
- Avoid hardcoding credentials - use environment variables or input variables
- Review publisher and configuration before starting servers

**Configuration Options**:
1. **Workspace `.vscode/mcp.json`**: Shared with team, project-specific
2. **User Profile**: Global, reused across projects
3. **Programmatic Registration**: Extension calls VS Code API
4. **Command Line**: `code --add-mcp=<config>`

### Docker Container Lifecycle in Extensions

**Microsoft's Dev Containers Extension** ([source](https://code.visualstudio.com/docs/devcontainers/containers)):
- Uses `devcontainer.json` for declarative container lifecycle
- Lifecycle hooks: `onCreateCommand`, `postCreateCommand`, `updateContentCommand`
- Extensions can trigger container operations via Docker API
- Container Explorer view for management

**Docker DX Extension Best Practices** ([source](https://www.docker.com/blog/docker-dx-extension-for-vs-code/)):
- Inline Dockerfile linting with Build Checks
- Vulnerability scanning with Docker Scout
- Docker Compose IntelliSense and management
- Never hardcode credentials - use Docker secrets or environment variables

**Container Orchestration Patterns**:
1. **Docker API**: Direct calls to Docker daemon
2. **Docker Compose CLI**: Shell out to `docker compose` commands
3. **Child Process**: Spawn containers as subprocesses
4. **Kubernetes APIs**: For production orchestration (overkill for local dev)

### Successful Extension Patterns

**Azure MCP Server** ([source](https://learn.microsoft.com/en-us/azure/developer/azure-mcp-server/get-started/tools/visual-studio-code)):
- Single installation via VS Code Marketplace
- Configuration through VS Code settings
- No manual setup required beyond API keys

**Python MCP Servers** ([source](https://jtemporal.com/configure-local-python-mcp-server-in-vscode/)):
- Installed via pip/npm
- Configured in `.vscode/mcp.json`
- Extension handles server lifecycle

**Eclipse MCP Extension Pattern** ([source](https://eclipsesource.com/blogs/2025/06/12/installing-mcp-servers-via-vscode-extensions/)):
- Extensions can bundle MCP servers
- VS Code manages server lifecycle automatically
- Users only configure provider-specific settings (API keys)

## Gap Analysis

### What We Have vs. What We Need

| Capability | Current State | Desired State |
|------------|---------------|---------------|
| **MCP Registration** | Manual `.vscode/mcp.json` edit | Automatic via extension |
| **Container Setup** | Manual CLI command | Triggered by extension activation |
| **Provider Config** | Manual env vars | UI-guided setup wizard |
| **Error Handling** | Generic "not available" | Actionable "run setup" prompts |
| **Status Visibility** | Hidden in Docker | Extension status bar |
| **Updates** | Manual container restart | Detect changes, offer restart |

### Key Insights

1. **The CLI Already Solves It**: We have a battle-tested setup command that works. Don't rebuild it - invoke it.

2. **VSCode Has Native MCP Support**: We don't need custom registration logic. Use the platform APIs.

3. **Container Management Is Solved**: Docker Compose handles orchestration. We just need to trigger it.

4. **User Experience Gaps**:
   - No onboarding flow
   - No status visibility
   - No recovery guidance

5. **The "Simple Command" Works Because**:
   - It's synchronous and blocking (shows progress)
   - It validates everything before returning
   - It provides clear error messages
   - It's idempotent (safe to re-run)

## Root Cause of Previous Complexity

Likely reasons previous attempts were "challenging":

1. **Async Coordination**: Trying to manage container lifecycle asynchronously in extension activation
2. **Status Polling**: Complex logic to detect when containers are ready
3. **Error Recovery**: Handling partial failures mid-setup
4. **Progress Reporting**: Showing progress in VS Code UI while containers start

**The CLI Avoids These** by:
- Blocking until complete
- Using Docker Compose's built-in wait conditions
- Printing progress to stdout
- Exiting with clear success/failure codes

## Proposed Approach

**Key Insight**: Don't replicate the CLI's logic. *Invoke* it from the extension with proper UI integration.

### Simplification Strategy

1. **Reuse the CLI**: Run `npx @crewchief/maproom-mcp setup` as a subprocess
2. **Stream Output**: Show CLI progress in VS Code Output panel
3. **Wrap in UI**: Add setup wizard for provider selection and API keys
4. **Auto-Register MCP**: After setup succeeds, register MCP server configuration
5. **Status Bar Integration**: Show service status and provide quick actions

### When to Run Setup

Three opportunities:
1. **On First Activation**: Detect no configuration, prompt to setup
2. **Via Command**: "Maproom: Run Setup" command in command palette
3. **Auto-Recovery**: When services unavailable, offer "Run Setup" button

### MCP Registration Strategy

After successful setup:
```json
{
  "mcpServers": {
    "maproom": {
      "command": "npx",
      "args": ["@crewchief/maproom-mcp"],
      "env": {
        "OPENAI_API_KEY": "${env:OPENAI_API_KEY}"
      }
    }
  }
}
```

Write to workspace `.vscode/mcp.json` for team sharing.

## Research Sources

- [Use MCP servers in VS Code](https://code.visualstudio.com/docs/copilot/customization/mcp-servers)
- [MCP developer guide | VS Code Extension API](https://code.visualstudio.com/api/extension-guides/ai/mcp)
- [Installing MCP Servers via VS Code Extensions](https://eclipsesource.com/blogs/2025/06/12/installing-mcp-servers-via-vscode-extensions/)
- [Developing inside a Container](https://code.visualstudio.com/docs/devcontainers/containers)
- [Docker DX extension for VS Code](https://www.docker.com/blog/docker-dx-extension-for-vs-code/)
- [Azure MCP Server with Visual Studio Code](https://learn.microsoft.com/en-us/azure/developer/azure-mcp-server/get-started/tools/visual-studio-code)

## Conclusion

The path forward is clear: **invoke the existing CLI, don't replicate it**. The "complexity" was likely from trying to manage Docker directly rather than trusting the proven setup command. By wrapping the CLI in proper VS Code UI patterns (progress notifications, output streaming, command palette integration), we can deliver the "simple" experience the user expects while leveraging battle-tested orchestration logic.

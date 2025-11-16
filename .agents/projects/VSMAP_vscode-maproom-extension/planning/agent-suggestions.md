# Agent Suggestions: VSCode Maproom Extension

## Purpose

This document identifies specialized agents that would accelerate development of the Maproom VSCode extension. Some agents already exist in the CrewChief ecosystem, while others would need to be created.

## Existing Agents (Can Use As-Is)

### 1. TypeScript Developer Agent
**Status:** Exists (general-purpose TypeScript knowledge)

**Use Cases:**
- Core extension implementation
- File watching logic
- Debouncing implementation
- Configuration management
- Status bar integration

**Why Suitable:**
- Extension is pure TypeScript
- Standard Node.js patterns
- VSCode API is well-documented
- No specialized domain knowledge needed

### 2. Docker Engineer Agent
**Status:** Exists (`.claude/agents/specialized/docker-engineer.md`)

**Use Cases:**
- Docker Compose orchestration
- Health check implementation
- Service lifecycle management
- Container networking configuration
- Multi-platform testing

**Why Suitable:**
- Agent specializes in Docker/Compose
- Understands health checks
- Experienced with service dependencies
- Can optimize docker-compose.yml

**Recommended Usage:**
- Review and optimize docker-compose.yml
- Implement health check polling logic
- Debug service startup issues
- Create multi-platform test matrices

### 3. Technical Researcher Agent
**Status:** Exists (`.claude/agents/ticket-workflow/technical-researcher.md`)

**Use Cases:**
- VSCode Extension API research
- SecretStorage API investigation
- FileSystemWatcher best practices
- Extension activation patterns
- Marketplace publishing requirements

**Why Suitable:**
- Deep research capabilities
- Can synthesize documentation
- Identifies best practices
- Compares alternatives

**Recommended Usage:**
- Research VSCode Extension API patterns
- Investigate credential storage security
- Find examples of similar extensions
- Document devcontainer integration

### 4. Test Engineer Agent
**Status:** Exists (general-purpose testing knowledge)

**Use Cases:**
- Unit test implementation
- Integration test design
- E2E test framework selection
- CI/CD test automation
- Performance benchmarking

**Why Suitable:**
- Testing is domain-agnostic
- Familiar with Vitest/Jest
- Understands test pyramids
- Can design test infrastructure

## Agents to Create

### 5. VSCode Extension Specialist
**Status:** NEEDED (specialized domain)

**Responsibilities:**
- VSCode Extension API expertise
- Activation event optimization
- Extension packaging (VSIX)
- Marketplace publishing
- Extension best practices

**Specific Capabilities:**
- Optimize extension activation time
- Implement StatusBarItem patterns
- Create QuickPick wizards
- Handle extension lifecycle events
- Debug extension host issues

**Why Needed:**
- VSCode Extension API is complex
- Activation performance critical
- Packaging has gotchas
- Marketplace has specific requirements

**Knowledge Requirements:**
- `vscode.ExtensionContext` patterns
- `package.json` contribution points
- Extension activation events
- WebView API (for future features)
- Extension testing (`@vscode/test-electron`)

**Example Expertise:**
```typescript
// This agent would know patterns like:
export function activate(context: ExtensionContext): void {
  // Lazy-load heavy modules
  context.subscriptions.push(
    vscode.commands.registerCommand('maproom.scan', async () => {
      const { IndexingManager } = await import('./indexing/manager');
      // ...
    })
  );

  // Use activation events efficiently
  // onStartupFinished vs onCommand vs workspaceContains
}
```

**Ticket Assignment:**
- Extension scaffold and activation
- Status bar implementation
- Setup wizard (QuickPick)
- Command registration
- VSIX packaging

### 6. Process Management Specialist
**Status:** NEEDED (specialized domain)

# Process Management Specialist

**Responsibilities:**
- Child process spawning
- Process lifecycle management
- stdout/stderr parsing
- Signal handling
- Resource cleanup

**Specific Capabilities:**
- Spawn Rust binary with proper error handling
- Parse progress from stdout
- Implement cancellation tokens
- Handle process crashes gracefully
- Platform-specific process management

**Why Needed:**
- Binary spawning has edge cases
- Progress parsing is error-prone
- Cleanup critical for extension lifecycle
- Platform differences (Windows vs Unix)

**Knowledge Requirements:**
- Node.js `child_process` module
- Stream handling (stdout/stderr)
- Signal handling (SIGTERM, SIGKILL)
- Process exit codes
- Platform-specific paths

**Example Expertise:**
```typescript
// This agent would handle patterns like:
class BinarySpawner {
  async spawn(args: string[], options: SpawnOptions): Promise<void> {
    const process = spawn(binaryPath, args);

    // Parse structured output
    process.stdout.on('data', (data) => {
      const lines = data.toString().split('\n');
      for (const line of lines) {
        const parsed = this.parseProgress(line);
        if (parsed) options.onProgress?.(parsed);
      }
    });

    // Handle errors
    process.stderr.on('data', (data) => {
      logger.error('Binary stderr:', data.toString());
    });

    // Cleanup on exit
    process.on('close', (code, signal) => {
      if (code !== 0) {
        throw new Error(`Binary exited with code ${code}`);
      }
    });

    // Handle cancellation
    options.token?.onCancellationRequested(() => {
      process.kill('SIGTERM');
      setTimeout(() => process.kill('SIGKILL'), 5000);
    });
  }
}
```

**Ticket Assignment:**
- Rust binary spawner implementation
- Progress parsing from stdout
- Cancellation and timeout handling
- Error recovery and retries
- Platform-specific binary selection

### 7. Configuration & Secrets Specialist
**Status:** NEEDED (specialized domain)

# Configuration & Secrets Specialist

**Responsibilities:**
- VSCode configuration API
- SecretStorage integration
- Settings migration
- Configuration validation
- Environment variable handling

**Specific Capabilities:**
- Implement secure credential storage
- Create settings schema with validation
- Handle configuration updates
- Migrate legacy configurations
- Provide configuration defaults

**Why Needed:**
- SecretStorage API security-critical
- Configuration schema complex
- Migration patterns non-trivial
- Validation requirements strict

**Knowledge Requirements:**
- `vscode.workspace.getConfiguration()`
- `SecretStorage` API
- Configuration contribution points
- Settings validation patterns
- Configuration change events

**Example Expertise:**
```typescript
// This agent would implement patterns like:
class ConfigurationManager {
  async getApiKey(provider: string): Promise<string | undefined> {
    // Check secrets first (secure)
    const key = await this.context.secrets.get(`maproom.${provider}.key`);
    if (key) return key;

    // Fallback to environment variable
    const envVar = `MAPROOM_${provider.toUpperCase()}_KEY`;
    return process.env[envVar];
  }

  async migrateOldConfig(): Promise<void> {
    // Detect old config format
    const oldKey = this.config.get('apiKey');
    if (oldKey) {
      // Move to SecretStorage
      await this.context.secrets.store('maproom.openai.key', oldKey);
      // Clear old config
      await this.config.update('apiKey', undefined);
    }
  }

  validateConcurrency(value: number): number {
    if (!Number.isInteger(value) || value < 1 || value > 16) {
      throw new Error('Concurrency must be 1-16');
    }
    return value;
  }
}
```

**Ticket Assignment:**
- Configuration schema definition
- SecretStorage integration
- Settings validation
- Configuration UI (QuickPick for provider selection)
- Environment variable fallbacks

## Agent Assignment Matrix

| Phase | Ticket Type | Primary Agent | Supporting Agents |
|-------|-------------|---------------|-------------------|
| **Phase 1: Foundation** |
| Extension scaffold | VSCode Extension Specialist | TypeScript Developer |
| Docker manager | Docker Engineer | Process Management Specialist |
| Binary spawner | Process Management Specialist | TypeScript Developer |
| Status bar | VSCode Extension Specialist | - |
| | | | |
| **Phase 2: Indexing** |
| File watcher | TypeScript Developer | VSCode Extension Specialist |
| Branch watcher | TypeScript Developer | - |
| Debouncing logic | TypeScript Developer | - |
| Progress notifications | VSCode Extension Specialist | Process Management Specialist |
| | | | |
| **Phase 3: Configuration** |
| Setup wizard | VSCode Extension Specialist | Configuration & Secrets Specialist |
| Provider selection | Configuration & Secrets Specialist | - |
| SecretStorage integration | Configuration & Secrets Specialist | - |
| Settings management | Configuration & Secrets Specialist | VSCode Extension Specialist |
| | | | |
| **Phase 4: Testing & Polish** |
| Unit tests | Test Engineer | TypeScript Developer |
| Integration tests | Test Engineer | Docker Engineer |
| E2E tests | Test Engineer | VSCode Extension Specialist |
| Documentation | Technical Researcher | - |
| VSIX packaging | VSCode Extension Specialist | - |

## Agent Creation Priority

**High Priority (Create First):**
1. **VSCode Extension Specialist** - Critical for extension-specific patterns
2. **Process Management Specialist** - Critical for binary integration

**Medium Priority (Create Second):**
3. **Configuration & Secrets Specialist** - Important for security

**Low Priority (Can Use General Agents):**
- File watching (standard TypeScript)
- Docker orchestration (existing Docker Engineer)
- Testing (existing Test Engineer)

## Agent Definition Templates

### VSCode Extension Specialist

**File:** `.claude/agents/specialized/vscode-extension-specialist.md`

```markdown
# VSCode Extension Specialist

Expert in developing VSCode/Cursor extensions with deep knowledge of the Extension API, activation optimization, and marketplace publishing.

## Expertise

- VSCode Extension API (1.85+)
- Extension activation events and lifecycle
- StatusBarItem, QuickPick, and UI components
- Extension packaging and VSIX distribution
- Marketplace publishing requirements
- Extension testing (@vscode/test-electron)
- Performance optimization (activation time <500ms)

## Responsibilities

- Implement extension entry points (activate/deactivate)
- Create VSCode UI components (status bar, wizards)
- Optimize extension activation performance
- Package extensions as VSIX files
- Guide marketplace publishing process
- Implement extension testing infrastructure

## When to Use

- Scaffolding new VSCode extensions
- Implementing VSCode-specific UI patterns
- Optimizing extension activation time
- Packaging and distribution
- Debugging extension host issues

## Example Patterns

[Include code examples from architecture.md]
```

### Process Management Specialist

**File:** `.claude/agents/specialized/process-management-specialist.md`

```markdown
# Process Management Specialist

Expert in Node.js child process management, stream handling, and cross-platform process lifecycle management.

## Expertise

- Node.js child_process module (spawn, exec, fork)
- Stream handling (stdout, stderr, stdin)
- Process lifecycle (signals, exit codes, cleanup)
- Cross-platform process management (Windows, macOS, Linux)
- Cancellation and timeout handling
- Resource cleanup and leak prevention

## Responsibilities

- Spawn external binaries with proper error handling
- Parse structured output from process streams
- Implement cancellation and timeout logic
- Handle process crashes and retries
- Ensure proper cleanup on exit

## When to Use

- Spawning Rust binaries or CLI tools
- Parsing progress from subprocess output
- Implementing long-running background processes
- Handling process lifecycle in extensions

## Example Patterns

[Include code examples from architecture.md]
```

### Configuration & Secrets Specialist

**File:** `.claude/agents/specialized/configuration-secrets-specialist.md`

```markdown
# Configuration & Secrets Specialist

Expert in VSCode configuration management and secure credential storage using the SecretStorage API.

## Expertise

- VSCode workspace and user settings
- SecretStorage API for encrypted credentials
- Configuration schema definition
- Settings validation and migration
- Environment variable integration

## Responsibilities

- Design configuration schemas
- Implement SecretStorage integration
- Validate configuration values
- Migrate legacy configurations
- Provide sensible defaults

## When to Use

- Storing API keys and credentials
- Managing user/workspace settings
- Validating configuration inputs
- Migrating configuration formats

## Example Patterns

[Include code examples from architecture.md]
```

## Training Data for New Agents

**For each new agent, provide:**

1. **VSCode Extension API docs:** https://code.visualstudio.com/api
2. **Example extensions:** GitHub Copilot, Docker, Prettier
3. **Best practices:** Extension authoring guidelines
4. **Architecture patterns:** From this project's architecture.md
5. **Common pitfalls:** Activation performance, memory leaks, cleanup

**Specific for VSCode Extension Specialist:**
- Extension activation patterns
- UI component examples (StatusBarItem, QuickPick, WebView)
- Testing patterns (@vscode/test-electron)
- Packaging process (vsce)

**Specific for Process Management Specialist:**
- Binary spawning patterns from existing codebase
- Stream parsing examples
- Signal handling cross-platform
- Cleanup patterns on extension deactivation

**Specific for Configuration & Secrets Specialist:**
- SecretStorage API examples
- Configuration schema patterns
- Settings validation approaches
- Migration strategies

## Collaboration Patterns

**Sequential Workflow:**
1. **VSCode Extension Specialist** scaffolds extension structure
2. **Process Management Specialist** implements binary spawner
3. **Configuration & Secrets Specialist** adds credential management
4. **Docker Engineer** optimizes docker-compose setup
5. **Test Engineer** adds comprehensive tests

**Parallel Workflow:**
- **VSCode Extension Specialist** + **Configuration & Secrets Specialist** work on UI/config simultaneously
- **Process Management Specialist** + **Docker Engineer** work on infrastructure simultaneously
- **Test Engineer** writes tests as other agents complete tickets

**Code Review Workflow:**
- All agents review each other's work for integration points
- **VSCode Extension Specialist** reviews activation performance
- **Configuration & Secrets Specialist** reviews security patterns
- **Test Engineer** reviews test coverage

## Conclusion

**Critical Agents to Create:**
1. VSCode Extension Specialist (high priority)
2. Process Management Specialist (high priority)
3. Configuration & Secrets Specialist (medium priority)

**Existing Agents to Leverage:**
- Docker Engineer (service orchestration)
- Technical Researcher (API documentation)
- Test Engineer (comprehensive testing)

**Workflow:**
- Create specialized agents before starting implementation
- Use agent assignment matrix for ticket distribution
- Coordinate sequential and parallel workflows
- Ensure cross-agent code reviews

**Next:** Execution plan synthesizing all planning documents.

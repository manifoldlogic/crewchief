# Agent Suggestions: VSCode Maproom Extension

## Purpose

This document identifies specialized agents that would accelerate development of the Maproom VSCode extension. The simplified architecture (thin orchestration layer) requires fewer specialized agents than originally planned.

## Architecture Change Impact

**Original Plan:** 3 specialized agents (VSCode Extension Specialist, Process Management Specialist, Configuration & Secrets Specialist)

**Revised Plan:** 2 specialized agents (Process Management Specialist, VSCode Extension Specialist)

**Why Fewer Agents:**
- No FileWatcher/BranchWatcher implementation needed (Rust handles it)
- No debouncing logic needed (Rust handles it)
- Simpler code (~300 lines vs ~3000 lines)
- Focus on process spawning and stdout parsing

## Recommended Specialized Agents

### 1. process-management-specialist

**Purpose:** Expert in spawning and managing child processes in Node.js

**Core Competencies:**
- Child process spawning (`child_process.spawn()`)
- Stdout/stderr stream handling
- NDJSON parsing and structured logging
- Process lifecycle management (start/stop/restart)
- Graceful shutdown and cleanup
- Crash recovery with exponential backoff
- Cross-platform binary execution
- Platform detection (process.platform, process.arch)

**VSCode-Specific Knowledge:**
- Extension activation/deactivation lifecycle
- Process cleanup on extension deactivate
- Logging best practices for extensions

**Training Data Requirements:**
- Node.js child_process API documentation
- NDJSON parsing examples
- Exponential backoff patterns
- Circuit breaker implementations
- Platform-specific binary path resolution

**Example Tasks:**
- Spawn `crewchief-maproom watch` process
- Parse NDJSON output from Rust binary
- Implement crash recovery with backoff
- Kill processes gracefully on deactivate
- Handle stdout buffering and line splitting

**Complexity:** Medium
**Estimated Creation Time:** 2 hours

**Example Expertise:**
```typescript
// This agent would handle patterns like:
class ProcessOrchestrator {
  async spawn(command: string, args: string[]): Promise<void> {
    const process = spawn(command, args);

    // Parse NDJSON from stdout
    const lineParser = new LineParser();
    process.stdout.on('data', (data) => {
      lineParser.append(data);
      for (const line of lineParser.lines()) {
        try {
          const event = JSON.parse(line);
          this.handleEvent(event);
        } catch (err) {
          logger.warn('Malformed NDJSON:', line);
        }
      }
    });

    // Restart on crash with backoff
    process.on('exit', (code) => {
      if (code !== 0 && this.shouldRestart) {
        this.scheduleRestart();
      }
    });
  }

  scheduleRestart(): void {
    const delay = Math.min(1000 * Math.pow(2, this.restartCount), 30000);
    setTimeout(() => this.spawn(), delay);
  }
}
```

### 2. vscode-extension-specialist

**Purpose:** Expert in VSCode Extension API and extension development

**Core Competencies:**
- Extension activation events and performance (<500ms)
- StatusBarItem creation and updates
- QuickPick UI for user selections
- SecretStorage API for credentials
- Extension configuration and settings
- Progress notification API
- VSIX packaging and distribution

**Simplified Requirements (No FileSystemWatcher!):**
- No need for FileSystemWatcher knowledge (Rust handles it)
- No need for complex state machines (delegation simplifies)
- Focus on orchestration, not implementation

**Training Data Requirements:**
- VSCode Extension API documentation
- StatusBarItem examples
- SecretStorage usage patterns
- QuickPick UI examples
- Extension activation best practices

**Example Tasks:**
- Create status bar item with click handler
- Show QuickPick for provider selection
- Store API keys in SecretStorage
- Update status bar based on parsed output
- Package extension as VSIX

**Complexity:** Medium
**Estimated Creation Time:** 2 hours

**Example Expertise:**
```typescript
// This agent would know patterns like:
export function activate(context: ExtensionContext): void {
  // Lazy-load heavy modules for fast activation
  const statusBar = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Right,
    100
  );
  statusBar.text = '$(eye) Maproom';
  statusBar.tooltip = 'Click for status';
  statusBar.command = 'maproom.showStatus';
  statusBar.show();

  context.subscriptions.push(statusBar);

  // Spawn processes after activation completes
  setImmediate(() => {
    const orchestrator = new ProcessOrchestrator();
    orchestrator.spawnAll();
  });
}
```

## Agents NOT Needed

**❌ file-watcher-specialist** - Rust binary handles file watching
**❌ branch-watcher-specialist** - Rust binary handles branch detection
**❌ debounce-specialist** - Rust binary handles debouncing with --throttle
**❌ configuration-secrets-specialist** - VSCode Extension Specialist can handle this (simpler than originally planned)

## Existing Agents (Can Use As-Is)

### Docker Engineer Agent
**Status:** Exists (`.claude/agents/specialized/docker-engineer.md`)

**Use Cases:**
- Docker Compose orchestration
- Health check implementation
- Service lifecycle management
- Container networking configuration

**Why Suitable:**
- Extension only needs to spawn `docker-compose up/down`
- No complex Docker logic needed
- Existing agent can review docker-compose.yml

### Technical Researcher Agent
**Status:** Exists (`.claude/agents/ticket-workflow/technical-researcher.md`)

**Use Cases:**
- VSCode Extension API research
- SecretStorage API investigation
- Extension activation patterns
- Marketplace publishing requirements

### Test Engineer Agent
**Status:** Exists (general-purpose testing knowledge)

**Use Cases:**
- Unit test implementation
- Integration test design
- E2E test framework selection
- CI/CD test automation

## Agent Assignment Matrix

| Phase | Ticket Type | Primary Agent | Supporting Agents |
|-------|-------------|---------------|-------------------|
| **Phase 0: Agent Creation** |
| Create agents | Technical Researcher | - |
| Test agents | Test Engineer | - |
| | | | |
| **Phase 1: Core Infrastructure** |
| Extension scaffold | VSCode Extension Specialist | TypeScript Developer |
| Docker manager | Docker Engineer | Process Management Specialist |
| Process orchestrator | Process Management Specialist | - |
| NDJSON parser | Process Management Specialist | TypeScript Developer |
| Status bar | VSCode Extension Specialist | - |
| | | | |
| **Phase 2: Setup Wizard** |
| Setup wizard UI | VSCode Extension Specialist | - |
| Provider selection | VSCode Extension Specialist | - |
| SecretStorage integration | VSCode Extension Specialist | - |
| Initial scan trigger | Process Management Specialist | VSCode Extension Specialist |
| | | | |
| **Phase 3: Process Monitoring** |
| Crash recovery | Process Management Specialist | - |
| Exponential backoff | Process Management Specialist | - |
| Circuit breaker | Process Management Specialist | - |
| Status updates | VSCode Extension Specialist | Process Management Specialist |
| | | | |
| **Phase 4: Polish & Testing** |
| Unit tests | Test Engineer | Both Specialists |
| Integration tests | Test Engineer | Docker Engineer |
| E2E tests | Test Engineer | VSCode Extension Specialist |
| Documentation | Technical Researcher | - |
| VSIX packaging | VSCode Extension Specialist | - |

## Agent Testing Protocol

Before using agents on VSMAP:

1. **Create simple test extension** - "Hello World" with status bar
2. **Test process-management-specialist** - Spawn `echo` process, parse output
3. **Test vscode-extension-specialist** - Create status bar, QuickPick
4. **Verify agents work together** - Combined orchestration task

**Only proceed with VSMAP if agents pass tests.**

### Test Tasks for process-management-specialist

**Task 1: Spawn Echo Process**
```
Create a simple Node.js script that:
1. Spawns `echo` command with arguments
2. Captures stdout
3. Logs output
4. Handles exit code
```

**Expected Output:**
- Process spawns successfully
- Stdout captured correctly
- Exit code 0 detected

**Task 2: Parse NDJSON Stream**
```
Create a script that:
1. Spawns a process that outputs NDJSON
2. Parses each line as JSON
3. Handles malformed JSON
4. Logs parsed events
```

**Expected Output:**
- Valid JSON parsed
- Malformed JSON skipped with warning
- No crashes on invalid input

**Task 3: Crash Recovery**
```
Create a script that:
1. Spawns a process that crashes
2. Detects crash (exit code != 0)
3. Restarts with exponential backoff
4. Stops after 5 retries
```

**Expected Output:**
- Crash detected
- Backoff delays increase (1s, 2s, 4s, 8s, 16s)
- Circuit breaker stops retries

### Test Tasks for vscode-extension-specialist

**Task 1: Status Bar**
```
Create VSCode extension that:
1. Activates on startup
2. Creates status bar item
3. Updates text every second
4. Shows tooltip on hover
```

**Expected Output:**
- Extension activates <500ms
- Status bar appears
- Text updates correctly
- Tooltip shows

**Task 2: QuickPick**
```
Create command that:
1. Shows QuickPick with 3 options
2. User selects option
3. Stores selection in config
4. Shows confirmation notification
```

**Expected Output:**
- QuickPick displays
- Selection stored
- Notification shows

**Task 3: SecretStorage**
```
Create commands that:
1. Store API key in SecretStorage
2. Retrieve API key
3. Delete API key
4. Never log API key
```

**Expected Output:**
- Key stored securely
- Key retrieved correctly
- Key deleted successfully
- No keys in logs

## Agent Creation Priority

**Phase 0: Create Agents (2-3 days)**

**Day 1:**
1. Create `process-management-specialist.md`
2. Test with echo process task
3. Test with NDJSON parsing task
4. Test with crash recovery task

**Day 2:**
1. Create `vscode-extension-specialist.md`
2. Test with status bar task
3. Test with QuickPick task
4. Test with SecretStorage task

**Day 3:**
1. Test agents together on combined task
2. Refine agent definitions based on test results
3. Document agent usage patterns
4. Ready to start Phase 1

## Collaboration Patterns

**Sequential Workflow:**
1. **VSCode Extension Specialist** scaffolds extension structure
2. **Process Management Specialist** implements process orchestration
3. **VSCode Extension Specialist** adds UI (status bar, wizard)
4. **Docker Engineer** optimizes docker-compose setup
5. **Test Engineer** adds comprehensive tests

**Parallel Workflow:**
- **VSCode Extension Specialist** works on UI (status bar, wizard)
- **Process Management Specialist** works on process spawning simultaneously
- **Test Engineer** writes tests as other agents complete tickets

**Code Review Workflow:**
- All agents review each other's work for integration points
- **VSCode Extension Specialist** reviews activation performance
- **Process Management Specialist** reviews process lifecycle
- **Test Engineer** reviews test coverage

## Conclusion

**Critical Agents to Create:**
1. process-management-specialist (high priority)
2. vscode-extension-specialist (high priority)

**Existing Agents to Leverage:**
- Docker Engineer (service orchestration)
- Technical Researcher (API documentation)
- Test Engineer (comprehensive testing)

**Why Fewer Agents:**
- Simpler architecture (~300 lines vs ~3000 lines)
- Rust binary handles file/branch watching
- Focus on process spawning and parsing
- Less complex state management

**Workflow:**
- Create 2 specialized agents in Phase 0 (2-3 days)
- Test agents thoroughly before VSMAP implementation
- Use agent assignment matrix for ticket distribution
- Coordinate sequential and parallel workflows
- Ensure cross-agent code reviews

**Next:** Execution plan synthesizing all planning documents.

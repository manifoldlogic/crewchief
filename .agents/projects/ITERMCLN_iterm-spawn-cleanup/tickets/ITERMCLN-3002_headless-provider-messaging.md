# Ticket: ITERMCLN-3002: Add Messaging Support to HeadlessProvider

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- general-development
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement `sendMessage()` and `listAgents()` methods in HeadlessProvider using stdin pipe for communication. This enables `crewchief agent message` to work with headless agents.

## Background
HeadlessProvider already spawns processes with `stdio: 'pipe'`, meaning stdin is available for writing. We need to track spawned agents and implement the new TerminalProvider optional methods to support messaging.

The design uses stdin pipe (NOT file-based IPC) for simplicity, reliability, and security. This approach leverages the existing pipe infrastructure without requiring file system operations or complex IPC mechanisms.

Reference: ITERMCLN architecture.md - HeadlessProvider with stdin Messaging

## Acceptance Criteria
- [ ] HeadlessProvider tracks spawned agents in internal Map
- [ ] `sendMessage()` writes to agent's stdin pipe
- [ ] `listAgents()` returns list of tracked agents with status
- [ ] `crewchief agent message <headless-agent> "text"` works
- [ ] TypeScript compilation succeeds
- [ ] Unit tests pass

## Technical Requirements

Update `packages/cli/src/terminal/providers/headless.ts` to implement the optional TerminalProvider methods:

```typescript
interface HeadlessAgent {
  child: ChildProcess
  name: string
  type: string
}

export class HeadlessProvider implements TerminalProvider {
  readonly id = 'headless'
  private agents: Map<string, HeadlessAgent> = new Map()

  // Update runCommand to track agents
  async runCommand(paneId: string, command: string): Promise<void> {
    const child = spawn(command, {
      shell: true,  // Note: kept for compatibility
      stdio: 'pipe',
    })

    if (child.pid) {
      this.agents.set(paneId, {
        child,
        name: paneId,
        type: this.parseAgentType(paneId),
      })
    }
    // ... existing stdout/stderr handling

    // NOTE: Do NOT delete from agents map on exit - keep for listAgents()
    // The existing code has: this.processes.delete(paneId) on exit
    // We need to REMOVE that line so listAgents() can show stopped agents
  }

  // NEW: Messaging method - uses stdin pipe directly
  async sendMessage(paneId: string, message: string): Promise<boolean> {
    const agent = this.agents.get(paneId)
    if (!agent) return false

    // Check if process is still running before attempting to write
    if (agent.child.exitCode !== null) {
      return false  // Process has exited, can't send message
    }

    if (agent.child.stdin?.writable) {
      agent.child.stdin.write(message + '\n')
      return true
    }
    return false
  }

  // NEW: List agents (includes both running and stopped)
  async listAgents(): Promise<AgentInfo[]> {
    return Array.from(this.agents.entries()).map(([id, agent]) => ({
      id,
      name: agent.name,
      type: agent.type,
      status: agent.child.exitCode === null ? 'running' : 'stopped',
    }))
  }

  private parseAgentType(paneId: string): string {
    // Extract type from name__type format
    const parts = paneId.split('__')
    return parts.length > 1 ? parts[parts.length - 1] : 'unknown'
  }
}
```

**IMPORTANT**: The existing HeadlessProvider deletes agents from `this.processes` on exit:
```typescript
child.on('exit', (code) => {
  this.processes.delete(paneId)  // ← REMOVE THIS LINE
})
```
This line must be removed so that `listAgents()` can return stopped agents.

Key implementation details:
- Track all spawned agents in a Map keyed by paneId (rename `processes` → `agents`)
- Use stdin pipe for writing messages (already available via `stdio: 'pipe'`)
- Check `stdin.writable` AND `exitCode === null` before writing
- Parse agent type from the `name__type` naming convention used by agent spawning
- Keep agents in Map even after exit so `listAgents()` shows stopped agents

## Implementation Notes

**stdin Pipe Communication**:
- stdin pipe is standard, secure, and requires no file system operations
- Already configured in existing `runCommand()` implementation
- Agents can read from stdin using standard input methods
- No need for temporary files, sockets, or complex IPC

**Agent Tracking**:
- Map keyed by paneId provides O(1) lookup
- Store ChildProcess reference for stdin access
- Store name and type for `listAgents()` response
- **DO NOT** delete entries when processes exit (needed for `listAgents()` to show stopped agents)
- Clean up only happens on explicit `dispose()` call

**Status Detection**:
- `child.exitCode === null` means process is still running
- `child.exitCode !== null` means process has exited
- Check exitCode before attempting stdin writes

**Type Parsing**:
- Agents spawned with naming pattern: `{name}__{type}`
- Extract type by splitting on `__` and taking last element
- Default to 'unknown' if pattern not followed

## Dependencies
- ITERMCLN-3001 (Extended TerminalProvider interface with optional messaging methods)

## Risk Assessment

- **Risk**: stdin not writable for some agent CLIs
  - **Mitigation**: Return false and log warning if stdin is closed; gracefully handle the case where messaging is unavailable

- **Risk**: Memory leak if agents not cleaned up from Map
  - **Mitigation**: Implement cleanup on dispose(), listen for process exit events to remove from Map, track exitCode for cleanup eligibility

- **Risk**: Messages sent to exited processes
  - **Mitigation**: Check `exitCode === null` before writing, return false if process already exited

- **Risk**: Agents don't read stdin (blocking writes)
  - **Mitigation**: Node.js streams handle buffering; if buffer fills, write() will indicate backpressure via return value

## Files/Packages Affected
- `packages/cli/src/terminal/providers/headless.ts` - ADD messaging methods (`sendMessage()`, `listAgents()`), ADD agent tracking Map, UPDATE `runCommand()` to track agents

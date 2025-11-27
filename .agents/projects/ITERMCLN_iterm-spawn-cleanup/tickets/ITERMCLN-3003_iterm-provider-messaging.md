# Ticket: ITERMCLN-3003: Add Messaging Methods to ITermProvider

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
Implement `sendMessage()` and `listAgents()` methods in ITermProvider using existing Python scripts. This consolidates messaging capabilities into the provider interface, enabling agent communication directly through the provider.

## Background
Currently `agent.ts` uses `ITermSimpleService` directly for messaging operations. This ticket adds messaging methods to ITermProvider, wrapping the existing `send_to_pane.py` and `list_panes.py` scripts. This consolidation enables potential future simplification of ITermSimpleService once agent.ts is updated to use the provider interface.

Reference: ITERMCLN architecture.md - ITermProvider with messaging

## Acceptance Criteria
- [ ] `sendMessage()` method implemented, calls `send_to_pane.py` via spawnSync
- [ ] `listAgents()` method implemented, calls `list_panes.py` and parses output
- [ ] Methods filter for agent panes using name__type format convention
- [ ] Existing `agent message` and `agent list` commands still work correctly
- [ ] TypeScript compilation succeeds without errors

## Technical Requirements

Add the following methods to `packages/cli/src/terminal/providers/iterm.ts`:

### sendMessage Method
```typescript
// Matches TerminalProvider interface: sendMessage?(paneId: string, message: string): Promise<boolean>
async sendMessage(paneId: string, message: string): Promise<boolean> {
  if (!this.scriptsDir) return false

  // Extract agent type from paneId (format: name__type) for proper Enter key handling
  const agentType = this.parseAgentType(paneId)

  const args = [
    join(this.scriptsDir, 'send_to_pane.py'),
    '--to', paneId,
    '--text', message,
  ]
  if (agentType && agentType !== 'unknown') {
    args.push('--agent', agentType)
  }

  const result = spawnSync('python3', args, { encoding: 'utf-8' })
  return result.status === 0
}

private parseAgentType(paneId: string): string {
  // Extract type from name__type format
  const parts = paneId.split('__')
  return parts.length > 1 ? parts[parts.length - 1] : 'unknown'
}
```

**Note**: The `agentType` is derived from the `paneId` (which uses `name__type` format) rather than passed as a parameter. This keeps the interface simple while still providing agent-specific Enter key handling to `send_to_pane.py`.

### listAgents Method
```typescript
async listAgents(): Promise<AgentInfo[]> {
  if (!this.scriptsDir) return []

  const result = spawnSync('python3', [
    join(this.scriptsDir, 'list_panes.py'),
  ], { encoding: 'utf-8' })

  if (result.status !== 0) return []
  return this.parsePaneList(result.stdout)
}
```

### parsePaneList Helper
```typescript
private parsePaneList(output: string): AgentInfo[] {
  // Parse list_panes.py output format
  // Filter for agent panes (name__type format)
  const agents: AgentInfo[] = []
  const lines = output.trim().split('\n')
  for (const line of lines) {
    const match = line.match(/\[([^\]]+)\].*ID:(\S+)/)
    if (match) {
      const [, label, sessionId] = match
      if (label.includes('__')) {
        const parts = label.split('__')
        agents.push({
          id: sessionId,
          name: label,
          type: parts[parts.length - 1],
          status: 'running',  // iTerm panes are always running
        })
      }
    }
  }
  return agents
}
```

## Implementation Notes

- Use the same `spawnSync` pattern as other ITermProvider methods for consistency
- Parse `list_panes.py` output which uses format: `  N. [label]     Window:W Tab:T ID:session_id`
- Filter for agent panes by checking for `__` in the label (following the name__type convention)
- Agent panes should have `status: 'running'` since iTerm panes are always active
- ITermSimpleService can be deprecated in a future ticket once agent.ts is updated to use ITermProvider
- The implementation wraps existing Python scripts, maintaining compatibility with current functionality

## Dependencies
- ITERMCLN-3001 (Extended TerminalProvider interface) - Must be complete
- ITERMCLN-2001 (ITermProvider rewrite - base implementation) - Must be complete

## Risk Assessment
- **Risk**: list_panes.py output format changes unexpectedly
  - **Mitigation**: Use robust regex matching with fallback to empty array on parse failures
- **Risk**: Duplicate functionality with ITermSimpleService during transition period
  - **Mitigation**: Acceptable for now - this enables future consolidation and maintains backward compatibility

## Files/Packages Affected
- `packages/cli/src/terminal/providers/iterm.ts` - ADD `sendMessage()`, `listAgents()`, `parsePaneList()`, and `parseAgentType()` methods

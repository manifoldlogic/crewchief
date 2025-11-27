# Ticket: ITERMCLN-3001: Extend TerminalProvider Interface with Messaging Methods

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
Add optional `sendMessage()` and `listAgents()` methods to the existing `TerminalProvider` interface to support provider-specific messaging capabilities while maintaining backward compatibility.

## Background
The ITERMCLN project review decided to extend the existing `TerminalProvider` interface rather than creating a new `AgentOrchestrator` interface. This keeps the architecture simple and follows existing patterns. The new methods are optional to ensure existing provider implementations don't break.

This implements Phase 3 of the ITERMCLN architecture, setting up the foundation for agent messaging features without requiring changes to existing providers.

Reference: `.agents/projects/ITERMCLN_iterm-spawn-cleanup/planning/architecture.md` - Extended TerminalProvider Interface

## Acceptance Criteria
- [ ] `TerminalProvider` interface extended with optional `sendMessage()` method
- [ ] `TerminalProvider` interface extended with optional `listAgents()` method
- [ ] New `AgentInfo` type exported from interface module
- [ ] TypeScript compilation succeeds with no errors
- [ ] Existing provider implementations still work (no breaking changes)

## Technical Requirements

Update `packages/cli/src/terminal/interface.ts`:

```typescript
// ADD to existing TerminalProvider interface
export interface TerminalProvider {
  // ... existing methods unchanged ...

  // NEW: Optional messaging methods (Phase 3)
  sendMessage?(paneId: string, message: string): Promise<boolean>
  listAgents?(): Promise<AgentInfo[]>
}

// NEW: Agent info type
export interface AgentInfo {
  id: string           // paneId or process ID
  name: string         // Human-readable name (task__type format)
  type: string         // claude, gemini, codex, etc.
  status: 'running' | 'stopped'
}
```

**Key requirements**:
- Methods use optional chaining (`?`) for backward compatibility
- `sendMessage` returns boolean indicating success/failure
- `listAgents` returns array of AgentInfo objects
- AgentInfo structure matches expected format for `agent.ts` commands
- No changes to existing method signatures

## Implementation Notes

**Design decisions**:
- Optional methods (`?`) ensure existing implementations (like iTerm provider) don't need immediate updates
- `sendMessage` returns a Promise<boolean> for simple success/failure indication
- `listAgents` returns structured data that can be used by both CLI and orchestration logic
- AgentInfo type provides standard structure for agent metadata across all providers

**Future use**:
- Phase 4 will implement these methods in the iTerm provider
- The interface extension enables both manual agent messaging and future orchestration features
- Other terminal providers can implement these methods if they support similar features

**Testing considerations**:
- This is primarily a type definition change
- No runtime behavior changes (methods are optional)
- TypeScript compilation is the primary validation
- Existing tests should continue to pass without modification

## Dependencies
- ITERMCLN-2002 (Verify spawn works before adding features) - MUST be completed first

## Risk Assessment
- **Risk**: Breaking existing provider implementations
  - **Mitigation**: Methods are optional (`?`) - existing implementations continue to work without changes
- **Risk**: Type definition doesn't match future implementation needs
  - **Mitigation**: Design reviewed in architecture.md, matches expected usage patterns from Phase 4

## Files/Packages Affected
- `packages/cli/src/terminal/interface.ts` - ADD optional methods and AgentInfo type

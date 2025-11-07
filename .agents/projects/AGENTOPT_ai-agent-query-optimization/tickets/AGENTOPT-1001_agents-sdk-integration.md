# Ticket: AGENTOPT-1001 - Install and Configure Claude Code Agents SDK

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary

Install and configure the `@anthropic-ai/claude-agent-sdk` package in the crewchief CLI, create basic integration layer, and verify SDK can spawn agents programmatically.

## Background

The Agents SDK provides programmatic control over Claude Code agents with features like:
- Tool permission configuration
- Custom system prompts
- MCP server integration
- Hook into agent lifecycle events
- Streaming message handling

This is Phase 1 of the pivot from live user A/B testing to automated agent-based testing. See `.agents/work-in-progress/AGENTOPT-replan-analysis.md` for full context.

## Acceptance Criteria

- [ ] Install `@anthropic-ai/claude-agent-sdk` in packages/cli
- [ ] Create SDK integration module at `packages/cli/src/sdk/`
- [ ] Implement basic agent spawner with SDK
- [ ] Verify SDK can spawn agent and execute simple task
- [ ] Document SDK configuration and usage

## Technical Requirements

**Installation**:
```bash
cd packages/cli
pnpm add @anthropic-ai/claude-agent-sdk
```

**Module Structure**:
```
packages/cli/src/sdk/
├── index.ts           # Main SDK interface
├── spawner.ts         # Agent spawning logic
├── config.ts          # SDK configuration
└── types.ts           # TypeScript interfaces
```

**Core Functionality**:
```typescript
// packages/cli/src/sdk/spawner.ts
import { query } from '@anthropic-ai/claude-agent-sdk'

export interface AgentSpawnOptions {
  task: string
  worktreePath: string
  toolDescriptionOverrides?: Record<string, string>
  hooks?: {
    onToolUse?: (event: ToolUseEvent) => void
    onComplete?: (result: AgentResult) => void
  }
}

export async function spawnAgent(options: AgentSpawnOptions) {
  const result = await query({
    prompt: options.task,
    options: {
      workingDirectory: options.worktreePath,
      hooks: options.hooks,
      // TODO: Tool description injection (AGENTOPT-1002)
    }
  })

  return result
}
```

**Verification Test**:
Create a simple test that:
1. Spawns agent with SDK
2. Gives it a trivial task ("Create a file test.txt with 'Hello World'")
3. Verifies task completion
4. Captures tool use events

## Implementation Notes

**SDK Documentation**: https://docs.claude.com/en/api/agent-sdk/typescript

**Key SDK Features to Use**:
- `query()` function for agent spawning
- `hooks` option for event capture
- `permissionMode` for safety
- `systemPrompt` for custom instructions

**Integration Points**:
- Will connect to existing CompetitionManager (AGENTOPT-1003)
- Will support tool description injection (AGENTOPT-1002)
- Will feed into evaluation framework (AGENTOPT-1005)

**Notes**:
- Keep this focused on SDK basics
- Don't try to integrate with competition framework yet
- Don't implement tool description injection yet
- Just get SDK working and verified

## Dependencies

None - foundational ticket

## Risk Assessment

**Risk**: SDK has unexpected limitations
**Mitigation**: Read docs thoroughly, create minimal test first

**Risk**: SDK requires API key setup
**Mitigation**: Document setup clearly, test with valid credentials

## Files/Packages Affected

- packages/cli/package.json (add dependency)
- packages/cli/src/sdk/ (new module)
- packages/cli/tests/sdk/ (new tests)

## Planning References

- Replan Analysis: `.agents/work-in-progress/AGENTOPT-replan-analysis.md`
- SDK Docs: https://docs.claude.com/en/api/agent-sdk/typescript

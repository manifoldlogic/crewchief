# Ticket: ITERMCLN-4001: Enable Multi-Agent Spawn with Comma Syntax

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - build passes, manual testing verified in headless mode
- [x] **Verified** - by the verify-ticket agent

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
Re-enable the multi-agent spawn feature that allows users to spawn multiple agents with a single command using comma-separated agent types (e.g., `crewchief spawn claude,gemini "task"`).

## Background
The spawn command currently has multi-agent support explicitly disabled with a "not yet supported" error. The infrastructure exists - `spawn_multi_agents.py` (312 lines) in the Python scripts provides iTerm-specific implementation. This ticket enables the TypeScript CLI to parse comma-separated agents and spawn them in parallel.

This implements Phase 4 - Multi-Agent Spawn from the ITERMCLN project plan, restoring functionality that was temporarily disabled during the codebase migration.

## Acceptance Criteria
- [x] `crewchief spawn claude,gemini "task"` spawns both agents successfully
- [x] Each agent gets its own terminal pane/process
- [x] Failures for one agent don't block others (graceful degradation)
- [x] Results reported for each agent (success/failure status)
- [x] Single agent spawn still works unchanged
- [x] Works with both iTerm and headless providers

## Technical Requirements

### 1. Update spawn command parser
Modify `packages/cli/src/cli/spawn.ts` to accept comma-separated agent types:

```typescript
.action(async (agents: string, task: string | undefined, options: SpawnOptions) => {
  const agentTypes = agents.split(',').map(a => a.trim())

  if (agentTypes.length === 1) {
    // Single agent - existing logic
    await scheduler.assignSingleAgent(task, agentTypes[0])
  } else {
    // Multi-agent spawn
    console.log(chalk.cyan(`🚀 Spawning ${agentTypes.length} agents...`))

    const results = await Promise.allSettled(
      agentTypes.map(type => scheduler.assignSingleAgent(task, type))
    )

    results.forEach((result, i) => {
      if (result.status === 'fulfilled') {
        console.log(chalk.green(`✅ ${agentTypes[i]}: ${result.value}`))
      } else {
        console.log(chalk.red(`❌ ${agentTypes[i]}: ${result.reason}`))
      }
    })
  }
})
```

### 2. Remove blocking check
Remove the explicit "multi-agent not supported" error that currently prevents this feature from working.

### 3. Parallel execution
Use `Promise.allSettled` to handle partial failures gracefully - if one agent fails to spawn, others should continue.

### 4. Result reporting
Provide clear feedback for each agent spawn attempt showing success/failure status.

### 5. Optional: Agent validation
Consider validating agent types against a known list to provide early feedback on typos (optional enhancement from security review).

## Implementation Notes

### Approach
- Keep it simple - just run single-agent spawns in parallel
- Use `Promise.allSettled` instead of `Promise.all` to handle partial failures
- Each spawn is independent, minimizing race condition risks
- Consider reusing `spawn_multi_agents.py` for iTerm if it provides better pane layout control

### Testing considerations
- Test with 2-3 agents to verify parallel spawning
- Test failure scenarios (one agent fails, all agents fail)
- Verify single agent spawn remains unchanged
- Test with both iTerm and headless providers

### Edge cases
- Empty agent types after split
- Duplicate agent types in the list
- Invalid agent types mixed with valid ones

## Dependencies
- **ITERMCLN-2001** (ITermProvider rewrite) - spawn infrastructure must work first
- **ITERMCLN-2002** (Verify spawn works) - single agent spawn must be verified working

## Risk Assessment

- **Risk**: Race conditions in parallel spawn
  - **Mitigation**: Each spawn is independent, use Promise.allSettled to isolate failures

- **Risk**: iTerm pane layout issues with multiple spawns
  - **Mitigation**: Test with 2-3 agents, document any limitations in user-facing docs

- **Risk**: Unclear error messages when multiple agents fail
  - **Mitigation**: Report each agent's result individually with clear success/failure indicators

## Files/Packages Affected
- `packages/cli/src/cli/spawn.ts` - ADD multi-agent parsing and parallel spawn logic
- `packages/cli/src/orchestrator/scheduler.ts` - MAY NEED updates for parallel spawn handling
- `packages/cli/tests/cli/spawn.test.ts` - ADD tests for multi-agent spawn (if exists)

# Ticket: CLIUX-2001: Migrate spawn to agent subcommand

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary

Move the `spawn` command from the top level (`crewchief spawn`) to under the `agent` command group (`crewchief agent spawn`). This improves CLI organization by grouping all agent-related commands together.

## Background

Currently, agent operations are split between:
- `crewchief spawn <agent>` - Create and run agents (top-level)
- `crewchief agent message` - Send messages to agents
- `crewchief agent list` - List running agents
- `crewchief agent close` - Close an agent

Logically, `spawn` belongs with the other agent commands. This ticket moves it to `crewchief agent spawn` for better CLI organization.

This implements Phase 2 of the CLI UX Refinements plan.

## Acceptance Criteria

- [ ] `crewchief agent spawn <agent> [task]` works identically to current `crewchief spawn`
- [ ] All existing options preserved: `-n`, `-v`, `-a`, `--no-label`, `--backend`, `--headless`
- [ ] `crewchief spawn` no longer works (command not found)
- [ ] `crewchief agent --help` shows `spawn` subcommand
- [ ] `crewchief agent spawn --help` shows complete help with examples
- [ ] `crewchief --help` does not show `spawn` at top level
- [ ] Unit tests verify command accessibility and option preservation

## Technical Requirements

### New Command Location
```
crewchief agent spawn <agents> [task]

Arguments:
  agents              Agent type (e.g., claude, gemini)
  task                Optional task description

Options:
  -n, --name <name>   Custom agent name
  -v, --vertical      Split pane vertically
  -a, --args <args>   Additional arguments to agent command
  --no-label          Skip pane labeling
  --backend <backend> Force specific backend
  --headless          Force headless mode
```

### Files to Modify

1. **`packages/cli/src/cli/agent.ts`** - Add spawn subcommand
2. **`packages/cli/src/cli/index.ts`** - Remove `registerSpawnCommand()` call
3. **`packages/cli/src/cli/spawn.ts`** - Delete or mark as deprecated

## Implementation Notes

### SpawnOptions Type

Define the `SpawnOptions` interface inline in `agent.ts` (since we're deleting `spawn.ts`):

```typescript
interface SpawnOptions {
  name?: string
  vertical?: boolean
  args?: string
  noLabel?: boolean
  backend?: string
  headless?: boolean
}
```

### Option 1: Move Logic Inline (Recommended)

Move the spawn logic directly into `agent.ts`:

```typescript
// In agent.ts, after existing commands

agent
  .command('spawn')
  .argument('<agents>', 'Agent type(s) - e.g., claude, gemini')
  .argument('[task]', 'Optional task description')
  .option('-n, --name <name>', 'Custom name for the agent')
  .option('-v, --vertical', 'Split pane vertically')
  .option('-a, --args <args>', 'Additional arguments to agent command')
  .option('--no-label', 'Skip pane labeling')
  .option('--backend <backend>', 'Force specific backend')
  .option('--headless', 'Force headless mode')
  .description('Spawn AI agent(s) in dedicated terminal pane(s)')
  .addHelpText('after', `
Examples:
  Spawn a Claude agent:
    crewchief agent spawn claude "fix login bug"

  Spawn in vertical split:
    crewchief agent spawn claude -v "add tests"

  Spawn in headless mode:
    crewchief agent spawn gemini --headless "refactor module"
`)
  .action(async (agent: string, task: string | undefined, options: SpawnOptions) => {
    // Move logic from spawn.ts action handler
  })
```

### Option 2: Import and Delegate

If the spawn logic is complex, import the action handler:

```typescript
import { spawnAction, SpawnOptions } from './spawn'

agent
  .command('spawn')
  // ... options ...
  .action(spawnAction)
```

### Changes to `index.ts`

Remove the spawn command registration:

```typescript
// REMOVE this line:
// registerSpawnCommand(program)
```

### Handling `spawn.ts`

Choose one:
1. **Delete**: Remove the file entirely (cleaner)
2. **Keep for reference**: Add comment that it's deprecated

Recommendation: Delete the file. The code will live in `agent.ts`.

### Test File: `packages/cli/src/cli/__tests__/agent-spawn.test.ts`

**Note:** Create the test directory first if it doesn't exist:
```bash
mkdir -p packages/cli/src/cli/__tests__
```

Create test file verifying:
- Command is accessible at `agent spawn`
- All options are accepted
- Action handler is called correctly
- Top-level `spawn` is not accessible

## Dependencies

- CLIUX-1001 (Modify `worktree use`) - sequential execution
- CLIUX-1002 (Modify `worktree create`) - sequential execution

## Risk Assessment

- **Risk**: Breaking scripts using `crewchief spawn`
  - **Mitigation**: Clear error from Commander.js ("unknown command"); users can easily find `agent spawn`

- **Risk**: Regression in spawn behavior
  - **Mitigation**: Tests verify all options work; behavior should be identical

## Files/Packages Affected

- `packages/cli/src/cli/agent.ts` - Add spawn subcommand
- `packages/cli/src/cli/index.ts` - Remove spawn registration
- `packages/cli/src/cli/spawn.ts` - Delete file
- `packages/cli/src/cli/__tests__/agent-spawn.test.ts` - New test file

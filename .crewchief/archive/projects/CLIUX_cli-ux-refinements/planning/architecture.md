# Architecture: CLI UX Refinements

## Design Principles

1. **Explicit over implicit**: Commands should do one thing clearly
2. **Script-friendly defaults**: Default output should be machine-consumable
3. **Logical grouping**: Related commands should be colocated
4. **Minimal breaking changes**: Preserve functionality, change defaults

## Output Stream Requirements

**Critical for script compatibility**: Commands must separate machine-readable output from human-readable messages.

| Stream | Content | Purpose |
|--------|---------|---------|
| **stdout** | Path only (newline-terminated) | Machine consumption, `cd $(...)` |
| **stderr** | Messages via `logger.*()` | Human feedback, can be ignored |

**Why this matters**: `cd $(crewchief worktree use feature-x)` captures stdout. If messages go to stdout, the `cd` command will fail.

**Implementation pattern**:
```typescript
// Messages go to stderr (logger uses console.log → stderr when piped)
logger.success(`Created worktree at ${createdPath}`)

// Path goes to stdout
process.stdout.write(createdPath + '\n')
```

**Exit codes**:
- `0`: Success
- `1`: Error (worktree not found, creation failed, etc.)

**Note**: The existing `logger` utility already outputs to stderr when stdout is piped, so this behavior is largely automatic. The key change is ensuring we use `process.stdout.write()` for the path output, not `console.log()` or `logger.*()`.

## --print Flag Deprecation

The `--print` flag on `worktree use` will be kept indefinitely as a no-op alias for backwards compatibility. Since the default behavior is now to print the path, `--print` has no effect but doesn't harm anything. No deprecation warning will be shown.

## Solution Design

### 1. Worktree Commands Redesign

#### `worktree use <name>`

**New behavior:**
```
crewchief worktree use <name> [options]

Arguments:
  name          Worktree selector (branch name, directory name, or path)

Options:
  --shell       Start interactive subshell in worktree (default: print path)
  -p, --print   Print absolute path (explicit form of default behavior)
  --branch <b>  (REMOVED - no longer creates)
  --base-path   (REMOVED - no longer creates)

Behavior:
  1. Search for existing worktree matching <name>
  2. If found: output absolute path to stdout (or spawn shell with --shell)
  3. If not found: error with suggestion to use 'worktree create'
```

**Implementation changes in worktree.ts:**
```typescript
.command('use')
.argument('<name>')
.option('--shell', 'Start interactive subshell in worktree')
.option('-p, --print', 'Print absolute path (default behavior)')
.description('Switch to an existing worktree. Prints path by default.')
.action(async (name, opts) => {
  // Search only - no creation
  const matches = findWorktree(name)

  if (matches.length === 0) {
    logger.error(`Worktree '${name}' not found.`)
    logger.info(`Create it with: crewchief worktree create ${name}`)
    process.exitCode = 1
    return
  }

  if (matches.length > 1) {
    // Existing ambiguity handling
  }

  const targetPath = resolve(matches[0].path)

  if (opts.shell) {
    // Spawn interactive shell
    displaySubshellMessage(...)
    spawn(shell, { stdio: 'inherit', cwd: targetPath })
  } else {
    // Default: print path
    process.stdout.write(targetPath + '\n')
  }
})
```

#### `worktree create <name>`

**New behavior:**
```
crewchief worktree create <name> [options]

Arguments:
  name              Name for the new worktree branch

Options:
  --branch <base>   Base branch to create from (default: main)
  --base-path <dir> Directory for worktrees
  --shell           Start interactive subshell after creating
  --no-copy-ignored Skip copying ignored files

Behavior:
  1. Create worktree via WorktreeService
  2. Print path to stdout (or spawn shell with --shell)
```

**Implementation changes:**
```typescript
.command('create')
.argument('<name>')
.option('--branch <base>', 'Base branch')
.option('--base-path <dir>', 'Worktree storage directory')
.option('--shell', 'Start interactive subshell after creating')
.option('--no-copy-ignored', 'Skip copying ignored files')
.description('Create a new git worktree. Prints path by default.')
.action(async (name, opts) => {
  const createdPath = await wt.createWorktree(...)
  logger.success(`Created worktree at ${createdPath}`)

  if (opts.shell) {
    displaySubshellMessage(...)
    spawn(shell, { stdio: 'inherit', cwd: createdPath })
  } else {
    // Print path for scripting
    process.stdout.write(createdPath + '\n')
  }
})
```

**Note:** The existing `--no-cd` flag will be removed. The new `--shell` flag is the inverse semantic.

### 2. Agent Spawn Migration

#### Current Structure
```
crewchief
├── spawn <agents> [task]    # Top-level
└── agent
    ├── message
    ├── list
    └── close
```

#### New Structure
```
crewchief
└── agent
    ├── spawn <agents> [task]  # Moved here
    ├── message
    ├── list
    └── close
```

**Implementation approach:**

1. Move spawn logic from `spawn.ts` into `agent.ts`
2. Remove `registerSpawnCommand()` call from `index.ts`
3. Keep `spawn.ts` file for potential deprecation alias

**In agent.ts:**
```typescript
agent
  .command('spawn')
  .argument('<agents>', 'Agent type(s)')
  .argument('[task]', 'Optional task description')
  .option('-n, --name <name>', 'Custom agent name')
  .option('-v, --vertical', 'Split pane vertically')
  .option('-a, --args <args>', 'Additional arguments')
  .option('--no-label', 'Skip pane labeling')
  .option('--backend <backend>', 'Force specific backend')
  .option('--headless', 'Force headless mode')
  .description('Spawn AI agent(s) in dedicated terminal pane(s)')
  .action(async (agent, task, options) => {
    // Existing spawn logic from spawn.ts
  })
```

### 3. Help Text Updates

Each command needs updated help text reflecting:
- New default behaviors
- New flag names
- Example usage patterns

**Example for `worktree use`:**
```typescript
.addHelpText('after', `
Examples:
  Switch to worktree (prints path):
    cd $(crewchief worktree use feature-x)

  Open worktree in subshell:
    crewchief worktree use feature-x --shell

  Use in scripts:
    path=$(crewchief worktree use my-branch)
    code "$path"
`)
```

**Example for `agent spawn`:**
```typescript
.addHelpText('after', `
Examples:
  Spawn a Claude agent:
    crewchief agent spawn claude "fix login bug"

  Spawn in vertical split:
    crewchief agent spawn claude -v "add tests"

  Spawn in headless mode:
    crewchief agent spawn gemini --headless "refactor module"
`)
```

## File Changes Summary

| File | Changes |
|------|---------|
| `src/cli/worktree.ts` | Modify `use` (remove create, change default), modify `create` (change default) |
| `src/cli/agent.ts` | Add `spawn` subcommand with full implementation |
| `src/cli/spawn.ts` | Remove or keep for deprecation alias |
| `src/cli/index.ts` | Remove `registerSpawnCommand()` call |

## Command Flow Diagrams

### Worktree Use Flow (New)
```
worktree use <name>
      │
      ▼
┌─────────────────┐
│ Search worktrees│
└────────┬────────┘
         │
    ┌────┴────┐
    │ Found?  │
    └────┬────┘
     no  │  yes
    ┌────┴────┐
    │         │
    ▼         ▼
┌───────┐  ┌──────────┐
│ Error │  │ --shell? │
│ + tip │  └────┬─────┘
└───────┘   no  │  yes
           ┌────┴────┐
           │         │
           ▼         ▼
      ┌────────┐ ┌──────────┐
      │ Print  │ │ Spawn    │
      │ path   │ │ subshell │
      └────────┘ └──────────┘
```

### Worktree Create Flow (New)
```
worktree create <name>
      │
      ▼
┌─────────────────┐
│ Create worktree │
└────────┬────────┘
         │
    ┌────┴────┐
    │--shell? │
    └────┬────┘
     no  │  yes
    ┌────┴────┐
    │         │
    ▼         ▼
┌────────┐ ┌──────────┐
│ Print  │ │ Spawn    │
│ path   │ │ subshell │
└────────┘ └──────────┘
```

## Backwards Compatibility

### Migration Notes

1. **Scripts using `worktree use` with auto-create**: Will fail with clear error message suggesting `worktree create`

2. **Scripts expecting subshell**: Will get path output instead. Add `--shell` flag.

3. **Scripts using `crewchief spawn`**: Will fail. Use `crewchief agent spawn` instead.

### Deprecation Strategy (Optional)

If desired, we could add a deprecation layer:

```typescript
// In index.ts - deprecated top-level spawn
program
  .command('spawn')
  .argument('<agents>')
  .argument('[task]')
  .action(async (agents, task, opts) => {
    logger.warn('`crewchief spawn` is deprecated. Use `crewchief agent spawn` instead.')
    // Forward to agent spawn
  })
```

**Recommendation**: Skip deprecation alias. The change is straightforward and clean breaks are better than lingering deprecations.

## Technology Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Default output format | Plain path, newline-terminated | Maximum scriptability, works with `$()` |
| Shell spawning | Opt-in via `--shell` | Non-invasive default |
| Spawn location | Under `agent` subcommand | Logical grouping |
| Deprecation alias | Not implemented | Clean break preferred |

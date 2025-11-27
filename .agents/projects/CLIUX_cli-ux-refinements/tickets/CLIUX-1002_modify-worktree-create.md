# Ticket: CLIUX-1002: Modify `worktree create` command

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

Modify the `worktree create` command to change the default output from spawning a subshell to printing the worktree path to stdout. Add `--shell` flag for opt-in subshell behavior.

## Background

The current `worktree create` command spawns an interactive subshell by default after creating a worktree. This is invasive for scripting - users must type `exit` to return, and scripts can't easily capture the created path.

This ticket implements Phase 1, Part 2 of the CLI UX Refinements plan. The change makes the command script-friendly by default while preserving interactive use via opt-in flag.

**Design Reference**: See `planning/architecture.md` for the "Output Stream Requirements" specification.

## Acceptance Criteria

- [ ] `worktree create <name>` prints the absolute path to **stdout** (newline-terminated) after creation
- [ ] Success message goes to **stderr** via `logger.success()` (won't break `cd $(...)`)
- [ ] `worktree create <name> --shell` spawns interactive subshell (opt-in behavior)
- [ ] `--no-cd` flag is removed (replaced by new default behavior)
- [ ] Existing options preserved: `--branch`, `--base-path`, `--no-copy-ignored`
- [ ] Help text updated with examples showing `cd $(crewchief worktree create ...)`
- [ ] Unit tests pass for all new behaviors

## Technical Requirements

### Output Streams (Critical)
```typescript
// Success message goes to stderr
logger.success(`Created worktree at ${createdPath}`)

// Path goes to stdout for machine consumption
process.stdout.write(createdPath + '\n')
```

### Exit Codes
- `0`: Success (worktree created)
- `1`: Error (creation failed)

### Command Signature
```
crewchief worktree create <name> [options]

Options:
  --branch <base>     Base branch to create from (default: main)
  --base-path <dir>   Directory for worktrees
  --shell             Start interactive subshell after creating
  --no-copy-ignored   Skip copying ignored files
```

## Implementation Notes

### Changes to `packages/cli/src/cli/worktree.ts`

1. **Change default behavior** (in `create` command action):
   - Remove subshell spawn from default path
   - Add `process.stdout.write(createdPath + '\n')` after creation

2. **Add `--shell` flag**:
   - New option: `.option('--shell', 'Start interactive subshell after creating')`
   - When set, use existing subshell logic (`displaySubshellMessage()` + `spawn()`)

3. **Remove `--no-cd` flag**:
   - Delete the option definition
   - The `opts.cd !== false` check is no longer needed

4. **Update help text**:
   ```typescript
   .addHelpText('after', `
   Examples:
     Create and switch to worktree:
       cd $(crewchief worktree create feature-x)

     Create and open in subshell:
       crewchief worktree create feature-x --shell

     Create from specific branch:
       cd $(crewchief worktree create hotfix --branch release-1.0)
   `)
   ```

### Current Code Pattern (to be modified)
```typescript
// Current default behavior (to be changed)
const shouldCd = opts.cd !== false
if (shouldCd) {
  displaySubshellMessage(...)
  spawn(shell, { stdio: 'inherit', cwd: createdPath })
}
```

### New Code Pattern
```typescript
// New default behavior
if (opts.shell) {
  displaySubshellMessage(...)
  spawn(shell, { stdio: 'inherit', cwd: createdPath })
} else {
  // Default: print path for scripting
  process.stdout.write(createdPath + '\n')
}
```

### Test File: `packages/cli/src/cli/__tests__/worktree-create.test.ts`

Create new test file with:
- Mock `WorktreeService.createWorktree()`
- Mock `child_process.spawn`
- Test stdout output isolation
- Test that options are passed to WorktreeService
- Test `--shell` flag spawns subshell

## Dependencies

- CLIUX-1001 (Modify `worktree use` command) - for consistent behavior across worktree commands

## Risk Assessment

- **Risk**: Scripts relying on subshell behavior will get path output instead
  - **Mitigation**: Add `--shell` flag; document in help text

- **Risk**: Stdout/stderr separation incorrect
  - **Mitigation**: Explicit test; use `process.stdout.write()` for path, `logger.*()` for messages

## Files/Packages Affected

- `packages/cli/src/cli/worktree.ts` - Main implementation changes
- `packages/cli/src/cli/__tests__/worktree-create.test.ts` - New test file

# Ticket: CLIUX-1001: Modify `worktree use` command

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary

Modify the `worktree use` command to only work with existing worktrees (remove auto-create behavior) and change the default output from spawning a subshell to printing the worktree path to stdout.

## Background

The current `worktree use` command has two behaviors that violate the principle of least surprise:

1. **Auto-create**: If the worktree doesn't exist, it creates one. Users expect "use" to only work with existing worktrees.
2. **Subshell default**: The command spawns an interactive subshell by default, which is invasive for scripting.

This ticket implements Phase 1, Part 1 of the CLI UX Refinements plan. The goal is to make commands explicit and script-friendly.

**Design Reference**: See `planning/architecture.md` for the full "Output Stream Requirements" specification.

## Acceptance Criteria

- [ ] `worktree use <name>` prints the absolute path to **stdout** (newline-terminated) when worktree exists
- [ ] `worktree use <name>` exits with code 1 and shows error message with suggestion when worktree not found
- [ ] `worktree use <name> --shell` spawns interactive subshell (opt-in behavior)
- [ ] `--print` flag is accepted as no-op alias (backwards compatibility)
- [ ] `--branch` and `--base-path` options are removed (no longer applicable)
- [ ] Help text updated with examples showing `cd $(crewchief worktree use ...)`
- [ ] Unit tests pass for all new behaviors

## Technical Requirements

### Output Streams (Critical)
```typescript
// Path goes to stdout for machine consumption
process.stdout.write(targetPath + '\n')

// Messages go to stderr via logger (won't break cd $(...))
logger.error(`Worktree '${name}' not found.`)
logger.info(`Create it with: crewchief worktree create ${name}`)
```

### Exit Codes
- `0`: Success (worktree found)
- `1`: Error (worktree not found, ambiguous selector, etc.)

### Command Signature
```
crewchief worktree use <name> [options]

Options:
  --shell       Start interactive subshell in worktree
  -p, --print   Print absolute path (default behavior, kept for compatibility)
```

### Error Message Format
```
[err] Worktree 'feature-x' not found.
[info] Create it with: crewchief worktree create feature-x
```

## Implementation Notes

### Changes to `packages/cli/src/cli/worktree.ts`

1. **Remove auto-create logic** (lines ~205-215 in current code):
   - Delete the block that creates a worktree when `matches.length === 0`
   - Replace with error message and exit code 1

2. **Change default behavior**:
   - Remove subshell spawn from default path
   - Add `process.stdout.write(targetPath + '\n')` as default

3. **Add `--shell` flag**:
   - New option: `.option('--shell', 'Start interactive subshell in worktree')`
   - When set, use existing subshell logic (`displaySubshellMessage()` + `spawn()`)

4. **Remove options**:
   - Delete `--branch` option (no longer creates)
   - Delete `--base-path` option (no longer creates)
   - Delete `--no-copy-ignored` option (no longer creates)

5. **Keep `--print` as alias**:
   - Keep the option definition but make it a no-op
   - No deprecation warning needed

6. **Update help text**:
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

### Test File: `packages/cli/src/cli/__tests__/worktree-use.test.ts`

Create new test file with:
- Mock `WorktreeService.listWorktrees()`
- Mock `child_process.spawn`
- Test stdout output isolation
- Test exit codes
- Test error messages

## Dependencies

- None (first ticket in sequence)

## Risk Assessment

- **Risk**: Breaking scripts that rely on auto-create behavior
  - **Mitigation**: Error message includes suggestion to use `worktree create`

- **Risk**: Stdout/stderr separation incorrect, breaking `cd $(...)`
  - **Mitigation**: Explicit test for stdout isolation; use `process.stdout.write()` not `console.log()`

## Files/Packages Affected

- `packages/cli/src/cli/worktree.ts` - Main implementation changes
- `packages/cli/src/cli/__tests__/worktree-use.test.ts` - New test file

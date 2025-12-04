# Bash Tool Use Hook

This hook blocks dangerous git commands that could permanently delete uncommitted work.

## Blocked Commands

The following git commands are **BLOCKED** and will return an error:

- `git reset --hard` - Permanently deletes uncommitted changes
- `git clean -fd` - Permanently deletes untracked files and directories
- `git clean -fdx` - Permanently deletes untracked and ignored files

## Why These Are Blocked

These commands permanently delete work without any recovery mechanism (not in reflog, not in stash). In a workflow where agents implement code, test it, and then commit it, losing uncommitted work means losing hours of implementation effort.

## Safe Alternatives

Instead of `git reset --hard origin/main`:
```bash
# Save work first, then sync
git stash push -m "WIP: saving before sync"
git fetch origin
git rebase origin/main
git stash pop
```

Instead of `git clean -fd`:
```bash
# Review what would be deleted first
git clean -fdn
# Then selectively delete if needed
rm -rf specific/directory
```

## Override

If you absolutely must use these commands, the user can run them directly in their terminal. Agents should never use these commands autonomously.

## Triggering Condition

This hook triggers on any Bash tool use. It checks the command string for the blocked patterns and returns an error if found.

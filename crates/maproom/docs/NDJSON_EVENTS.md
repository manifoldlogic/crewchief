# NDJSON Events

The `maproom watch` command emits NDJSON (Newline Delimited JSON) events to stdout. These events allow external tools (like the VSCode extension) to monitor indexing progress and respond to repository state changes.

## Event Format

All events are single-line JSON objects written to stdout, one per line. Each event has a `type` field identifying the event kind.

## Event Types

### `branch_switched`

Emitted when the watch command detects a git branch switch.

**When emitted:**
- After `.git/HEAD` file changes
- After successful branch detection
- After worktree record lookup/creation in database
- Before starting file watch on the new branch

**Format:**
```json
{"type":"branch_switched","timestamp":"2025-01-17T02:30:00Z","repo":"crewchief","old_branch":"main","new_branch":"feature-auth","old_worktree_id":1,"new_worktree_id":42,"worktree_created":false}
```

**Pretty-printed for readability:**
```json
{
  "type": "branch_switched",
  "timestamp": "2025-01-17T02:30:00Z",
  "repo": "crewchief",
  "old_branch": "main",
  "new_branch": "feature-auth",
  "old_worktree_id": 1,
  "new_worktree_id": 42,
  "worktree_created": false
}
```

**Fields:**

| Field | Type | Description |
|-------|------|-------------|
| `type` | string | Always `"branch_switched"` |
| `timestamp` | string | ISO 8601 timestamp (UTC) when the switch was detected |
| `repo` | string | Repository name |
| `old_branch` | string | Previous branch name |
| `new_branch` | string | Current branch name |
| `old_worktree_id` | integer | Database ID of the previous worktree record |
| `new_worktree_id` | integer | Database ID of the current worktree record |
| `worktree_created` | boolean | `true` if the worktree record was newly created, `false` if it already existed |

**Use cases:**
- VSCode extension updates status bar to show current branch
- VSCode extension refreshes file decorations for new worktree context
- External tools track branch switches for analytics
- Automation scripts trigger actions on branch changes

**Example consumer (TypeScript):**
```typescript
import { spawn } from 'child_process';
import * as readline from 'readline';

const watcher = spawn('maproom', ['watch']);
const rl = readline.createInterface({ input: watcher.stdout });

rl.on('line', (line) => {
  try {
    const event = JSON.parse(line);

    if (event.type === 'branch_switched') {
      console.log(`Branch changed: ${event.old_branch} → ${event.new_branch}`);
      console.log(`Worktree ID: ${event.old_worktree_id} → ${event.new_worktree_id}`);

      if (event.worktree_created) {
        console.log('New worktree record created in database');
      }

      // Update UI, refresh context, etc.
    }
  } catch (e) {
    // Not a JSON line (probably log output to stderr)
  }
});
```

## Consumption Guidelines

**Best practices:**
1. Parse each line as JSON individually
2. Check the `type` field to dispatch to appropriate handlers
3. Handle parse errors gracefully (non-JSON lines may appear if logging is verbose)
4. Use stderr for logs, stdout for NDJSON events (maproom follows this convention)
5. Events are emitted immediately without buffering

**Error handling:**
```typescript
try {
  const event = JSON.parse(line);
  handleEvent(event);
} catch (e) {
  // Ignore non-JSON lines (could be debug output to stdout by accident)
  // Consider logging to help diagnose issues:
  console.error('Failed to parse NDJSON:', line);
}
```

## Future Events

Additional event types may be added in future versions:

- `scan_started`: Emitted when a full scan begins
- `scan_completed`: Emitted when a scan finishes with stats
- `file_indexed`: Emitted when an individual file is processed
- `error`: Emitted when an error occurs during watching

**Forward compatibility:**
Consumers should ignore unknown event types gracefully:

```typescript
if (event.type === 'branch_switched') {
  // Handle known event
} else {
  // Ignore unknown events (forward compatibility)
  console.debug('Unknown event type:', event.type);
}
```

## Testing

To test NDJSON event emission:

```bash
# Watch in one terminal
maproom watch 2>/dev/null | jq

# In another terminal, switch branches
cd /your/repo
git checkout feature-branch

# Verify branch_switched event appears in first terminal
```

## See Also

- `crates/maproom/CLAUDE.md` - Development documentation
- `crates/maproom/src/indexer/mod.rs` - Event emission implementation (search for `BranchSwitchEvent`)

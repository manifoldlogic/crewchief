# Ticket: VSMAP-3001: Enhance StdoutParser with detailed event extraction

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (237 tests pass)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- process-management-specialist
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Expand NDJSON parser to extract all event types from Rust binary. Update status bar with file counts, timestamps, and error details.

## Background
This implements Phase 3 (Process Monitoring) of the VSMAP plan. The basic StdoutParser from VSMAP-1004 handles simple event parsing, but we need to extract additional metadata for rich status bar updates and detailed error reporting. This ticket enhances the parser to extract file counts, elapsed time, error contexts, and other metadata that improves the user experience.

Reference: VSMAP_PLAN.md Phase 3 "Process Monitoring - Enhanced Event Parsing"

## Acceptance Criteria
- [x] All NDJSON event types parsed: progress, error, complete, status, file_processed
- [x] Status bar shows file counts during indexing (e.g., "Indexing: 1,234 files")
- [x] Status bar shows relative timestamps (e.g., "Last indexed: 2 minutes ago")
- [x] Error events displayed with file path if available
- [x] Malformed JSON logged as warning but doesn't crash parser
- [x] Parser emits detailed events to subscribers

## Technical Requirements
- Parse additional event fields:
  - `files: number` - total files processed
  - `elapsed: number` - milliseconds elapsed
  - `file_path?: string` - current file being processed (for errors)
  - `error_type?: string` - classification of error
- Store last indexed timestamp in workspace state
- Format timestamps: "2 minutes ago", "just now", "1 hour ago" (relative time)
- Emit detailed events to subscribers (StatusBar, error handler)
- Handle malformed JSON gracefully with try/catch, log warning
- Add TypeScript types for all event shapes

## Implementation Notes
Expand the event type definitions:

```typescript
type ProgressEvent = {
  type: 'progress';
  percent: number;
  files: number;
  elapsed: number;
  current_file?: string;
};

type ErrorEvent = {
  type: 'error';
  message: string;
  file_path?: string;
  error_type?: 'parse' | 'io' | 'embedding' | 'database';
};

type CompleteEvent = {
  type: 'complete';
  files: number;
  elapsed: number;
  timestamp: string;
};

type StatusEvent = {
  type: 'status';
  state: 'idle' | 'scanning' | 'watching';
  files?: number;
};
```

Relative time formatting:
```typescript
function formatRelativeTime(timestamp: Date): string {
  const seconds = Math.floor((Date.now() - timestamp.getTime()) / 1000);
  if (seconds < 60) return 'just now';
  if (seconds < 3600) return `${Math.floor(seconds / 60)} minutes ago`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)} hours ago`;
  return `${Math.floor(seconds / 86400)} days ago`;
}
```

Status bar integration:
- On progress events: Update to show file count
- On complete events: Store timestamp, show "Last indexed: X ago"
- On error events: Show error icon, tooltip with file path
- On status events: Update state indicator

Malformed JSON handling:
```typescript
try {
  const event = JSON.parse(line);
  // process event
} catch (err) {
  output.appendLine(`[WARN] Malformed JSON: ${line}`);
  // continue processing
}
```

## Dependencies
- VSMAP-1004 (basic stdout parser) as foundation
- VSMAP-1005 (status bar manager) for status updates
- VSMAP-2003 (initial scan) will use enhanced events

## Risk Assessment
- **Risk**: Rust binary may emit events faster than parser can process
  - **Mitigation**: Use buffered line reading, process events asynchronously
- **Risk**: Malformed JSON could indicate binary crash or corruption
  - **Mitigation**: Log warnings, track frequency, escalate if >10% malformed
- **Risk**: Timestamp calculations may be incorrect across timezones
  - **Mitigation**: Use UTC timestamps, convert to local for display

## Files/Packages Affected
- `src/process/parser.ts` (enhance existing file, +80 lines)
- `src/types/events.ts` (new file for event type definitions)
- `src/process/statusBar.ts` (update to use detailed events)
- `src/test/parser.test.ts` (expand tests for new event types)

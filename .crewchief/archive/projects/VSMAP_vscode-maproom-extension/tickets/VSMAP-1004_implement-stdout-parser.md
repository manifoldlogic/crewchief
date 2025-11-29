  # Ticket: VSMAP-1004: Implement NDJSON stdout parser for process output

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - tests executed and passing (or N/A if no tests)
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
Create `StdoutParser` to parse NDJSON (newline-delimited JSON) events from Rust binary stdout and emit structured events for the UI layer.

## Background
The Rust binary outputs NDJSON events with progress info, errors, and status. We need to parse these into TypeScript events for the status bar and error handling. This provides the data layer between the spawned processes and the UI.

This ticket completes **Milestone 1.2: Binary Spawner** from Phase 1 of the VSMAP project plan by implementing the communication layer between Rust processes and TypeScript UI.

## Acceptance Criteria
- [x] Parse NDJSON lines from stdout stream
- [x] Extract event types: `progress`, `error`, `complete`, `status`
- [x] Emit TypeScript events via EventEmitter
- [x] Handle malformed JSON gracefully (log warning, continue)
- [x] Buffer incomplete lines (handle partial writes)
- [x] Line-by-line parsing (no buffering entire stdout)

## Technical Requirements
- Use Node.js `readline.createInterface()` for line-by-line parsing
- Parse each line as JSON: `JSON.parse(line)`
- Event types (expected from Rust binary):
  ```typescript
  type WatchEvent =
    | { type: 'progress', files: number, complete: number }
    | { type: 'error', message: string, file?: string }
    | { type: 'complete', files: number, duration: number }
    | { type: 'status', state: 'watching' | 'indexing' | 'idle' }
  ```
- Extend Node.js EventEmitter for pub/sub pattern
- Handle parse errors (invalid JSON, missing fields)
- Validate event structure (type field exists, expected shape)

## Implementation Notes
- Reference Rust binary's NDJSON output format (document expected schema in events.ts)
- Use try/catch for JSON.parse() robustness
- Log malformed events to Output panel (debugging aid)
- Don't crash on bad input (defensive programming)
- Consider schema validation library (e.g., zod) for runtime type checking
- Unit tests should cover: valid events, malformed JSON, partial lines, missing fields
- Store last valid event for debugging (helps diagnose parser issues)

## Dependencies
- VSMAP-1003 (ProcessOrchestrator provides stdout stream)

## Risk Assessment
- **Risk**: Rust binary output format may change
  - **Mitigation**: Version the output format, detect mismatches, fail gracefully
- **Risk**: High-frequency events may overwhelm parser
  - **Mitigation**: readline handles backpressure, but add rate limiting if needed
- **Risk**: Partial JSON across buffer boundaries
  - **Mitigation**: readline handles this, but test edge cases

## Files/Packages Affected
- `src/process/parser.ts` (create, ~150 lines)
- `src/process/events.ts` (create, ~50 lines - type definitions)
- `src/process/parser.test.ts` (create, ~100 lines - unit tests)
- `src/process/orchestrator.ts` (modify to use parser)

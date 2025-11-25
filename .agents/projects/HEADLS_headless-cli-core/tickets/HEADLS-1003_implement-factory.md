# Ticket: Implement TerminalFactory with Auto-Detection

**ID:** HEADLS-1003
**Phase:** 1
**Status:** Pending
**Assigned To:** TypeScript Engineer

## Summary
Create a `TerminalFactory` that detects the running environment and returns the appropriate `TerminalProvider`. It must support a `--headless` override flag.

## Background
The CLI needs to know which provider to use. It should default to `Headless` in CI/Linux, `ITerm` in iTerm.app, but allow users to force `Headless` even in iTerm.

## Acceptance Criteria
- [ ] `TerminalFactory` implemented in `packages/cli/src/terminal/factory.ts`.
- [ ] `autoDetect()` method logic:
  1. If `process.argv` includes `--headless`, return `HeadlessProvider`.
  2. If `TERM_PROGRAM === 'iTerm.app'`, return `ITermProvider`.
  3. Else, return `HeadlessProvider`.
- [ ] Factory can also accept an explicit provider ID for testing.
- [ ] Unit tests cover all detection cases (mocking `process.env` and `process.argv`).

## Technical Requirements
- **File Path**: `packages/cli/src/terminal/factory.ts`
- **Dependencies**: Import providers from `./providers/`.
- **Note**: Since `ITermProvider` and `HeadlessProvider` are not fully implemented yet (in this phase), you can stub them or just return the `MockProvider` temporarily if needed, or create empty shells for them in this ticket. (Prefer empty shells).

## Implementation Notes
- Use `commander` or manual parsing for the `--headless` flag check if `program` object isn't available at this stage of boot. Manual parsing of `process.argv` is safer for early boot logic.

## Dependencies
- HEADLS-1001
- HEADLS-1002

## Risks
- Circular dependencies if Factory imports fully implemented providers that import other things. Keep imports clean.


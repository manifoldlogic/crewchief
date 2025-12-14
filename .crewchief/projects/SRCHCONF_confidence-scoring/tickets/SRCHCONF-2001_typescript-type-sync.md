# Ticket: [SRCHCONF-2001]: TypeScript Type Sync and Validation Tests

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- typescript-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create TypeScript ConfidenceSignals interface matching the Rust struct field-for-field, add TYPE_SYNC comments, and implement type validation tests to ensure Rust-TypeScript boundary remains synchronized.

## Background
Phase 2 begins with synchronizing types across the Rust-TypeScript boundary. The daemon (Rust) will serialize ConfidenceSignals to JSON, and the TypeScript client must deserialize it correctly. Type mismatches break MCP consumers.

This follows the proven pattern from QueryUnderstanding (SRCHTRN project) using TYPE_SYNC comments and validation tests.

## Acceptance Criteria
- [ ] `ConfidenceSignals` interface created in `packages/daemon-client/src/types.ts` with 3 fields matching Rust
- [ ] TYPE_SYNC comment added: `/** Sync with: crates/maproom/src/search/results.rs::ConfidenceSignals */`
- [ ] All 3 fields match Rust struct exactly: source_count (number), score_gap (number), is_exact_match (boolean)
- [ ] Type validation tests added to `packages/daemon-client/src/types.test.ts`
- [ ] Serialization roundtrip test passes (Rust JSON → TypeScript)
- [ ] Optional field handling test passes (confidence present/absent)
- [ ] All TypeScript tests pass (`pnpm test packages/daemon-client`)
- [ ] Zero TypeScript compilation errors (`pnpm typecheck`)

## Technical Requirements
**TypeScript Interface**:
```typescript
/**
 * Confidence signals for assessing search result quality.
 *
 * Sync with: crates/maproom/src/search/results.rs::ConfidenceSignals
 */
export interface ConfidenceSignals {
  /** Number of search sources that returned this chunk (1-4) */
  source_count: number
  /** Score difference between this result and next result */
  score_gap: number
  /** Whether query exactly matched symbol name */
  is_exact_match: boolean
}
```

**Type Validation Tests** (minimum 3 tests):
1. ConfidenceSignals deserializes correctly from Rust JSON
2. All 3 fields have correct types and values
3. Optional confidence field handling (present vs absent)

**Test Example**:
```typescript
it('should deserialize ConfidenceSignals from Rust JSON', () => {
  const rustJson = {
    source_count: 3,
    score_gap: 1.25,
    is_exact_match: true
  };

  const signals: ConfidenceSignals = rustJson;

  expect(signals.source_count).toBe(3);
  expect(signals.score_gap).toBeCloseTo(1.25);
  expect(signals.is_exact_match).toBe(true);
});
```

## Implementation Notes
Follow the TYPE_SYNC pattern established in the codebase:
- Rust has authoritative comment: `/// TYPE_SYNC: packages/daemon-client/src/types.ts::ConfidenceSignals`
- TypeScript has mirror comment: `/** Sync with: crates/maproom/src/search/results.rs::ConfidenceSignals */`
- Comments make manual sync process explicit
- Validation tests catch discrepancies automatically

Type sync strategy from architecture.md:
- Manual synchronization with TYPE_SYNC comments (proven pattern)
- Validation tests run on every commit (CI enforcement)
- Clear documentation of sync requirements

Testing philosophy from quality-strategy.md:
- Test for confidence, not coverage
- Focus on Rust-TypeScript boundary correctness
- Validate optional field handling (backward compatibility)

## Dependencies
- **Prerequisite**: SRCHCONF-1001 (Rust ConfidenceSignals struct must exist)
- **Prerequisite**: Phase 1 complete (Rust types stable before TypeScript sync)

## Risk Assessment
- **Risk**: Type sync breaks if Rust struct changes without updating TypeScript
  - **Mitigation**: TYPE_SYNC comments make relationship explicit. Validation tests fail if mismatch. CI enforces test passing.
- **Risk**: Optional field handling incorrect (confidence: undefined vs null vs missing)
  - **Mitigation**: Specific test for optional field behavior. Follow ChunkSearchResult pattern.

## Files/Packages Affected
- `packages/daemon-client/src/types.ts` - Add ConfidenceSignals interface
- `packages/daemon-client/src/types.test.ts` - Add validation tests
- `packages/daemon-client/package.json` - Verify jest configuration

## Verification Notes
The verify-ticket agent should check:
- ConfidenceSignals interface has exactly 3 fields with correct TypeScript types
- TYPE_SYNC comment present and correctly formatted in both Rust and TypeScript
- Type validation tests exist and pass
- Test output shows all 3+ tests passing
- No TypeScript compilation errors
- Interface matches Rust struct field-for-field (check naming: snake_case in both)

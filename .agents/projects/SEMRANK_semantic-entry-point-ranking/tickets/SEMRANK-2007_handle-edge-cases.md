# Ticket: SEMRANK-2007: Handle Edge Cases (Null, Unknown, Empty)

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
- database-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Ensure null symbol_name returns exact_mult=1.0, unknown kind returns kind_mult=1.0, empty query handled gracefully with error, multi-word queries work correctly.

## Background
Real codebases contain edge cases that must be handled gracefully:
- Documentation/comments may have NULL symbol_name
- Unknown kinds may exist from parsing edge cases or future tree-sitter updates
- Users may submit empty queries
- Multi-word queries need proper normalization
- Special characters in queries must be safe (no SQL injection, no crashes)

This ticket validates and tests all edge case handling, ensuring the semantic ranking system degrades gracefully rather than crashing. Most protections are already in place from SEMRANK-2003/2004; this ticket verifies and documents them.

References SEMRANK plan Section 3.4 (Edge Case Handling).

## Acceptance Criteria
- [ ] Null symbol_name handled: exact_mult = 1.0 (no boost, no crash)
- [ ] Unknown kind handled: kind_mult = 1.0 via ELSE clause (neutral baseline)
- [ ] NULL kind handled: kind_mult = 1.0 via explicit WHEN c.kind IS NULL
- [ ] Empty query handled: Returns error message "Query cannot be empty"
- [ ] Multi-word queries work: "HTTP handler" normalized correctly to "http_handler"
- [ ] Special characters in query: Handled safely (no SQL injection, no crash)
- [ ] All edge cases tested with integration tests

## Technical Requirements
- Null symbol_name: Already handled by CASE ELSE 1.0 in SEMRANK-2004a
- Unknown/NULL kind: Already handled by CASE ELSE 1.0 in SEMRANK-2003
- Empty query validation: Add in TypeScript search.ts:
  ```typescript
  if (!params.query || params.query.trim() === '') {
    throw new Error('Query cannot be empty');
  }
  ```
- Multi-word normalization: "HTTP handler" → "http_handler" (already handled by normalizeForExactMatch from SEMRANK-2004b)
- Special chars: Already safe (parameterized queries prevent SQL injection)

## Implementation Notes
**Step 1: Add Empty Query Validation**
- Add validation in TypeScript search tool
- Check before normalization and SQL execution
- Return helpful error message

**Step 2: Create Edge Case Tests**
Test cases to validate:
1. **Null symbol_name**: Query chunks with null symbol_name (docs/markdown)
   - Expected: exact_mult = 1.0, no crash
2. **Unknown kind**: Query chunks with kind not in CASE statement (if any exist)
   - Expected: kind_mult = 1.0 via ELSE clause
3. **NULL kind**: Query chunks with kind IS NULL
   - Expected: kind_mult = 1.0 via explicit NULL handler
4. **Empty query**: Submit ""
   - Expected: Error "Query cannot be empty"
5. **Multi-word**: "HTTP handler", "database connection"
   - Expected: Normalized to "http_handler", "database_connection"
6. **Special chars**: "!@#$%", "'; DROP TABLE;"
   - Expected: No crash, safe handling (parameterized queries)

**Step 3: Document Behavior**
- Add code comments documenting edge case handling
- Document in implementation notes which protections exist
- Note that parameterized queries prevent SQL injection

**Validation**:
- [ ] All test cases pass
- [ ] No crashes or SQL errors
- [ ] Helpful error messages for invalid input
- [ ] Graceful degradation for missing data

## Dependencies
- SEMRANK-2003 (kind_mult with ELSE/NULL handling)
- SEMRANK-2004a (exact_mult with ELSE handling)
- SEMRANK-2004b (normalization function)

## Risk Assessment
- **Risk**: Undiscovered edge cases in production
  - **Mitigation**: Add tests as discovered, document behavior
- **Risk**: Error messages unclear to users
  - **Mitigation**: Provide helpful error text explaining issue
- **Risk**: Special characters causing unexpected behavior
  - **Mitigation**: Parameterized queries provide protection, test common cases

## Files/Packages Affected
- `/packages/maproom-mcp/src/tools/search.ts` (empty query validation)
- Integration tests validating all edge cases

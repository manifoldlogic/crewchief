# Ticket: SEMRANK-2004b: Implement Query Normalization (TypeScript)

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
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create normalizeForExactMatch() function with enhanced acronym handling, pass normalized query to SQL, remove old exact bonus logic from Rust.

## Background
Exact match detection requires query normalization to handle naming convention variations. A user searching "validateProvider" should match a function named "validate_provider". Standard camelCase → snake_case conversion is not sufficient for acronyms like "XMLParser" (should become "xml_parser", not "x_m_l_parser") or "HTTPSHandler" (should become "https_handler").

This ticket implements the TypeScript side of exact matching: normalization algorithm and integration. The companion ticket SEMRANK-2004a implements the SQL CASE statement. This ticket also removes the old +0.2 additive bonus logic to avoid conflicting scoring mechanisms.

References SEMRANK plan Section 3.2.2 (Exact Match Multiplier Implementation).

## Acceptance Criteria
- [ ] normalizeForExactMatch() function created in TypeScript with acronym handling
- [ ] Handles all test cases correctly:
  - camelCase → snake_case: "validateProvider" → "validate_provider"
  - Acronyms at start: "XMLParser" → "xml_parser"
  - Acronyms in middle: "validateHTTPRequest" → "validate_http_request"
  - Consecutive capitals: "HTTPSHandler" → "https_handler"
  - Numbers with capitals: "Base64Encoder" → "base64_encoder"
  - kebab-case → snake_case: "validate-provider" → "validate_provider"
- [ ] Normalized query passed to SQL as $normalized_query parameter
- [ ] Old exact bonus logic removed from Rust fts.rs (lines containing ILIKE substring logic)
- [ ] Verified no conflicting bonus logic remains (grep for ILIKE to confirm)
- [ ] Unit tests created for normalization function with all edge cases

## Technical Requirements
- Location: `/packages/maproom-mcp/src/tools/search.ts`
- Normalization algorithm implementation:
  ```typescript
  function normalizeForExactMatch(query: string): string {
    let normalized = query;

    // Handle consecutive uppercase (acronyms)
    normalized = normalized.replace(/([A-Z]+)([A-Z][a-z])/g, '$1_$2'); // XMLParser → XML_Parser
    normalized = normalized.replace(/([A-Z]{2,})/g, (match) => match.toLowerCase() + '_'); // HTTP → http_

    // Handle camelCase → snake_case
    normalized = normalized.replace(/([a-z])([A-Z])/g, '$1_$2');

    // Handle kebab-case and spaces → snake_case
    normalized = normalized.replace(/[\s\-\.]/g, '_');

    // Lowercase everything
    normalized = normalized.toLowerCase();

    // Clean up multiple/trailing underscores
    normalized = normalized.replace(/_+/g, '_').replace(/^_|_$/g, '');

    return normalized;
  }
  ```
- Pass to SQL: Update query invocation with `values: [ftsQuery, normalizedQuery, repoFilter, limit]`
- Remove old bonus: Delete lines in fts.rs that add +0.2 for ILIKE match
- Unit test location: Create `/packages/maproom-mcp/tests/unit/normalize.test.ts`

## Implementation Notes
**Step 1: Create Unit Tests**
- Test cases: XMLParser, HTTPSHandler, validateHTTPRequest, Base64Encoder, validate-provider
- Verify each transformation produces expected output
- Add edge cases discovered during implementation

**Step 2: Implement Normalization Function**
- Follow algorithm specification exactly
- Test regex patterns with unit tests
- Handle edge cases (empty string, single character, etc.)

**Step 3: Integrate with Search Tool**
- Update TypeScript search tool to call normalize before SQL
- Pass normalized query as parameter
- Ensure parameter binding matches SQL query expectations

**Step 4: Remove Old Bonus Logic**
- Grep for ILIKE in fts.rs to find old bonus logic
- Remove old exact bonus code
- Verify no conflicting logic remains

## Dependencies
- SEMRANK-2004a (SQL exact match CASE implemented)
- SEMRANK-0001 (search tool exists)

## Risk Assessment
- **Risk**: Regex complexity leading to incorrect transformations
  - **Mitigation**: Test thoroughly with unit tests covering all edge cases
- **Risk**: Old bonus logic not fully removed, causing double-counting
  - **Mitigation**: Grep for ILIKE to verify complete removal
- **Risk**: Normalization edge cases not covered
  - **Mitigation**: Add tests for discovered edge cases, document behavior

## Files/Packages Affected
- `/packages/maproom-mcp/src/tools/search.ts` (normalization function, integration)
- `/crates/maproom/src/search/fts.rs` (remove old bonus logic)
- `/packages/maproom-mcp/tests/unit/normalize.test.ts` (new test file)

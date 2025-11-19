# Ticket: SEMRANK-3004: Edge Case Testing

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing
- [ ] **Verified** - by the verify-ticket agent

## Agents
- integration-tester
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create tests for edge cases and error conditions to ensure robust handling of unusual inputs, missing data, and boundary conditions in the semantic ranking implementation.

## Background
Real-world search queries and database content include edge cases that can crash naive implementations:
- Chunks may have `null` symbol_name (documentation, comments, module-level code)
- Database may contain unknown or unexpected kind values
- Users may submit empty queries, very long queries, or special characters
- Case variations need consistent handling
- Multi-word and acronym queries require normalization

This ticket implements edge case validation from Phase 3 of the SEMRANK execution plan (plan.md, lines 198-207). The goal is defensive programming to prevent crashes and ensure graceful degradation for unusual inputs.

The edge case handling implementation was completed in SEMRANK-2007, and the query normalization (including acronym handling) was implemented in SEMRANK-2004b. This ticket validates those implementations work correctly.

## Acceptance Criteria
- [ ] Test: Null symbol_name chunks don't crash (return valid results with exact_mult = 1.0)
- [ ] Test: Unknown kind values fallback to 1.0 multiplier
- [ ] Test: Empty query returns error or empty array (doesn't crash)
- [ ] Test: Very long query (10000 chars) handled gracefully (truncate or reject)
- [ ] Test: Special characters in query (!@#$%^&*) handled safely
- [ ] Test: Case-insensitive exact match works (AUTHENTICATE == authenticate == Authenticate)
- [ ] Test: Multi-word queries normalized correctly ("validate provider" → "validate_provider")
- [ ] Test: Acronyms normalized correctly (XMLParser → xml_parser, HTTPSHandler → https_handler)
- [ ] Test: Consecutive capitals handled (validateHTTPRequest → validate_http_request)
- [ ] Test: Mixed case with numbers (Base64Encoder → base64_encoder)
- [ ] All edge case tests pass
- [ ] No unhandled exceptions or database errors

## Technical Requirements
- Create new test file: `/packages/maproom-mcp/tests/integration/edge-cases.test.ts`
- Test SQL edge case handling from SEMRANK-2007
- Test TypeScript query normalization from SEMRANK-2004b
- Use test corpus with intentionally problematic data (null symbol_name, unknown kinds)
- Capture and validate error messages (should be user-friendly, not stack traces)
- Add validation to `/packages/maproom-mcp/src/tools/search.ts` if edge cases expose missing checks
- Document edge case behavior in test comments

## Implementation Notes

### Test Categories

#### 1. Null/Missing Data Tests
```typescript
describe('Null and missing data', () => {
  it('handles null symbol_name without crashing', async () => {
    // Insert chunk with null symbol_name (e.g., module-level doc)
    // Search for content in that chunk
    // Verify: results returned, exact_mult = 1.0, no errors
  });

  it('handles missing kind with default multiplier', async () => {
    // Insert chunk with NULL or unexpected kind value
    // Verify: kind_mult defaults to 1.0
  });
});
```

#### 2. Query Validation Tests
```typescript
describe('Query validation', () => {
  it('rejects empty query', async () => {
    // Search with query = ''
    // Verify: error returned (not crash)
  });

  it('handles very long query', async () => {
    // Search with 10000 character query
    // Verify: truncated, rejected, or processed without timeout
  });

  it('handles special characters safely', async () => {
    // Search with !@#$%^&*()[]{}
    // Verify: no SQL injection, no crash
  });
});
```

#### 3. Normalization Tests (from SEMRANK-2004b)
```typescript
describe('Query normalization', () => {
  it('case-insensitive exact match', async () => {
    // Search: 'AUTHENTICATE', 'authenticate', 'Authenticate'
    // All should return same #1 result with exact_mult = 3.0
  });

  it('multi-word normalization', async () => {
    // 'validate provider' → 'validate_provider'
    // Should match symbol_name 'validate_provider'
  });

  it('acronym normalization', async () => {
    // 'XMLParser' → 'xml_parser'
    // 'HTTPSHandler' → 'https_handler'
    // 'IOManager' → 'io_manager'
  });

  it('consecutive capitals', async () => {
    // 'validateHTTPRequest' → 'validate_http_request'
    // 'parseXMLData' → 'parse_xml_data'
  });

  it('mixed case with numbers', async () => {
    // 'Base64Encoder' → 'base64_encoder'
    // 'SHA256Hash' → 'sha256_hash'
  });
});
```

#### 4. Database Edge Cases
```typescript
describe('Database edge cases', () => {
  it('handles chunks with very long content', async () => {
    // Chunk with 100KB content
    // Verify: search completes, preview truncated appropriately
  });

  it('handles empty chunks', async () => {
    // Chunk with empty content or ts_doc
    // Verify: doesn't appear in results or handled gracefully
  });

  it('handles duplicate symbol names', async () => {
    // Multiple chunks with same symbol_name, different kinds
    // Verify: ranking distinguishes by kind multiplier
  });
});
```

### Edge Case Handling Implementation
If tests expose missing validation, add to `/packages/maproom-mcp/src/tools/search.ts`:

```typescript
// Input validation
if (!query || query.trim() === '') {
  throw new Error('Query cannot be empty');
}

if (query.length > 1000) {
  query = query.substring(0, 1000); // or reject
}

// Sanitize special characters if needed
const sanitizedQuery = sanitizeQuery(query);
```

### Documentation
Each test should include a comment explaining:
- What edge case is being tested
- Expected behavior (error, fallback, graceful degradation)
- Why this edge case matters (real-world scenario)

Example:
```typescript
// Test: Null symbol_name (common in documentation chunks)
// Expected: No crash, exact_mult = 1.0 (no exact match possible)
// Real-world: Many docs/comments lack symbol_name
it('handles null symbol_name without crashing', async () => {
  // ...
});
```

## Dependencies
- SEMRANK-1006 (integration test framework)
- SEMRANK-2004b (query normalization with acronym handling)
- SEMRANK-2007 (edge case handling in SQL)
- SEMRANK-3003 (integration test examples for reference)

## Risk Assessment
- **Risk**: Tests may not cover all possible edge cases
  - **Mitigation**: Start with known problematic patterns (null, empty, special chars), expand based on findings
- **Risk**: Edge case fixes may require changes to SQL or TypeScript
  - **Mitigation**: Test-driven approach - write failing tests first, then fix
- **Risk**: Overly strict validation may reject legitimate queries
  - **Mitigation**: Balance safety with usability; document trade-offs
- **Risk**: Normalization may produce false positives (e.g., "IOError" → "io_error" vs "ioerror")
  - **Mitigation**: Document normalization behavior; accept some ambiguity as acceptable

## Files/Packages Affected
- `/packages/maproom-mcp/tests/integration/edge-cases.test.ts` (new file)
- `/packages/maproom-mcp/src/tools/search.ts` (add validation if gaps found)
- `/packages/maproom-mcp/src/utils/query-normalization.ts` (if normalization needs fixes)

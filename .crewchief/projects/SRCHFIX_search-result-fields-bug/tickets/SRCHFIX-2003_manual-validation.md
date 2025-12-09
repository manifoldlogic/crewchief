# Ticket: [SRCHFIX-2003]: Manual Validation

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
- typescript-expert
- verify-ticket
- commit-ticket

## Summary
Perform manual end-to-end validation that chunk_id, symbol_name, and kind fields appear correctly in live MCP server responses and enable context retrieval.

## Background
After automated tests pass, we need human verification that the bug fix works in a realistic scenario. This involves running the MCP server and inspecting actual JSON responses to confirm all three fields are present with valid values.

This ticket implements Task 2.3 from the execution plan: Manual Validation.

## Acceptance Criteria
- [ ] MCP server built and started successfully
- [ ] Search performed via MCP client or daemon returns valid results
- [ ] chunk_id field present in results and > 0
- [ ] symbol_name field present (non-empty string or null)
- [ ] kind field present (non-empty string like "function", "class", etc.)
- [ ] Context retrieval works using chunk_id from search results
- [ ] Validation results documented in completion notes with example JSON

## Technical Requirements
**Build and start MCP server**:
```bash
cd /workspace/packages/maproom-mcp
pnpm build
npx @crewchief/maproom-mcp
```

**Perform search** (via MCP client or direct daemon):
```typescript
const result = await client.search({
  query: 'function search',
  repo: 'crewchief',
  worktree: 'main'
})
```

**Verify fields in response**:
```typescript
console.log('chunk_id:', result.hits[0].chunk_id)     // Should be > 0
console.log('symbol_name:', result.hits[0].symbol_name)  // Should be string or null
console.log('kind:', result.hits[0].kind)             // Should be "function", "class", etc.
```

**Test context retrieval**:
```typescript
const context = await client.context({
  chunk_id: result.hits[0].chunk_id
})
console.log('Context items:', context.items.length)  // Should be > 0
```

## Implementation Notes
**Validation steps**:

1. **Build verification**:
   - Run `pnpm build` in maproom-mcp package
   - Confirm build succeeds with no errors
   - Check that compiled output exists

2. **Server startup**:
   - Start MCP server
   - Verify it connects to daemon successfully
   - Check logs for any warnings about missing fields

3. **Search validation**:
   - Perform search with known query
   - Inspect JSON response structure
   - Verify all three fields present and valid
   - Test multiple search results (not just first)

4. **Context retrieval**:
   - Extract chunk_id from search result
   - Call context tool with that chunk_id
   - Verify context returns successfully
   - Confirm context relates to the search hit

5. **Edge cases**:
   - Test anonymous code (null symbol_name)
   - Test different kinds (function, class, method)
   - Test with different search queries

**Documentation requirements**:
Capture in completion notes:
- Example JSON response showing all fields
- chunk_id values observed
- symbol_name examples (both null and non-null)
- kind values observed
- Context retrieval success confirmation

## Dependencies
- **Requires**: SRCHFIX-2002 (integration test passes)
- **Requires**: All Phase 1 and Phase 2 tickets complete

## Risk Assessment
- **Risk**: MCP server fails to start
  - **Mitigation**: Check build output, verify daemon is running, check logs
- **Risk**: Search returns no results
  - **Mitigation**: Verify database has indexed data, try different queries
- **Risk**: Fields present but with wrong values
  - **Mitigation**: Document actual vs expected values, investigate mapping code
- **Risk**: Context retrieval fails
  - **Mitigation**: Verify chunk_id is valid, check daemon logs for errors

## Files/Packages Affected
- `/workspace/packages/maproom-mcp/` (build and execution)
- No code changes (validation only)

## Verification Notes
Document in completion notes with this format:

```
Manual Validation Results
=========================

Build Status: ✓ Success

Server Status: ✓ Running

Search Results:
{
  "hits": [
    {
      "chunk_id": 1234,
      "symbol_name": "authenticateUser",
      "kind": "function",
      "file_path": "src/auth.ts",
      "start_line": 45,
      "end_line": 67,
      "score": 0.92
    }
  ]
}

Field Validation:
- chunk_id: ✓ Present, value 1234 (positive integer)
- symbol_name: ✓ Present, value "authenticateUser" (non-empty string)
- kind: ✓ Present, value "function" (valid kind)

Context Retrieval:
- Input: chunk_id 1234
- Output: 5 context items
- Status: ✓ Success

Edge Cases Tested:
- Anonymous code: symbol_name = null ✓
- Different kinds: function, class, method ✓
- Multiple results: All have valid fields ✓

Conclusion: All fields correctly populated and functional.
```

Include actual JSON snippets from testing to provide concrete evidence of success.

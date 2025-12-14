# Ticket: [SRCHREL-2003]: End-to-End Integration Tests

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
- test-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Create comprehensive end-to-end integration tests validating the complete search pipeline with relationship expansion, from MCP client through daemon to Rust backend.

## Background
Integration tests validate the entire relationship expansion feature across the TypeScript ↔ Rust boundary. These tests ensure type synchronization works correctly, JSON serialization round-trips successfully, and the feature delivers expected value end-to-end.

This implements Phase 2 deliverables: E2E integration tests, type sync validation, backward compatibility verification.

## Acceptance Criteria
- [ ] MCP integration test created: `packages/maproom-mcp/tests/search-relationships.test.ts`
- [ ] Test validates MCP tool accepts `include_related=true` parameter
- [ ] Test validates high-confidence results have `related` field populated
- [ ] Test validates `related` array contains correct structure (RelatedChunkResult fields)
- [ ] Test validates backward compatibility (without parameter, no `related` field)
- [ ] Test validates auto-enable (include_related=true enables confidence)
- [ ] Test validates empty result semantics (None vs Some([]))
- [ ] All integration tests pass with `npm test`
- [ ] JSON serialization round-trip test passes (Rust → JSON → TypeScript)

## Technical Requirements

### Test File Structure
Create `packages/maproom-mcp/tests/search-relationships.test.ts`:

```typescript
describe('Search with relationships integration', () => {
  let mcpClient: MaproomMCPClient;

  beforeAll(async () => {
    mcpClient = await setupTestMCPClient();
    await indexTestRepository();
  });

  it('returns related chunks for high-confidence results', async () => {
    const response = await mcpClient.call('search', {
      query: 'authentication handler',
      repo: 'test-repo',
      include_related: true,
    });

    expect(response.results).toBeDefined();
    expect(response.results.length).toBeGreaterThan(0);

    // Find high-confidence result
    const highConfResult = response.results.find(r =>
      r.confidence?.source_count >= 2 || r.confidence?.is_exact_match
    );

    expect(highConfResult).toBeDefined();
    expect(highConfResult!.related).toBeDefined();
    expect(Array.isArray(highConfResult!.related)).toBe(true);
    expect(highConfResult!.related!.length).toBeGreaterThan(0);
    expect(highConfResult!.related!.length).toBeLessThanOrEqual(5);

    // Validate RelatedChunkResult structure
    const relatedChunk = highConfResult!.related![0];
    expect(typeof relatedChunk.chunk_id).toBe('number');
    expect(typeof relatedChunk.relpath).toBe('string');
    expect(typeof relatedChunk.kind).toBe('string');
    expect(typeof relatedChunk.start_line).toBe('number');
    expect(typeof relatedChunk.end_line).toBe('number');
    expect(typeof relatedChunk.preview).toBe('string');
    expect(typeof relatedChunk.depth).toBe('number');
    expect(typeof relatedChunk.relevance).toBe('number');
    expect(typeof relatedChunk.relationship_type).toBe('string');
    expect([1, 2]).toContain(relatedChunk.depth);
    expect(relatedChunk.relevance).toBeGreaterThan(0);
    expect(relatedChunk.relevance).toBeLessThanOrEqual(1);
  });

  it('handles backward compatibility without include_related', async () => {
    const response = await mcpClient.call('search', {
      query: 'authentication handler',
      repo: 'test-repo',
      include_confidence: true,
      // include_related NOT specified
    });

    expect(response.results).toBeDefined();

    // No result should have related field
    for (const result of response.results) {
      expect(result.related).toBeUndefined();
    }
  });

  it('auto-enables confidence when include_related is true', async () => {
    const response = await mcpClient.call('search', {
      query: 'authentication handler',
      repo: 'test-repo',
      include_related: true,
      // include_confidence NOT specified
    });

    expect(response.results).toBeDefined();

    // High-confidence results should have confidence field (auto-enabled)
    const resultsWithConfidence = response.results.filter(r => r.confidence !== undefined);
    expect(resultsWithConfidence.length).toBeGreaterThan(0);
  });

  it('handles empty related array (no relationships found)', async () => {
    // Mock or find a high-confidence result with no relationships
    const response = await mcpClient.call('search', {
      query: 'isolated function',  // Function with no imports/calls
      repo: 'test-repo',
      include_related: true,
    });

    const resultWithEmptyRelated = response.results.find(r =>
      r.confidence && (r.confidence.source_count >= 2) && r.related && r.related.length === 0
    );

    // If found, validate empty array semantics
    if (resultWithEmptyRelated) {
      expect(resultWithEmptyRelated.related).toEqual([]);
    }
  });

  it('validates MAX_CONCURRENT_EXPANSIONS cap', async () => {
    // Search that returns 5+ high-confidence results
    const response = await mcpClient.call('search', {
      query: 'common term',
      repo: 'test-repo',
      include_related: true,
      limit: 10,
    });

    const resultsWithRelated = response.results.filter(r => r.related !== undefined);

    // At most 3 results should have related field (MAX_CONCURRENT_EXPANSIONS)
    expect(resultsWithRelated.length).toBeLessThanOrEqual(3);
  });

  it('validates relevance scores are within range', async () => {
    const response = await mcpClient.call('search', {
      query: 'authentication handler',
      repo: 'test-repo',
      include_related: true,
    });

    const resultWithRelated = response.results.find(r => r.related && r.related.length > 0);

    if (resultWithRelated) {
      for (const relatedChunk of resultWithRelated.related!) {
        expect(relatedChunk.relevance).toBeGreaterThan(0);
        expect(relatedChunk.relevance).toBeLessThanOrEqual(1);
      }
    }
  });
});
```

### Test Repository Setup
Create test repository with:
- Code with clear relationships (imports, calls, extends)
- Mix of high/low confidence chunks
- Isolated chunks (no relationships) for empty result testing
- Mix of production code and tests for edge weight validation

## Implementation Notes

Test data strategy:
- Use in-memory test database or test repository
- Create controlled relationships for predictable test outcomes
- Include edge cases (no relationships, cyclic graphs, >5 related chunks)

JSON round-trip validation:
- Rust serializes `RelatedChunkResult` → JSON
- Daemon client deserializes JSON → TypeScript `RelatedChunkResult`
- Tests validate all fields present and correctly typed

Empty result semantics:
- `result.related === undefined`: Expansion didn't run (low confidence or disabled)
- `result.related === []`: Expansion ran but found no relationships
- Tests should validate both cases

Performance validation (deferred to Phase 3):
- Phase 2 focuses on correctness
- Phase 3 adds performance regression tests

## Dependencies
- SRCHREL-2001 (TypeScript types)
- SRCHREL-2002 (MCP schema update)
- SRCHREL-1003 (Rust pipeline integration)

## Risk Assessment
- **Risk**: Test database doesn't have realistic relationships
  - **Mitigation**: Carefully design test data with known relationship patterns
- **Risk**: Type mismatch not caught by tests
  - **Mitigation**: Comprehensive field validation in tests

## Files/Packages Affected
- `packages/maproom-mcp/tests/search-relationships.test.ts` (new file)
- Test fixtures/data (create test repository or database)

## Verification Notes
The verify-ticket agent should check:
- All tests pass: `cd packages/maproom-mcp && npm test search-relationships.test.ts`
- Test coverage includes:
  - High-confidence results have related field
  - RelatedChunkResult structure validated
  - Backward compatibility (without parameter)
  - Auto-enable confidence
  - Empty result semantics
  - MAX_CONCURRENT_EXPANSIONS cap
  - Relevance score range validation
- Tests use realistic data (not trivial mocks)
- No flaky tests (deterministic outcomes)

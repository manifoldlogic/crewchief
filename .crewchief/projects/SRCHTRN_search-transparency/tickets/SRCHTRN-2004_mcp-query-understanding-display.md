# SRCHTRN-2004: MCP Query Understanding Display

## Title
Display query understanding metadata in MCP search responses

## Status
- [ ] **Implementation Complete**
- [ ] **Tests Passing**
- [ ] **Verified**
- [ ] **Committed**

## Agents
- **Primary**: typescript-engineer
- **unit-test-runner**: Execute tests
- **verify-ticket**: Acceptance criteria validation
- **commit-ticket**: Commit creation

## Summary
Update `packages/maproom-mcp/src/tools/search.ts` to include query understanding metadata in successful search responses. Format timing breakdown for readability and expose query interpretation details to MCP clients.

## Background
With Phase 2 Rust implementation (SRCHTRN-2002) populating metadata and TypeScript types (SRCHTRN-2003) enabling deserialization, the MCP tool can now display query understanding to users. This completes Phase 2 by making query interpretation visible on every search.

**User Impact**: Users will see how their queries are interpreted (mode, tokens, expanded terms, timing) alongside search results.

## Acceptance Criteria
- [ ] Search response includes `metadata.understanding` when available
- [ ] Timing breakdown formatted for readability (ms precision)
- [ ] Query mode, tokens, and expanded terms displayed
- [ ] Filters and fusion strategy included
- [ ] Integration test validates query understanding visibility
- [ ] Manual test: Search "authenticate user" shows mode=auto, tokens, expanded terms
- [ ] E2E test validates metadata present in 100% of successful searches
- [ ] All tests passing

## Technical Requirements

### Update Search Response: `packages/maproom-mcp/src/tools/search.ts`

```typescript
import type { QueryUnderstanding } from '@crewchief/daemon-client'

export async function handleSearchTool(
  params: SearchParams,
  client: DaemonClient
): Promise<MCPResponse> {
  try {
    const results = await client.search(params)

    // Format search results with query understanding
    return {
      isError: false,
      content: [
        {
          type: 'text',
          text: JSON.stringify(
            {
              results: results.hits.map(formatHit),
              total: results.total,
              metadata: results.metadata.understanding
                ? formatQueryUnderstanding(results.metadata.understanding)
                : undefined,
            },
            null,
            2
          ),
        },
      ],
    }
  } catch (error) {
    return formatSearchError(error)
  }
}

function formatQueryUnderstanding(understanding: QueryUnderstanding) {
  return {
    query_interpretation: {
      mode: understanding.mode,
      tokens: understanding.tokens,
      expanded_terms: understanding.expanded_terms,
    },
    filters: {
      repo_id: understanding.filters.repo_id,
      worktree_id: understanding.filters.worktree_id,
      file_types: understanding.filters.file_types,
    },
    fusion_strategy: understanding.fusion_strategy,
    timing: {
      query_processing: `${understanding.timing.query_processing_ms.toFixed(1)}ms`,
      search_execution: `${understanding.timing.search_execution_ms.toFixed(1)}ms`,
      score_fusion: `${understanding.timing.score_fusion_ms.toFixed(1)}ms`,
      result_assembly: `${understanding.timing.result_assembly_ms.toFixed(1)}ms`,
      total: `${understanding.timing.total_ms.toFixed(1)}ms`,
    },
  }
}
```

### Example Response Format
```json
{
  "results": [
    {
      "file": "src/auth/user.rs",
      "chunk": "...",
      "score": 0.95
    }
  ],
  "total": 10,
  "metadata": {
    "query_interpretation": {
      "mode": "auto",
      "tokens": ["authenticate", "user"],
      "expanded_terms": ["auth", "login", "authentication"]
    },
    "filters": {
      "repo_id": 1,
      "worktree_id": 2,
      "file_types": []
    },
    "fusion_strategy": "reciprocal_rank_fusion",
    "timing": {
      "query_processing": "4.2ms",
      "search_execution": "35.8ms",
      "score_fusion": "2.1ms",
      "result_assembly": "6.4ms",
      "total": "48.5ms"
    }
  }
}
```

### Integration Test: `packages/maproom-mcp/tests/query-understanding-display.test.ts`

```typescript
import { handleSearchTool } from '../src/tools/search.js'

describe('Query understanding display', () => {
  it('should include query understanding in successful search', async () => {
    // Mock daemon client with understanding metadata
    const mockClient = createMockDaemonClient({
      search: () => ({
        hits: [
          { file: 'test.rs', chunk: 'test content', score: 0.95 }
        ],
        total: 1,
        metadata: {
          understanding: {
            mode: 'auto',
            tokens: ['authenticate', 'user'],
            expanded_terms: ['auth', 'login'],
            filters: {
              repo_id: 1,
              worktree_id: null,
              file_types: [],
              recency_threshold: null
            },
            fusion_strategy: 'reciprocal_rank_fusion',
            timing: {
              query_processing_ms: 4.2,
              search_execution_ms: 35.8,
              score_fusion_ms: 2.1,
              result_assembly_ms: 6.4,
              total_ms: 48.5
            }
          }
        }
      })
    })

    const result = await handleSearchTool(
      { query: 'authenticate user', repo: 'crewchief' },
      mockClient
    )

    expect(result.isError).toBe(false)
    const response = JSON.parse(result.content[0].text)

    // Verify query understanding included
    expect(response.metadata).toBeDefined()
    expect(response.metadata.query_interpretation.mode).toBe('auto')
    expect(response.metadata.query_interpretation.tokens).toEqual([
      'authenticate',
      'user'
    ])
    expect(response.metadata.timing.total).toBe('48.5ms')
  })

  it('should handle responses without understanding (backward compat)', async () => {
    const mockClient = createMockDaemonClient({
      search: () => ({
        hits: [],
        total: 0,
        metadata: {} // No understanding field
      })
    })

    const result = await handleSearchTool(
      { query: 'test', repo: 'crewchief' },
      mockClient
    )

    expect(result.isError).toBe(false)
    const response = JSON.parse(result.content[0].text)
    expect(response.metadata).toBeUndefined()
  })
})
```

### Manual E2E Test Checklist

```bash
# Test 1: Successful search with understanding
npx @crewchief/maproom-mcp
# → Search query="authenticate user" repo="crewchief"
# Expected: Response includes metadata with:
#   - mode: "auto"
#   - tokens: ["authenticate", "user"]
#   - expanded_terms: ["auth", "login", "authentication"]
#   - timing breakdown with total ~50-100ms

# Test 2: Different search modes
# → Search query="User::authenticate" repo="crewchief"
# Expected: mode: "code"

# → Search query="how to authenticate a user" repo="crewchief"
# Expected: mode: "text"

# Test 3: Timing breakdown accuracy
# Expected: total_ms approximately equals sum of parts
# Expected: All timing values > 0ms
```

## Implementation Notes
1. Add query understanding formatting to search response
2. Format timing values with 1 decimal place (e.g., "4.2ms")
3. Keep metadata optional (backward compatibility)
4. Display only if `understanding` field present
5. Organize into logical sections: interpretation, filters, timing

**Formatting Guidelines**:
- Timing: 1 decimal place precision (e.g., "35.8ms")
- Arrays: Display as-is (no special formatting)
- Optional fields: Omit if null/undefined

## Dependencies
- **SRCHTRN-2002**: Metadata assembly in pipeline (provides data)
- **SRCHTRN-2003**: TypeScript query understanding types (provides interfaces)

## Risk Assessment
**Risk Level**: Low

**Risks**:
- Breaking existing MCP response format
- Timing display confusing to users

**Mitigations**:
- Metadata field is optional (backward compatible)
- Clear formatting with labeled sections
- Integration tests validate structure

## Files/Packages Affected
- **Modified**: `packages/maproom-mcp/src/tools/search.ts` (~40 lines added)
- **New file**: `packages/maproom-mcp/tests/query-understanding-display.test.ts`
- **Import**: `import type { QueryUnderstanding } from '@crewchief/daemon-client'`

## Estimated Effort
3-4 hours

**Breakdown**:
- Response formatting: 1-2 hours
- Integration tests: 1-2 hours
- Manual E2E testing: 1 hour

## Planning References
- [plan.md](../planning/plan.md) - Phase 2 ticket breakdown, acceptance tests
- [architecture.md](../planning/architecture.md) - MCP query understanding display design
- [quality-strategy.md](../planning/quality-strategy.md) - E2E testing approach

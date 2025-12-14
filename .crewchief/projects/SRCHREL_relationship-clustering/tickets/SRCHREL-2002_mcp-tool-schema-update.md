# Ticket: [SRCHREL-2002]: MCP Search Tool Schema Update

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
- mcp-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Update the MCP search tool schema to expose the `include_related` parameter with documentation and validate parameter passing through the daemon client.

## Background
The MCP search tool is the user-facing API for relationship-aware search. Adding the `include_related` parameter enables clients to opt into relationship expansion. Clear documentation is critical for user adoption.

This implements Phase 2 deliverables: MCP tool schema update and parameter documentation.

## Acceptance Criteria
- [ ] `include_related` parameter added to MCP search tool schema
- [ ] Parameter type is boolean with default false
- [ ] Parameter description is comprehensive and mentions confidence auto-enable
- [ ] Example usage documented in schema or tool description
- [ ] Daemon client passes parameter correctly to Rust backend
- [ ] MCP tool tests validate parameter acceptance
- [ ] Backward compatibility verified (without parameter still works)

## Technical Requirements

### Schema Update
Update `packages/maproom-mcp/src/tools/search_schema.ts` (or equivalent):

```typescript
export const searchToolSchema = {
  name: 'search',
  description: 'Search indexed code using full-text, vector, or hybrid search',
  inputSchema: {
    type: 'object',
    properties: {
      // ... existing parameters ...
      include_confidence: {
        type: 'boolean',
        description: 'Include confidence scoring signals for each result. Default: false.',
        default: false,
      },
      include_related: {
        type: 'boolean',
        description: 'Include related chunks for high-confidence results via graph traversal. Automatically enables confidence scoring. Default: false.',
        default: false,
      },
    },
    required: ['query', 'repo'],
  },
};
```

### Parameter Documentation
Include in description or separate usage docs:
- **What it does**: Finds top 5 related chunks for high-confidence results
- **Confidence threshold**: source_count >= 2 OR is_exact_match
- **Performance impact**: <20ms overhead
- **Auto-enable**: Confidence scoring automatically enabled when include_related=true
- **Response structure**: Results with high confidence get `related` array field

### Example Usage
```typescript
// Basic search with relationships
{
  "query": "authentication handler",
  "repo": "my-app",
  "include_related": true
}

// Explicit confidence + relationships (redundant but allowed)
{
  "query": "error handling",
  "repo": "my-app",
  "include_confidence": true,
  "include_related": true
}
```

### Daemon Client Integration
Ensure `packages/daemon-client/src/client.ts` passes parameter:

```typescript
async search(params: SearchParams): Promise<SearchResponse> {
  // Validate params
  const requestParams = {
    query: params.query,
    repo: params.repo,
    worktree: params.worktree,
    limit: params.limit,
    mode: params.mode,
    debug: params.debug,
    include_confidence: params.include_confidence,
    include_related: params.include_related,  // NEW
    deduplicate: params.deduplicate,
  };

  return this.sendRequest('search', requestParams);
}
```

## Implementation Notes

Parameter design principles:
- Boolean flag (not enum or string) for simplicity
- Default false (opt-in feature) for safety
- Clear naming (`include_related` matches `include_confidence`)
- Description mentions auto-enable to avoid user confusion

Documentation clarity:
- Explain confidence threshold (users need to understand when expansion happens)
- Set performance expectations (<20ms overhead)
- Provide example showing typical usage

Testing considerations:
- Test with parameter set to true
- Test with parameter set to false
- Test with parameter omitted (default behavior)
- Test auto-enable (include_related=true should enable confidence)

## Dependencies
- SRCHREL-2001 (TypeScript types must exist)
- SRCHREL-1003 (Rust backend must accept parameter)

## Risk Assessment
- **Risk**: Schema description is unclear, users don't understand when to use it
  - **Mitigation**: Comprehensive description with examples; Phase 3 adds user documentation
- **Risk**: Parameter not passed correctly through daemon client
  - **Mitigation**: Integration tests validate end-to-end parameter flow

## Files/Packages Affected
- `packages/maproom-mcp/src/tools/search_schema.ts` (or equivalent schema definition)
- `packages/daemon-client/src/client.ts` (ensure parameter passed)
- `packages/maproom-mcp/src/tools/search_tool.ts` (if parameter handling needed)

## Verification Notes
The verify-ticket agent should check:
- `include_related` parameter present in schema with correct type (boolean)
- Parameter description is comprehensive (mentions auto-enable, threshold, performance)
- Example usage documented or easily accessible
- MCP tool tests validate parameter acceptance
- Backward compatibility test passes (search without parameter works)
- Parameter correctly passed from MCP tool → daemon client → Rust backend
- No TypeScript compilation errors in MCP package

# Ticket: MCP_CORE-1004: Explain Tool Implementation

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- mcp-tools-engineer
- test-runner (e.g. unit-test-runner)
- verify-ticket
- commit-ticket

## Summary
Implement the Explain tool for the MCP server, which generates symbol cards for code chunks with metadata, relationships, and usage examples. The tool provides markdown-formatted explanations with intelligent caching to improve performance.

## Background
The Explain tool is a supporting feature in Phase 1 (Week 2) of the MCP_CORE project. It provides detailed explanations of code symbols to help users understand code chunks returned from search results. This tool enhances the developer experience by providing rich context about symbols, including their relationships, usage patterns, and metadata.

The tool is marked as experimental in the configuration and will be disabled by default until fully tested and validated.

## Acceptance Criteria
- [ ] Symbol card generation working - generates cards with chunk metadata, relationships, and examples
- [ ] Caching logic functional - checks cache before generating, stores results after generation
- [ ] Template system for formatting - reusable template for consistent card structure
- [ ] Markdown output correct - properly formatted markdown with appropriate sections
- [ ] Unit tests pass - comprehensive test coverage for all functionality
- [ ] Integration with MCP server - tool properly registered and accessible via MCP protocol

## Technical Requirements
- **Parameter Validation**: Use Zod schema to validate chunk_id (required parameter)
- **Database Query**: Query database for chunk details including metadata and relationships
- **Cache Layer**: Implement cache check/set with key pattern `explain:${chunk_id}`
- **Symbol Card Generation**: Create SymbolCard object with:
  - Chunk metadata (file path, line numbers, language)
  - Symbol relationships (imports, exports, dependencies)
  - Usage examples (if available)
  - Code preview/snippet
- **Markdown Formatting**: Convert SymbolCard to well-formatted markdown
- **Error Handling**: Gracefully handle missing chunks, database errors, cache failures
- **Configuration**: Tool must respect `mcp.tools.explain.enabled` config flag (default: false)

## Implementation Notes

### Architecture Reference
See `/workspace/crewchief_context/maproom/MCP_CORE/MCP_CORE_ARCHITECTURE.md` lines 129-148 for the Explain tool architecture pattern.

### Implementation Pattern
```typescript
class ExplainTool {
  async execute(params: ExplainParams): Promise<SymbolCard> {
    const chunk = await this.db.getChunk(params.chunk_id);

    // Check cache
    const cached = await this.cache.get(`explain:${params.chunk_id}`);
    if (cached) return cached;

    // Generate explanation
    const card = await this.generateCard(chunk);

    // Cache result
    await this.cache.set(`explain:${params.chunk_id}`, card);

    return card;
  }
}
```

### Key Components
1. **Tool Handler** (`explain.ts`): Main execution logic with cache checking
2. **Schema Definition** (`explain_schema.ts`): Zod validation for chunk_id parameter
3. **Symbol Card Template** (`symbol_card.ts`): Reusable template for card structure
4. **Cache Utilities** (`cache.ts`): Generic caching layer (may be shared with other tools)
5. **Unit Tests** (`explain_test.ts`): Test cache hits/misses, card generation, error cases

### Caching Strategy
- Use in-memory cache with configurable TTL
- Cache key pattern: `explain:${chunk_id}`
- Consider cache invalidation when chunks are updated (upsert operations)
- Track cache hit/miss metrics for monitoring

### Template Structure
Symbol card should include:
- **Header**: Symbol name and type
- **Location**: File path, line range
- **Metadata**: Language, symbol kind, visibility
- **Relationships**: Imports, exports, references
- **Preview**: Code snippet with syntax highlighting
- **Examples**: Usage patterns (if available)

## Dependencies
- Database schema with chunk metadata (assumed to be in place)
- Database access layer for querying chunks
- MCP server base infrastructure (from other Phase 1 tasks)
- Zod library for schema validation

## Risk Assessment
- **Risk**: Cache invalidation complexity when chunks are updated
  - **Mitigation**: Start with simple TTL-based expiration, add smarter invalidation in Phase 2 if needed

- **Risk**: Performance impact of card generation for large symbols
  - **Mitigation**: Set reasonable limits on preview size, use streaming for large outputs if needed

- **Risk**: Database queries for relationships may be slow
  - **Mitigation**: Ensure proper indexes on relationship tables, use caching aggressively

- **Risk**: Tool is experimental and may need iteration based on usage
  - **Mitigation**: Keep disabled by default, gather feedback before enabling in production

## Files/Packages Affected
- `packages/maproom-mcp/src/tools/explain.ts` - Explain tool handler (new)
- `packages/maproom-mcp/src/tools/explain_schema.ts` - Zod schema for validation (new)
- `packages/maproom-mcp/src/templates/symbol_card.ts` - Symbol card template (new)
- `packages/maproom-mcp/src/utils/cache.ts` - Caching utilities (new, may be shared)
- `packages/maproom-mcp/src/server.ts` - Register explain tool (modify)
- `packages/maproom-mcp/tests/tools/explain_test.ts` - Unit tests (new)
- `packages/maproom-mcp/config/default.yaml` - Tool configuration (modify)

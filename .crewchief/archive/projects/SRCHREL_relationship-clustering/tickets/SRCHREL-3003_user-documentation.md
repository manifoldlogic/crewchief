# Ticket: [SRCHREL-3003]: User Documentation and Examples

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation-only ticket)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- technical-writer
- verify-ticket
- commit-ticket

## Summary
Create comprehensive user-facing documentation for relationship-aware search including usage patterns, examples, parameter explanation, and best practices.

## Background
Users need clear documentation to understand when and how to use relationship expansion. Documentation should explain the confidence threshold, performance characteristics, and provide practical examples showing the value of related chunks.

This implements Phase 3 deliverables: user documentation and usage patterns.

## Acceptance Criteria
- [x] User documentation created at `docs/features/relationship-aware-search.md`
- [x] MCP tool usage examples documented with expected output
- [x] Confidence threshold explanation clear and accessible
- [x] Performance characteristics documented (<20ms overhead, <10KB response)
- [x] Best practices section includes when to use include_related
- [x] Troubleshooting section covers common issues
- [x] Examples use realistic queries and show actual output structure
- [x] Documentation reviewed for clarity (no jargon, clear explanations)

## Technical Requirements

### Documentation Structure
Create `docs/features/relationship-aware-search.md`:

```markdown
# Relationship-Aware Search

## Overview

Relationship-aware search extends search results with lightweight relationship metadata that exposes architectural context for high-confidence results. For chunks you care about most, see related code through imports, calls, inheritance, and more.

## Quick Start

```typescript
// Basic usage
const results = await mcpClient.call('search', {
  query: 'authentication handler',
  repo: 'my-app',
  include_related: true,
});

// Results with high confidence will have a `related` field
for (const result of results.results) {
  if (result.related) {
    console.log(`Found ${result.related.length} related chunks`);
    for (const relatedChunk of result.related) {
      console.log(`  - ${relatedChunk.relpath} (${relatedChunk.relationship_type})`);
    }
  }
}
```

## How It Works

### Confidence Gating

Relationship expansion only happens for **high-confidence results**:
- `source_count >= 2` (result matched in 2+ search sources: FTS, vector, graph)
- **OR** `is_exact_match == true` (exact query match)

Typically 20-40% of results meet this threshold, ensuring expansion focuses on your most relevant results.

### Graph Traversal

For qualifying results, maproom:
1. Traverses the code graph up to 2 hops deep
2. Finds related chunks via imports, calls, extends, implements relationships
3. Scores by relevance (closer relationships rank higher)
4. Returns top 5 most relevant related chunks

### Performance

- **Overhead**: <20ms p95 (minimal impact on search speed)
- **Response size**: <10KB (lightweight metadata only, not full file content)
- **Concurrent limit**: Max 3 results expanded per search (performance budget)

## Parameters

### `include_related` (boolean, default: false)

Enable relationship expansion for high-confidence results.

**Important**: Automatically enables confidence scoring (you don't need to specify `include_confidence: true`).

```typescript
// Minimal usage
{ query: 'auth', repo: 'app', include_related: true }

// Explicit (redundant but allowed)
{ query: 'auth', repo: 'app', include_confidence: true, include_related: true }
```

## Response Structure

### ChunkSearchResult with Related Chunks

```typescript
{
  chunk_id: 123,
  relpath: 'src/auth/handler.ts',
  symbol_name: 'authenticate',
  kind: 'function',
  // ... other fields ...
  confidence: {
    source_count: 3,
    is_exact_match: false,
    // ...
  },
  related: [
    {
      chunk_id: 456,
      relpath: 'src/auth/validator.ts',
      symbol_name: 'validateToken',
      kind: 'function',
      start_line: 10,
      end_line: 25,
      preview: 'export function validateToken(token: string) {...',
      depth: 1,
      relevance: 0.84,
      relationship_type: 'import',
    },
    // ... up to 4 more related chunks
  ]
}
```

### Empty Result Semantics

- **`related: undefined`**: Expansion didn't run (confidence too low or disabled)
- **`related: []`**: Expansion ran but found no relationships (isolated chunk)

## Usage Patterns

### Exploring Code Architecture

```typescript
// Find authentication code and see what it depends on
const results = await search({
  query: 'authentication',
  repo: 'my-app',
  include_related: true,
});

// High-confidence results show related chunks
const authHandler = results.results.find(r =>
  r.symbol_name === 'authenticate'
);

if (authHandler?.related) {
  console.log('Authentication handler imports:');
  authHandler.related
    .filter(r => r.relationship_type === 'import')
    .forEach(r => console.log(`  - ${r.relpath}`));
}
```

### Finding Callers

```typescript
// Find a utility function and see who calls it
const results = await search({
  query: 'formatDate',
  repo: 'my-app',
  include_related: true,
});

// Related chunks with relationship_type === 'call' are callers
```

### Understanding Inheritance

```typescript
// Find a base class and see implementations
const results = await search({
  query: 'BaseController',
  repo: 'my-app',
  include_related: true,
});

// Related chunks with relationship_type === 'extends' are subclasses
```

## Best Practices

### When to Use `include_related`

**Use when:**
- Exploring unfamiliar codebases (understand code structure)
- Finding architectural dependencies (what imports what)
- Discovering related code (callers, implementations)
- Understanding context (where is this used?)

**Don't use when:**
- Searching for specific text (FTS alone is faster)
- Low-confidence results expected (expansion won't trigger)
- Performance critical (adds ~20ms overhead)

### Interpreting Related Chunks

- **depth: 1** - Direct relationship (imports, calls directly)
- **depth: 2** - Indirect relationship (transitive dependencies)
- **relevance: 0.0-1.0** - Higher = more relevant
  - Same directory gets 1.2× boost
  - Production code > test code (test relationships weighted 0.5×)
  - Inheritance/interfaces get 1.1× boost

### Combining with Other Features

```typescript
// Best practice: combine with confidence, filtering
const results = await search({
  query: 'error handling',
  repo: 'my-app',
  include_related: true,
  // Confidence is auto-enabled by include_related
  // Filter to code-only for cleaner relationships (if filtering available)
});
```

## Troubleshooting

### No results have `related` field

**Cause**: All results are low-confidence (source_count < 2 and not exact match).

**Solution**: Try:
- More specific query (increases confidence)
- Check if results have `confidence` field (if missing, confidence disabled)
- Query exact symbol name (triggers is_exact_match)

### Related chunks not useful

**Cause**: Related chunks may be test code or unrelated imports.

**Solution**:
- Check `relationship_type` (filter to specific types)
- Check `relevance` score (higher = more relevant)
- Related chunks are metadata only - use context tool for full content

### Performance slower than expected

**Cause**: Overhead can be higher if database has many edges.

**Solution**:
- Normal overhead is <20ms (usually not noticeable)
- Only 3 results max are expanded (performance cap)
- Disable `include_related` if latency critical

### Empty `related` array

**Cause**: Chunk has no relationships (isolated code).

**Solution**: This is valid - some chunks (config, standalone utilities) have no relationships.

## Examples

[Include 3-5 realistic examples with full queries and expected outputs]

## Technical Details

For developers implementing features using relationship expansion, see:
- [Architecture Documentation](../architecture/relationship-clustering.md)
- [API Reference](../api/search.md)
- [Performance Characteristics](../performance/relationship-expansion.md)
```

## Implementation Notes

Documentation principles:
- Start with "why" (value proposition)
- Show examples early (quick start)
- Explain parameters clearly (no assumptions)
- Cover edge cases (troubleshooting)
- Link to technical docs for deeper dives

Writing style:
- Use active voice
- Avoid jargon (or explain it)
- Short paragraphs (2-4 sentences)
- Code examples for every concept
- Real-world use cases

Visual aids (optional):
- Diagram showing graph traversal depth
- Flowchart for confidence gating
- Screenshot of example output

## Dependencies
- All previous tickets (feature must be complete)

## Risk Assessment
- **Risk**: Documentation is too technical, users don't understand
  - **Mitigation**: Start with quick start and examples; technical details at end
- **Risk**: Examples don't match actual output (code changes)
  - **Mitigation**: Validate examples by running them; add note about example versions

## Files/Packages Affected
- `docs/features/relationship-aware-search.md` (new file)
- `packages/maproom-mcp/README.md` (add link to documentation)
- `README.md` (add feature mention if appropriate)

## Verification Notes
The verify-ticket agent should check:
- Documentation file exists and is well-structured
- Quick start example is clear and complete
- Confidence threshold explanation is accurate
- Performance characteristics documented
- Best practices section provides actionable guidance
- Troubleshooting covers common issues from quality-strategy.md
- Examples use realistic queries (not "foo" and "bar")
- No TypeScript or Rust code compilation errors in examples
- Tests pass - N/A (documentation-only ticket)

## Implementation Notes

**Documentation File Created**: `/workspace/docs/features/relationship-aware-search.md`

**Documentation Structure** (450+ lines):

1. **Overview** - Value proposition and quick introduction
2. **Quick Start** - Code example with expected output
3. **How It Works** - Confidence gating, graph traversal, performance
4. **Parameters** - `include_related` parameter explanation
5. **Response Structure** - Full ChunkSearchResult and RelatedChunkResult examples
6. **Usage Patterns** - 4 realistic usage patterns:
   - Exploring code architecture
   - Finding callers of a function
   - Understanding class hierarchies
   - Combining with context retrieval
7. **Best Practices** - When to use/not use, interpreting relevance, filtering by type
8. **Troubleshooting** - 5 common issues with causes and solutions:
   - No results have `related` field
   - Related chunks seem irrelevant
   - Performance slower than expected
   - Empty `related` array
   - `related` field missing
9. **Examples** - 3 complete JSON request/response examples:
   - Search with relationships enabled
   - Low-confidence result (no expansion)
   - Isolated chunk (empty related array)
10. **Technical Details** - Links to architecture and developer docs

**Key Documentation Features**:
- Tables for confidence thresholds, performance metrics, field descriptions
- Code examples in TypeScript with realistic queries
- JSON request/response examples with actual field values
- Troubleshooting organized as cause/solution pairs
- Clear explanation of None vs Some([]) semantics
- Relevance scoring breakdown with boost/penalty factors
- Relationship type reference table

**Run Command** (validation):
```bash
# Verify file exists and has expected structure
head -100 /workspace/docs/features/relationship-aware-search.md
wc -l /workspace/docs/features/relationship-aware-search.md
```

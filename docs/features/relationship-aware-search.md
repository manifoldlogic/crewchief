# Relationship-Aware Search

Discover architectural context for your most relevant search results.

## Overview

Relationship-aware search extends search results with lightweight relationship metadata. For high-confidence results, you'll see related code through imports, calls, inheritance, and more - helping you understand code architecture without manually tracing dependencies.

## Quick Start

```typescript
// Enable relationship expansion
const results = await mcpClient.call('search', {
  query: 'authentication handler',
  repo: 'my-app',
  include_related: true,
});

// High-confidence results have a `related` field
for (const result of results.results) {
  if (result.related) {
    console.log(`${result.symbol_name} has ${result.related.length} related chunks:`);
    for (const chunk of result.related) {
      console.log(`  - ${chunk.symbol_name} in ${chunk.relpath} (${chunk.relationship_type})`);
    }
  }
}
```

**Example output:**
```
authenticate has 3 related chunks:
  - validateToken in src/auth/validator.ts (import)
  - createSession in src/auth/session.ts (call)
  - UserCredentials in src/auth/types.ts (import)
```

## How It Works

### Confidence Gating

Relationship expansion only runs for **high-confidence results** to avoid noise:

| Condition | Threshold | Example |
|-----------|-----------|---------|
| Multiple sources | `source_count >= 2` | Result matched in both FTS and vector search |
| Exact match | `is_exact_match == true` | Query exactly matches symbol name |

Typically 20-40% of results meet this threshold, focusing expansion on your most relevant results.

### Graph Traversal

For qualifying results, Maproom:

1. **Traverses** the code graph up to 2 hops deep
2. **Finds** related chunks via imports, calls, extends, implements relationships
3. **Scores** by relevance (closer relationships rank higher)
4. **Returns** top 5 most relevant related chunks

### Performance Characteristics

| Metric | Budget | Typical |
|--------|--------|---------|
| Overhead | <20ms p95 | ~2-5ms |
| Response size | <10KB | ~200 bytes per related chunk |
| Results expanded | Max 3 | Performance cap |

## Parameters

### `include_related` (boolean)

**Default:** `false`

Enable relationship expansion for high-confidence results.

```typescript
// Enable relationships (confidence is auto-enabled)
{ query: 'auth', repo: 'app', include_related: true }

// Explicit (redundant but allowed)
{ query: 'auth', repo: 'app', include_confidence: true, include_related: true }
```

**Note:** Setting `include_related: true` automatically enables confidence scoring. You don't need to specify `include_confidence: true` separately.

## Response Structure

### ChunkSearchResult with Related Chunks

```typescript
{
  chunk_id: 123,
  relpath: 'src/auth/handler.ts',
  symbol_name: 'authenticate',
  kind: 'function',
  start_line: 45,
  end_line: 72,
  preview: 'export async function authenticate(request: Request) {...',
  score: 0.95,
  confidence: {
    source_count: 3,       // Matched in FTS, vector, and graph
    score_gap: 0.12,       // Gap to next result
    is_exact_match: false
  },
  related: [
    {
      chunk_id: 456,
      relpath: 'src/auth/validator.ts',
      symbol_name: 'validateToken',
      kind: 'function',
      start_line: 10,
      end_line: 25,
      preview: 'export function validateToken(token: string): boolean {...',
      depth: 1,                    // Direct relationship
      relevance: 0.84,             // High relevance (same directory)
      relationship_type: 'import'  // This chunk imports validateToken
    },
    {
      chunk_id: 789,
      relpath: 'src/session/manager.ts',
      symbol_name: 'createSession',
      kind: 'function',
      start_line: 30,
      end_line: 55,
      preview: 'export async function createSession(userId: string) {...',
      depth: 1,
      relevance: 0.72,
      relationship_type: 'call'    // This chunk calls createSession
    }
    // ... up to 5 related chunks total
  ]
}
```

### RelatedChunkResult Fields

| Field | Type | Description |
|-------|------|-------------|
| `chunk_id` | number | Unique identifier for requesting full context |
| `relpath` | string | File path relative to repository root |
| `symbol_name` | string \| null | Symbol name (null for anonymous chunks) |
| `kind` | string | Symbol kind: function, class, interface, etc. |
| `start_line` | number | Start line (1-based) |
| `end_line` | number | End line (1-based) |
| `preview` | string | First 100 characters of code (truncated) |
| `depth` | number | Graph traversal depth (1 = direct, 2 = indirect) |
| `relevance` | number | Relevance score (0.0-1.0, higher = more relevant) |
| `relationship_type` | string | Type of relationship: import, call, extends, etc. |

### Empty Result Semantics

Understanding when `related` is absent vs empty:

| Value | Meaning |
|-------|---------|
| `related: undefined` | Expansion didn't run (confidence too low or disabled) |
| `related: []` | Expansion ran but found no relationships (isolated chunk) |

## Usage Patterns

### Exploring Code Architecture

Find a feature and understand its dependencies:

```typescript
const results = await search({
  query: 'authentication middleware',
  repo: 'my-app',
  include_related: true,
});

// Find the main authentication handler
const authHandler = results.results.find(r =>
  r.symbol_name?.includes('authenticate') && r.related
);

if (authHandler) {
  console.log('Authentication dependencies:');

  // Group by relationship type
  const imports = authHandler.related.filter(r => r.relationship_type === 'import');
  const calls = authHandler.related.filter(r => r.relationship_type === 'call');

  console.log('  Imports:', imports.map(r => r.symbol_name).join(', '));
  console.log('  Calls:', calls.map(r => r.symbol_name).join(', '));
}
```

### Finding Callers of a Function

Discover who uses a specific function:

```typescript
const results = await search({
  query: 'formatDate',
  repo: 'my-app',
  include_related: true,
});

// Related chunks with relationship_type 'call' are callers
const formatDateResult = results.results.find(r =>
  r.symbol_name === 'formatDate' && r.related
);

if (formatDateResult) {
  const callers = formatDateResult.related.filter(r =>
    r.relationship_type === 'call'
  );

  console.log('Functions that call formatDate:');
  for (const caller of callers) {
    console.log(`  - ${caller.symbol_name} in ${caller.relpath}`);
  }
}
```

### Understanding Class Hierarchies

Find implementations of an interface or subclasses:

```typescript
const results = await search({
  query: 'BaseController',
  repo: 'my-app',
  include_related: true,
});

const baseController = results.results.find(r =>
  r.symbol_name === 'BaseController' && r.related
);

if (baseController) {
  const implementations = baseController.related.filter(r =>
    r.relationship_type === 'extends' || r.relationship_type === 'implements'
  );

  console.log('Classes that extend BaseController:');
  for (const impl of implementations) {
    console.log(`  - ${impl.symbol_name} in ${impl.relpath}`);
  }
}
```

### Combining with Context Retrieval

Use relationship info to fetch full context for interesting related chunks:

```typescript
const results = await search({
  query: 'error handler',
  repo: 'my-app',
  include_related: true,
});

// Find a high-relevance related chunk
const mainResult = results.results.find(r => r.related?.length > 0);
const mostRelevant = mainResult?.related?.sort((a, b) => b.relevance - a.relevance)[0];

if (mostRelevant) {
  // Fetch full context for the related chunk
  const context = await mcpClient.call('context', {
    chunk_id: mostRelevant.chunk_id,
    callers: true,
    callees: true,
  });

  console.log('Full context for most relevant related chunk:', context);
}
```

## Best Practices

### When to Use `include_related`

**Recommended for:**
- Exploring unfamiliar codebases (understand structure)
- Finding architectural dependencies (what imports what)
- Discovering related code (callers, implementations)
- Understanding context (where is this code used?)
- Code review (see what a change might affect)

**Not recommended for:**
- Simple text searches (FTS alone is faster)
- High-volume automated queries (adds latency)
- Low-confidence results expected (expansion won't trigger)

### Interpreting Relevance Scores

The `relevance` score (0.0-1.0) is computed from:

| Factor | Effect | Example |
|--------|--------|---------|
| Depth | 0.7× per hop | depth=2 is ~0.49× of depth=1 |
| Same directory | 1.2× boost | Related chunks in same folder ranked higher |
| Test code | 0.5× penalty | Production code ranked above test code |
| Inheritance | 1.1× boost | extends/implements relationships ranked higher |

**Tip:** Filter by `relevance > 0.5` for most useful related chunks.

### Filtering by Relationship Type

Common relationship types and their uses:

| Type | Meaning | Use Case |
|------|---------|----------|
| `import` | This chunk imports the related chunk | Find dependencies |
| `call` | This chunk calls the related function | Find usage patterns |
| `extends` | This chunk extends the related class | Find subclasses |
| `implements` | This chunk implements the related interface | Find implementations |
| `direct` | Direct relationship (depth=1) | Immediate dependencies |
| `indirect` | Indirect relationship (depth=2) | Transitive dependencies |

## Troubleshooting

### No results have `related` field

**Cause:** All results are low-confidence (source_count < 2 and not exact match).

**Solutions:**
1. Try a more specific query (increases confidence)
2. Search for exact symbol names (triggers `is_exact_match`)
3. Verify results have `confidence` field (if missing, check `include_related` is true)

### Related chunks seem irrelevant

**Cause:** Related chunks may include test code or distant dependencies.

**Solutions:**
1. Filter by `relevance > 0.6` for higher-quality relationships
2. Filter by `relationship_type` to focus on specific relationships
3. Filter by `depth === 1` for direct relationships only
4. Check if related chunk is test code (often has "test" in relpath)

### Performance slower than expected

**Cause:** Graph traversal overhead on large codebases.

**Solutions:**
1. Normal overhead is <20ms (usually not noticeable)
2. Only 3 results max are expanded (hard cap)
3. Disable `include_related` for latency-critical queries
4. Use more specific queries (fewer results = less expansion)

### Empty `related` array (`[]`)

**Cause:** The chunk has no code relationships (isolated code).

**Context:** This is valid and expected for:
- Configuration files
- Standalone utility functions
- Entry points with no dependencies
- Self-contained modules

### `related` field missing (not empty)

**Cause:** Confidence threshold not met - expansion didn't run.

**Check:**
1. Result has `confidence` field
2. `confidence.source_count >= 2` OR `confidence.is_exact_match === true`
3. Result is within first 3 qualifying results (MAX_CONCURRENT_EXPANSIONS cap)

## Examples

### Example 1: Search with Relationships Enabled

**Request:**
```json
{
  "query": "user authentication",
  "repo": "crewchief",
  "include_related": true
}
```

**Response (truncated):**
```json
{
  "results": [
    {
      "chunk_id": 12345,
      "relpath": "src/auth/handler.ts",
      "symbol_name": "authenticateUser",
      "kind": "function",
      "score": 0.92,
      "confidence": {
        "source_count": 3,
        "score_gap": 0.15,
        "is_exact_match": false
      },
      "related": [
        {
          "chunk_id": 12346,
          "relpath": "src/auth/validator.ts",
          "symbol_name": "validateCredentials",
          "kind": "function",
          "depth": 1,
          "relevance": 0.84,
          "relationship_type": "import"
        },
        {
          "chunk_id": 12347,
          "relpath": "src/session/manager.ts",
          "symbol_name": "createSession",
          "kind": "function",
          "depth": 1,
          "relevance": 0.72,
          "relationship_type": "call"
        }
      ]
    }
  ]
}
```

### Example 2: Low-Confidence Result (No Expansion)

**Request:**
```json
{
  "query": "config",
  "repo": "crewchief",
  "include_related": true
}
```

**Response (truncated):**
```json
{
  "results": [
    {
      "chunk_id": 99999,
      "relpath": "src/config/settings.ts",
      "symbol_name": "defaultSettings",
      "kind": "const",
      "score": 0.45,
      "confidence": {
        "source_count": 1,
        "score_gap": 0.02,
        "is_exact_match": false
      }
      // Note: no "related" field (confidence too low)
    }
  ]
}
```

### Example 3: Isolated Chunk (Empty Related Array)

**Response showing isolated chunk:**
```json
{
  "chunk_id": 88888,
  "relpath": "src/utils/constants.ts",
  "symbol_name": "MAX_RETRY_COUNT",
  "kind": "const",
  "score": 0.88,
  "confidence": {
    "source_count": 2,
    "score_gap": 0.10,
    "is_exact_match": true
  },
  "related": []  // Expansion ran, but no relationships found
}
```

## Technical Details

For developers implementing features using relationship expansion, see:

- [Architecture Documentation](../architecture/relationship-clustering.md) - Internal design and implementation
- [Developer Guide](../development/relationship-expansion.md) - Contributing to relationship expansion
- [Performance Benchmarks](../architecture/performance/relationship-benchmarks.md) - Detailed performance analysis

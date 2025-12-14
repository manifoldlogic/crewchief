# Confidence Scoring

**Version**: 1.0
**Status**: Production
**Last Updated**: 2025-12-14

## Overview

Confidence scoring helps you assess the quality and reliability of search results by providing quantitative signals about each result. When enabled, search results include confidence metrics that indicate how likely a result is to be what you're looking for.

**Key Benefits**:
- **Make informed decisions** - Understand when to trust vs. investigate results
- **Identify high-quality matches** - Exact matches and multi-source results rank higher
- **Detect ambiguous searches** - Low confidence signals suggest refining your query
- **Opt-in design** - Zero overhead when disabled (default)

**When to Enable Confidence Scoring**:
- Evaluating search quality programmatically
- Building search result validation into workflows
- Investigating why certain results rank highly
- Tuning search queries for better precision

## Confidence Signals

Confidence scoring provides three core signals for each search result:

### 1. Source Count (`source_count`)

**What it measures**: Number of search sources (FTS, vector, graph) that independently returned this chunk.

**Range**: 1-4

**Interpretation**:
- **3-4 sources**: High confidence - Multiple search methods agree this is relevant
- **2 sources**: Moderate confidence - Result has support from two methods
- **1 source**: Lower confidence - Only one search method found this relevant

**Example**:
```typescript
{
  source_count: 3  // Appears in FTS, vector, and graph search
}
```

**Why it matters**: When multiple independent search methods return the same result, it's a strong signal that the result is genuinely relevant to your query, not just a false positive from one method.

### 2. Score Gap (`score_gap`)

**What it measures**: Numerical difference between this result's score and the next result's score.

**Range**: 0.0 to unlimited (typically 0.0-5.0)

**Interpretation**:
- **>1.0**: Large separation - This result is significantly better than alternatives
- **0.1-1.0**: Moderate separation - Result is somewhat better than next option
- **<0.1**: Small gap - Results are very close in quality (ambiguous)

**Example**:
```typescript
{
  score_gap: 2.5  // This result scored 2.5 points higher than next result
}
```

**Why it matters**: A large score gap indicates the search algorithm has high confidence this result is superior. Small gaps suggest multiple equally-good matches, which may mean you need a more specific query.

**Note**: The last result in a result set always has `score_gap: 0.0` since there's no result after it.

### 3. Exact Match (`is_exact_match`)

**What it measures**: Whether your query exactly matched the symbol name (function, class, variable, etc.).

**Range**: `true` or `false`

**Interpretation**:
- **`true`**: Query matches symbol name exactly (case-insensitive, normalized)
- **`false`**: Query is related but doesn't exactly match symbol name

**Example**:
```typescript
{
  is_exact_match: true  // Query "authenticate" matched function name "authenticate"
}
```

**Normalization Rules**:
- Case-insensitive: `Authenticate` → `authenticate`
- CamelCase → snake_case: `validateToken` → `validate_token`
- Spaces → underscores: `user auth` → `user_auth`

**Why it matters**: Exact matches are highly likely to be what you're searching for. If you search "authenticate", finding a function literally named `authenticate()` is a strong confidence signal.

## Interpreting Signals

### High Confidence Indicators

When you see these patterns, the result is very likely what you're looking for:

- **Multiple sources** (`source_count >= 3`)
- **Large score gap** (`score_gap > 1.0`)
- **Exact match** (`is_exact_match: true`)

**Combined interpretation**: When all three signals are positive, you can be highly confident the result is correct.

### Low Confidence Indicators

When you see these patterns, consider refining your query:

- **Single source** (`source_count == 1`)
- **Small score gap** (`score_gap < 0.1`)
- **No exact match** (`is_exact_match: false`)

**Combined interpretation**: When signals are weak, the search may be returning loosely-related results. Try a more specific query or different keywords.

### Mixed Signals

Real-world results often show mixed signals. Here's how to interpret them:

| Pattern | Interpretation | Action |
|---------|---------------|---------|
| High source count + exact match, but small gap | Multiple good matches exist (e.g., overloaded functions) | Review top 3-5 results |
| High source count + large gap, but no exact match | Conceptually strong match, not literal | Likely correct for concept searches |
| Exact match but single source + small gap | Found the symbol, but other similar symbols exist | Verify this is the specific instance you need |
| Large gap but single source + no exact match | One search method strongly prefers this result | Investigate why—may be false positive |

## Usage

### Enabling Confidence Scoring

Add the `include_confidence` parameter to your search request:

```typescript
import { search } from '@crewchief/maproom-mcp'

const results = await search({
  query: 'authenticate',
  repo: 'my-repo',
  include_confidence: true  // Enable confidence signals
})
```

### Accessing Confidence Signals

Each result includes a `confidence` field when enabled:

```typescript
results.hits.forEach(hit => {
  if (hit.confidence) {
    console.log(`Sources: ${hit.confidence.source_count}`)
    console.log(`Score gap: ${hit.confidence.score_gap.toFixed(2)}`)
    console.log(`Exact match: ${hit.confidence.is_exact_match}`)
  }
})
```

### Complete Example

```typescript
// Search with confidence enabled
const results = await search({
  query: 'validateToken',
  repo: 'auth-service',
  include_confidence: true
})

// Filter for high-confidence results
const highConfidence = results.hits.filter(hit => {
  const c = hit.confidence
  return c && c.source_count >= 2 && c.score_gap > 0.5
})

// Log confidence breakdown
highConfidence.forEach((hit, index) => {
  console.log(`\nResult ${index + 1}: ${hit.symbol_name}`)
  console.log(`  File: ${hit.file_path}`)
  console.log(`  Sources: ${hit.confidence.source_count}`)
  console.log(`  Gap: ${hit.confidence.score_gap.toFixed(2)}`)
  console.log(`  Exact: ${hit.confidence.is_exact_match}`)
})
```

## Example Scenarios

### Scenario 1: High Confidence - Exact Function Match

**Query**: `authenticate_user`

**Top Result**:
```typescript
{
  symbol_name: "authenticate_user",
  kind: "func",
  file_path: "src/auth/service.ts",
  score: 6.5,
  confidence: {
    source_count: 3,      // FTS, vector, and graph all found it
    score_gap: 2.8,       // Next result scored only 3.7
    is_exact_match: true  // Query matches function name exactly
  }
}
```

**Interpretation**: This is almost certainly the function you're looking for. All three confidence signals are strong:
- Multiple search methods agree (3 sources)
- Large separation from alternatives (gap: 2.8)
- Query matches function name exactly

**Recommendation**: Use this result with high confidence.

---

### Scenario 2: Low Confidence - Ambiguous Search

**Query**: `auth`

**Top Result**:
```typescript
{
  symbol_name: "AuthConfig",
  kind: "interface",
  file_path: "src/config/types.ts",
  score: 1.2,
  confidence: {
    source_count: 1,      // Only FTS found it
    score_gap: 0.05,      // Next result scored 1.15
    is_exact_match: false // "auth" doesn't exactly match "AuthConfig"
  }
}
```

**Interpretation**: Low confidence across all signals:
- Only one search method found this (single source)
- Very close to other results (tiny gap: 0.05)
- Not an exact symbol match

**Recommendation**: The query "auth" is too broad. Refine to something more specific like "AuthConfig", "authenticate", or "authorization" depending on what you're looking for.

---

### Scenario 3: Mixed Signals - Concept Match

**Query**: `user validation`

**Top Result**:
```typescript
{
  symbol_name: "validate_user_credentials",
  kind: "async_func",
  file_path: "src/auth/validators.ts",
  score: 4.2,
  confidence: {
    source_count: 2,      // FTS and vector found it
    score_gap: 1.5,       // Next result scored 2.7
    is_exact_match: false // "user validation" != "validate_user_credentials"
  }
}
```

**Interpretation**: Mixed but positive signals:
- Multiple sources (2) provide moderate support
- Large gap (1.5) shows this is significantly better than alternatives
- Not an exact match, but semantically related

**Recommendation**: This is likely the correct result for a concept-based search. The function name `validate_user_credentials` strongly relates to "user validation" even though the words don't match exactly. The large score gap and multi-source support indicate this is a strong semantic match.

---

### Scenario 4: Overloaded Function - Multiple Good Matches

**Query**: `connect`

**Top 3 Results**:
```typescript
// Result 1
{
  symbol_name: "connect",
  kind: "func",
  file_path: "src/db/connection.ts",
  score: 5.5,
  confidence: {
    source_count: 3,
    score_gap: 0.08,      // Very close to next result!
    is_exact_match: true
  }
}

// Result 2
{
  symbol_name: "connect",
  kind: "func",
  file_path: "src/websocket/client.ts",
  score: 5.42,
  confidence: {
    source_count: 3,
    score_gap: 0.15,
    is_exact_match: true
  }
}

// Result 3
{
  symbol_name: "connect",
  kind: "method",
  file_path: "src/redis/cache.ts",
  score: 5.27,
  confidence: {
    source_count: 2,
    score_gap: 1.1,
    is_exact_match: true
  }
}
```

**Interpretation**: Multiple exact matches with similar high scores (small gaps between top results):
- All have exact matches (same function name in different modules)
- High source counts show all are genuinely relevant
- Small gaps between top results indicate no single "best" match

**Recommendation**: Review the top 3-5 results to find the specific `connect` you need. The small gaps indicate these are all valid matches—context (file path) will help you choose the right one. Consider refining your query with more context (e.g., "database connect" or "websocket connect").

## Performance Characteristics

Confidence scoring is designed to be **lightweight and optional**:

### Performance Impact

- **Overhead**: <5ms per search (typically 1-3ms)
- **Scaling**: O(n) where n = number of results (negligible for typical result counts)
- **Database impact**: Zero additional queries (computed from existing data)

### Opt-In Design

```typescript
// Default: Confidence disabled, zero overhead
await search({ query: 'auth', repo: 'my-repo' })

// Opt-in: Enable when you need confidence signals
await search({ query: 'auth', repo: 'my-repo', include_confidence: true })
```

When `include_confidence` is `false` (default):
- No confidence computation occurs
- No confidence fields in response
- Zero performance overhead

## Backward Compatibility

Confidence scoring is **fully backward compatible**:

### Optional Parameter

The `include_confidence` parameter is **optional** and **defaults to `false`**:

```typescript
// These are equivalent:
await search({ query: 'auth', repo: 'my-repo' })
await search({ query: 'auth', repo: 'my-repo', include_confidence: false })
```

Existing code continues to work without modification.

### Response Structure

When confidence is disabled, response structure is unchanged:

```typescript
// Without confidence (default)
{
  hits: [
    {
      chunk_id: 123,
      symbol_name: "authenticate",
      score: 5.5
      // No confidence field
    }
  ]
}

// With confidence enabled
{
  hits: [
    {
      chunk_id: 123,
      symbol_name: "authenticate",
      score: 5.5,
      confidence: {              // Added only when enabled
        source_count: 3,
        score_gap: 2.5,
        is_exact_match: true
      }
    }
  ]
}
```

**No breaking changes**: Existing clients see the same response structure they always have.

## Related Documentation

- [Semantic Entry Point Ranking](./search-ranking.md) - How search results are ranked
- [Search Usage Patterns](./usage_patterns.md) - Common search patterns and best practices
- [Examples](./examples.md) - More search examples

## Troubleshooting

### Confidence fields are missing

**Problem**: Search returns results but no `confidence` field.

**Solution**: Ensure `include_confidence: true` is set in your search parameters.

```typescript
// Wrong - confidence disabled by default
await search({ query: 'auth', repo: 'my-repo' })

// Correct - explicitly enable confidence
await search({ query: 'auth', repo: 'my-repo', include_confidence: true })
```

### All results show low confidence

**Problem**: All results have `source_count: 1` and small `score_gap`.

**Possible causes**:
1. **Query too broad** - Try more specific keywords
2. **Limited indexed content** - Ensure your repository is fully indexed
3. **Genuinely ambiguous search** - Multiple equally-relevant results exist

**Solutions**:
- Add more specific terms to your query
- Use exact symbol names if known
- Review top 5-10 results instead of just the first
- Try different search modes (FTS vs vector vs hybrid)

### Unexpected exact matches

**Problem**: `is_exact_match: true` for results that don't seem to match your query.

**Explanation**: Query normalization converts different naming conventions:
- `validateToken` → `validate_token`
- `Authenticate` → `authenticate`
- `HTTP handler` → `http_handler`

**Solution**: This is expected behavior. Normalization helps match symbols across different naming conventions (camelCase, snake_case, etc.).

## Frequently Asked Questions

### Does confidence scoring affect search ranking?

No. Confidence signals are **computed after ranking** and do not influence which results are returned or their order. They only help you assess the quality of results that were already ranked by the search algorithm.

### Should I always enable confidence scoring?

Not necessarily. Enable it when:
- Building automated workflows that need to filter results
- Investigating search quality issues
- Programmatically validating search results
- Developing search-based tools

For interactive searches where you'll manually review results, the default behavior (confidence disabled) is fine.

### How do I interpret conflicting signals?

Use domain knowledge and file paths as additional context. For example:
- `is_exact_match: true` + low `source_count` → You found the right symbol name, but verify it's the right file/context
- High `source_count` + `is_exact_match: false` → Strong conceptual match even if not literal name match
- Large `score_gap` + low `source_count` → One search method strongly prefers this; investigate further

### What's a "good" source count?

Context-dependent:
- **Exact symbol search**: Expect 2-3 sources (FTS + vector at minimum)
- **Concept search**: 1-2 sources is normal (vector search often dominates)
- **Rare symbols**: 1 source is fine (may only appear in one context)

Don't use rigid thresholds—interpret source count alongside other signals and your search intent.

---

**Need Help?**
- [GitHub Issues](https://github.com/danielbushman/crewchief/issues)
- [Main Documentation](../README.md)

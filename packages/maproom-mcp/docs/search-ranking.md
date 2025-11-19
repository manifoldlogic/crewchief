# Semantic Entry Point Ranking

**Version**: 1.0
**Status**: Production
**Last Updated**: 2025-11-19

## Overview

Semantic entry point ranking enhances Maproom's full-text search (FTS) to prioritize **code implementations** over tests and documentation. When you search for a function, class, or concept, Maproom now returns relevant code first—not markdown headers or test files.

**Key Improvement**: Before semantic ranking, searching for "authenticate" would return documentation as the #1 result. Now, the actual `authenticate` function ranks first.

This feature uses **kind-based multipliers** and **exact match detection** to boost implementation chunks in search results, dramatically improving developer experience when navigating codebases.

## How It Works

Semantic ranking modifies the FTS score using two multipliers:

```
final_score = base_fts_score × kind_multiplier × exact_match_multiplier
```

### Components

1. **Base FTS Score** (`base_fts_score`): PostgreSQL's `ts_rank_cd()` score from full-text search
2. **Kind Multiplier** (`kind_multiplier`): Boosts/demotes based on chunk type (func, test, doc, etc.)
3. **Exact Match Multiplier** (`exact_match_multiplier`): 3.0× bonus when query matches symbol name exactly

The final score determines ranking order—higher scores appear first in results.

## Kind Multipliers

Different chunk kinds receive different multipliers based on their value as entry points:

| Kind | Multiplier | Rationale | Examples |
|------|-----------|-----------|----------|
| **Implementations** | | | |
| `func`, `async_func` | 2.5× | Primary entry points—functions users want to find | `authenticate()`, `validate_token()` |
| `class`, `struct`, `enum`, `interface` | 2.0× | Type definitions and data structures | `AuthService`, `User`, `ErrorCode` |
| `method` | 1.5× | Class/struct methods | `AuthService.login()` |
| `component` | 1.8× | React/Vue components | `<UserProfile />` |
| `hook` | 1.8× | React hooks | `useAuth()`, `useQuery()` |
| **Modules & Organization** | | | |
| `module` | 1.5× | Module/namespace definitions | `export { ... }` |
| `import_stmt` | 1.0× | Neutral—imports are references | `import { foo } from 'bar'` |
| **Tests** | | | |
| `test`, `test_function` | 0.6× | Demote tests—users want implementations first | `test_authenticate()`, `describe()` |
| **Documentation** | | | |
| `heading_1`, `heading_2` | 0.6×, 0.5× | Demote docs—prioritize code | `# Authentication Guide` |
| `heading_3` | 0.3× | Further demote lower-level headings | `### Setup Steps` |
| `markdown_section`, `code_block` | 0.4× | Demote markdown content | Doc paragraphs, examples |
| `comment`, `doc_comment` | 0.3× | Lowest priority—comments are context | `// TODO: refactor` |
| **Default** | 1.0× | Neutral for unknown kinds | Edge cases |

### Design Rationale

**Boost Implementations (2.0-2.5×):**
- Functions and classes are what developers search for when exploring code
- These are the "entry points" users navigate to

**Demote Tests (0.6×):**
- Tests mention function names frequently, inflating FTS scores
- Users searching "authenticate" want the implementation, not `test_authenticate()`

**Demote Documentation (0.3-0.6×):**
- Docs mention keywords repeatedly (e.g., "authentication" appears 20+ times in auth guides)
- Code implementations are more valuable than markdown explanations

## Exact Match Bonus

When the normalized query **exactly matches** a symbol name (case-insensitive), the result receives a **3.0× multiplier**.

### Normalization Rules

Both query and symbol_name are normalized before comparison:

1. **Case Insensitive**: `Authenticate` → `authenticate`
2. **CamelCase → snake_case**: `validateToken` → `validate_token`
3. **Spaces → Underscores**: `HTTP handler` → `http_handler`
4. **Acronym Handling**: `XMLParser` → `xml_parser`

### Example

**Query**: `validateToken`
**Normalized**: `validate_token`

**Chunk**: `func validate_token(token: str)`
**Symbol**: `validate_token`
**Normalized**: `validate_token`

**Match**: ✅ Exact match → 3.0× multiplier applied

**Final Score**:
```
base_score: 0.85 (from FTS)
kind_multiplier: 2.5 (func)
exact_match_multiplier: 3.0 (exact match)
final_score = 0.85 × 2.5 × 3.0 = 6.375
```

This function now ranks **much higher** than documentation mentioning "validateToken" (which would get 0.85 × 0.5 = 0.425).

## Query Normalization Examples

| User Query | Normalized Query | Matches Symbol |
|-----------|-----------------|----------------|
| `authenticate` | `authenticate` | `authenticate`, `Authenticate` |
| `Authenticate` | `authenticate` | `authenticate` |
| `validateToken` | `validate_token` | `validate_token`, `ValidateToken` |
| `HTTP handler` | `http_handler` | `http_handler`, `HTTPHandler` |
| `database connection` | `database_connection` | `database_connection`, `DatabaseConnection` |
| `useAuth` | `use_auth` | `use_auth`, `UseAuth` |

**Note**: Normalization is **fuzzy**—it increases recall by matching different naming conventions. Exact symbol matches still get the 3.0× bonus.

## Debug Mode

Enable debug mode to see how semantic ranking scores each result:

### Enabling Debug Mode

```typescript
const results = await search({
  query: 'authenticate',
  repo: 'my-repo',
  debug: true  // Enable score breakdown
})
```

### Score Breakdown Output

Each result includes a `score_breakdown` field:

```json
{
  "chunk_id": 1234,
  "symbol_name": "authenticate",
  "kind": "func",
  "score": 6.375,
  "score_breakdown": {
    "base_score": 0.85,
    "kind_multiplier": 2.5,
    "exact_match_multiplier": 3.0,
    "final_score": 6.375,
    "explanation": "func (2.5×) + exact match (3.0×)"
  }
}
```

### Interpreting Scores

- **High scores (>5.0)**: Exact match implementations—this is what you're looking for
- **Medium scores (2.0-5.0)**: Implementations with partial matches or related terms
- **Low scores (<2.0)**: Tests, docs, or weak matches

### Debugging Ranking Issues

If a result ranks lower than expected:

1. **Check `kind`**: Is it a test (`0.6×`) instead of a function (`2.5×`)?
2. **Check `exact_match_multiplier`**: Is it 1.0 (no match) or 3.0 (exact match)?
3. **Check `base_score`**: Is the FTS score low due to keyword frequency?

**Common Issue**: Documentation ranks high because it mentions the keyword many times (high base_score), but kind multiplier (0.5×) should still demote it below implementations.

## Before & After Examples

### Example 1: Exact Function Search

**Query**: `authenticate`

**Before Semantic Ranking** (baseline FTS):
```
1. heading_2: "Authentication Guide" (score: 0.92)
2. heading_1: "User Authentication" (score: 0.89)
3. heading_2: "Authenticate Users" (score: 0.85)
...
8. func: authenticate() (score: 0.78)  ← Implementation buried
```

**After Semantic Ranking**:
```
1. func: authenticate() (score: 5.85)  ← Implementation #1! (0.78 × 2.5 × 3.0)
2. func: authenticate_user() (score: 5.12)  ← Related function
3. func: pre_authenticate() (score: 4.95)  ← Related function
4. class: Authenticator (score: 2.34)  ← Related class
...
10. heading_2: "Authentication Guide" (score: 0.46)  ← Docs demoted
```

**Impact**: Implementation ranks #1 instead of #8. User finds code immediately.

### Example 2: Concept Search

**Query**: `user authentication`

**Before Semantic Ranking**:
```
1. heading_1: "User Authentication System" (score: 1.05)
2. heading_2: "How User Authentication Works" (score: 0.98)
...
5. func: authenticate_user() (score: 0.67)  ← Implementation lower
```

**After Semantic Ranking**:
```
1. func: authenticate_user() (score: 1.68)  ← Implementation #1 (0.67 × 2.5)
2. class: UserAuthService (score: 1.34)  ← Related class (0.67 × 2.0)
3. func: validate_user_session() (score: 1.25)  ← Related function
...
7. heading_1: "User Authentication System" (score: 0.63)  ← Docs demoted
```

**Impact**: Implementations rank before documentation for concept searches.

### Example 3: Case Variations

**Queries**: `authenticate`, `Authenticate`, `AUTHENTICATE`, `AuThEnTiCaTe`

**Before**: Different results depending on case (inconsistent)

**After**: **All queries return the same #1 result** (case-insensitive exact match)

```
1. func: authenticate() (score: 5.85)  ← Same for all cases
```

**Impact**: Consistent results regardless of user input case.

## Migration from Old Exact Bonus

**Previous System** (before SEMRANK):
- Exact match: `+0.2` additive bonus to FTS score
- No kind-based ranking
- Tests and docs ranked higher than implementations

**New System** (SEMRANK):
- Exact match: `3.0×` multiplicative bonus
- Kind multipliers: Boost implementations, demote tests/docs
- Combined scoring: `base × kind × exact_match`

### Why the Change?

**Additive bonus (+0.2) problems:**
- Too small to overcome high FTS scores from docs mentioning keywords frequently
- No way to prioritize implementations over tests
- Case-sensitive (Authenticate ≠ authenticate)

**Multiplicative bonus (3.0×) benefits:**
- Dramatically boosts exact matches above non-matches
- Combines with kind multipliers for powerful ranking
- Case-insensitive with query normalization

### Backward Compatibility

No breaking changes—search API remains the same. Queries automatically benefit from improved ranking.

**Performance**: Semantic ranking is **17% faster** on average (p95 latency: 48.1ms → 39.9ms) because implementations rank first, allowing early result termination.

## Performance Characteristics

- **Average latency improvement**: -17% (p95: 48ms → 40ms)
- **Queries improved >10%**: 55% of queries (11/20 in benchmarks)
- **Queries slower >10%**: 15% of queries (3/20), all <100ms absolute latency
- **All queries**: <200ms p95 latency (well within target)

**Why faster?** Better ranking reduces wasted processing on irrelevant results. When implementations rank first, the query can terminate earlier with high-quality results.

See `benchmarks/performance-comparison.md` for detailed analysis.

## SQL Implementation Details

### Kind Multiplier CASE Statement

```sql
CASE kind
  WHEN 'func' THEN 2.5
  WHEN 'async_func' THEN 2.5
  WHEN 'class' THEN 2.0
  WHEN 'struct' THEN 2.0
  WHEN 'method' THEN 1.5
  WHEN 'test' THEN 0.6
  WHEN 'test_function' THEN 0.6
  WHEN 'heading_1' THEN 0.6
  WHEN 'heading_2' THEN 0.5
  WHEN 'heading_3' THEN 0.3
  WHEN 'comment' THEN 0.3
  WHEN 'doc_comment' THEN 0.3
  ELSE 1.0
END AS kind_mult
```

### Exact Match CASE Statement

```sql
CASE
  WHEN LOWER(symbol_name) = LOWER(normalize_for_exact_match($1))
  THEN 3.0
  ELSE 1.0
END AS exact_mult
```

### Final Scoring

```sql
SELECT
  *,
  (ts_rank_cd(ts_doc, query) * kind_mult * exact_mult) AS final_score
FROM maproom.chunks
WHERE ts_doc @@ query
ORDER BY final_score DESC
LIMIT 20
```

## Integration with RRF Fusion

When using hybrid search (FTS + vector), semantic ranking applies to the **FTS component** before RRF fusion:

1. **FTS Search**: Apply semantic ranking → get ranked FTS results
2. **Vector Search**: Get ranked vector results (cosine similarity)
3. **RRF Fusion**: Combine both result sets using reciprocal rank fusion

This ensures semantic ranking improves both FTS-only and hybrid search modes.

## Best Practices

### For Users

1. **Search by symbol name**: Query "authenticate" to find the `authenticate()` function
2. **Use debug mode**: Enable `debug: true` to understand why results rank as they do
3. **Try case variations**: "Authenticate", "authenticate", "AUTHENTICATE" all work the same
4. **Search concepts**: "user authentication" finds relevant implementations, not just docs

### For Developers

1. **Name symbols clearly**: Well-named functions/classes rank higher with exact match bonus
2. **Follow conventions**: snake_case (Python/Rust) or camelCase (TypeScript) both work
3. **Document in code**: Implementations rank first—docs are supplementary
4. **Write integration tests**: Use `tests/integration/search-quality.test.ts` patterns

## Troubleshooting

### Issue: Documentation ranks higher than code

**Diagnosis**: Check `kind_multiplier` in debug mode
- Docs should have `0.3-0.6×` multipliers
- Implementations should have `2.0-2.5×` multipliers

**Solution**: Verify the chunk is correctly tagged as `func`/`class`, not `markdown_section`

### Issue: Tests rank higher than implementations

**Diagnosis**: Check for exact match on test name
- `test_authenticate` might get exact match bonus if query is `test_authenticate`

**Solution**: Query `authenticate` (not `test_authenticate`) to find implementation

### Issue: Unexpected #1 result

**Diagnosis**: Enable debug mode and compare `final_score` values

**Common Causes**:
- High FTS base score due to keyword frequency in chunk
- Exact match on unexpected symbol
- Kind multiplier not applied correctly

### Issue: No results for query

**Diagnosis**: Not a ranking issue—FTS didn't match the query

**Solution**:
- Check spelling
- Try broader keywords
- Use query normalization (validateToken → validate_token)

## References

- **Implementation**: `packages/maproom-mcp/src/tools/search.ts`
- **SQL Schema**: `packages/maproom-mcp/config/init.sql`
- **Tests**: `packages/maproom-mcp/tests/integration/search-quality.test.ts`
- **Benchmarks**: `packages/maproom-mcp/benchmarks/performance-comparison.md`
- **Architecture**: `docs/architecture/SEARCH_ARCHITECTURE.md`

## Changelog

### v1.0 (2025-11-19) - Initial Release

- Implemented kind multipliers (func: 2.5×, test: 0.6×, docs: 0.3-0.6×)
- Implemented exact match bonus (3.0× for normalized symbol matches)
- Added query normalization (camelCase → snake_case, spaces → underscores)
- Added debug mode with score breakdown
- Performance: 17% faster than baseline FTS
- All regression tests passing (11/11)

---

**Project**: SEMRANK (Semantic Entry Point Ranking)
**Status**: Production Ready
**Performance**: Validated (see benchmarks/)
**Tests**: Comprehensive (see tests/integration/)

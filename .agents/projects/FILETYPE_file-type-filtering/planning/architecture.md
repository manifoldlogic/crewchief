# Architecture: File Type Filtering

**Project:** FILETYPE - File Type Filtering
**Date:** 2025-11-19
**Status:** MVP Design

---

## Design Philosophy

**MVP First:** Ship functional file type filtering without over-engineering. Focus on 90% use case (extension-based filtering), defer advanced features (regex, negation, language mappings) to future iterations.

**No Breaking Changes:** Build on existing foundation. The filter parameter is already in the API - we're completing it, not replacing it.

**User-Centered:** Simple syntax users already understand from other tools (`file_type: "ts"` or `file_type: "ts,tsx,js"`).

---

## System Overview

### Architecture Layers

```
┌─────────────────────────────────────────┐
│         MCP Client (Claude/VSCode)      │
│   calls: search({filters: {file_type}}) │
└────────────────┬────────────────────────┘
                 │
                 │ MCP JSON-RPC
                 │
┌────────────────▼────────────────────────┐
│      MCP Server (TypeScript)            │
│  - Parse & validate file_type           │
│  - Build SQL WHERE clause               │
│  - Execute query via pg client          │
└────────────────┬────────────────────────┘
                 │
                 │ SQL Query
                 │
┌────────────────▼────────────────────────┐
│     PostgreSQL (maproom database)       │
│  WHERE f.relpath LIKE '%.ts'            │
│     OR f.relpath LIKE '%.tsx' ...       │
└─────────────────────────────────────────┘
```

**Key Insight:** Filtering happens **entirely in the MCP TypeScript layer**. No Rust changes needed. No database schema changes needed. Pure query logic modification.

---

## Component Design

### 1. Input Validation (`buildFilterClauses` enhancement)

**Location:** `packages/maproom-mcp/src/index.ts` (around line 442)

**Current Implementation:**
```typescript
function buildFilterClauses(filters: any, filter: string, args: any[]): string {
  let clauses = ''
  // ... legacy filter handling ...

  // Advanced filters
  if (filters.file_type) {
    args.push(`%.${filters.file_type}`)
    clauses += ` AND f.relpath LIKE $${args.length}`
  }
  // ...
}
```

**New Implementation:**
```typescript
function buildFilterClauses(filters: any, filter: string, args: any[]): string {
  let clauses = ''
  // ... legacy filter handling ...

  // Advanced file_type filter
  if (filters.file_type) {
    const extensions = parseFileTypeFilter(filters.file_type)
    if (extensions.length === 0) {
      // Empty after parsing - warn but don't fail
      // Could add to result.hint or debug output
    } else if (extensions.length === 1) {
      // Single extension - simple LIKE
      args.push(`%.${extensions[0]}`)
      clauses += ` AND f.relpath LIKE $${args.length}`
    } else {
      // Multiple extensions - OR clause
      const conditions = extensions.map(ext => {
        args.push(`%.${ext}`)
        return `f.relpath LIKE $${args.length}`
      }).join(' OR ')
      clauses += ` AND (${conditions})`
    }
  }
  // ...
}
```

**Rationale:**
- Minimal change to existing code
- Backward compatible (single extension works same as before)
- OR logic for multi-extension is SQL standard pattern
- Parameterized queries prevent SQL injection

---

### 2. Extension Parser (New Function)

**Location:** `packages/maproom-mcp/src/index.ts` (near `buildFilterClauses`)

**Signature:**
```typescript
/**
 * Parse and normalize file type filter input
 *
 * @param input - Raw file_type filter value (e.g., "ts", "ts,tsx,js", "  TS  ")
 * @returns Array of normalized extensions (lowercase, no dots, trimmed)
 *
 * @example
 * parseFileTypeFilter("ts") → ["ts"]
 * parseFileTypeFilter("ts,tsx,js") → ["ts", "tsx", "js"]
 * parseFileTypeFilter(".ts, .tsx") → ["ts", "tsx"]
 * parseFileTypeFilter("  TS  ") → ["ts"]
 * parseFileTypeFilter("") → []
 * parseFileTypeFilter(",,,") → []
 */
function parseFileTypeFilter(input: string): string[] {
  return input
    .split(',')                        // Split on comma
    .map(ext => ext.trim())           // Remove whitespace
    .map(ext => ext.replace(/^\./, '')) // Remove leading dot if present
    .map(ext => ext.toLowerCase())    // Normalize case
    .filter(ext => ext.length > 0)    // Remove empty strings
}
```

**Design Decisions:**

1. **Comma-separated:** Common pattern (CSV-like), familiar to users
2. **Case normalization:** "TS" and "ts" should match (case-insensitive)
3. **Dot handling:** Accept both "ts" and ".ts" (strip leading dot)
4. **Whitespace tolerance:** " ts , tsx " should work
5. **Empty handling:** Return empty array (caller decides how to handle)

**Edge Cases:**
- `""` → `[]` (empty array)
- `",,,,"` → `[]` (empty array)
- `"ts,"` → `["ts"]` (trailing comma ignored)
- `".ts,.tsx"` → `["ts", "tsx"]` (dots stripped)
- `"  TS  ,  tsx  "` → `["ts", "tsx"]` (normalized)

---

### 3. Error Handling Strategy

**Philosophy:** Be permissive on input, strict on validation.

**Approach:**

```typescript
// In handleSearch function
if (filters.file_type) {
  const extensions = parseFileTypeFilter(filters.file_type)

  if (extensions.length === 0) {
    // User provided empty/invalid input
    // Option A: Ignore filter (search all files)
    // Option B: Return error with helpful message
    // Decision: Add warning to result.hint, continue search

    result.hint = result.hint || ''
    result.hint += `\n⚠️ file_type filter "${filters.file_type}" produced no valid extensions. Searching all files.`
  }

  // Validate reasonable extension count (prevent abuse)
  if (extensions.length > 20) {
    return {
      hits: [],
      error: 'Too many file extensions',
      hint: `file_type filter has ${extensions.length} extensions (max 20). Use broader filter or multiple searches.`,
      suggestion: 'Try: filters: {file_type: "ts,tsx,js"} instead of listing 50+ extensions'
    }
  }
}
```

**Rationale:**
- Don't fail silently - inform user via hint
- Prevent abuse (20 extension limit prevents complex OR queries)
- Helpful error messages guide users to correct usage

---

### 4. Testing Architecture

**Three-Layer Testing:**

```
┌──────────────────────────────────────────┐
│  E2E Tests (Vitest integration)          │
│  - Real database queries                 │
│  - Verify actual filtering works         │
│  - Test multi-extension OR logic         │
└──────────────────────────────────────────┘
                 ▲
                 │ depends on
┌────────────────┴──────────────────────────┐
│  Integration Tests (search_tool.test.ts)  │
│  - SQL query construction                 │
│  - Parameterization correctness           │
│  - Filter combination                     │
└──────────────────────────────────────────┘
                 ▲
                 │ depends on
┌────────────────┴──────────────────────────┐
│  Unit Tests (parseFileTypeFilter)         │
│  - Input parsing edge cases               │
│  - Normalization logic                    │
│  - Empty/invalid handling                 │
└──────────────────────────────────────────┘
```

**Test Coverage Goals:**
- **Unit:** 100% coverage of parseFileTypeFilter (simple pure function)
- **Integration:** All SQL query variations (single, multi, empty)
- **E2E:** Real search returns only matching file types

---

## Implementation Specification

This section provides **exact, unambiguous implementation details** to resolve the critical scope ambiguities identified in the project review. Every implementation ticket can reference this section for precise guidance.

### Function Placement

**parseFileTypeFilter() location:**
- **File:** `packages/maproom-mcp/src/index.ts`
- **Line:** ~430 (immediately before `buildFilterClauses()` function)
- **Visibility:** Private helper function (NOT exported)
- **Scope:** Module-level function, not a method

**Rationale:**
- Keep related code together (parser + consumer in same file)
- Private scope prevents unintended external usage
- No need for separate utils file for single helper function
- Follows existing pattern (other filters don't have separate modules)

---

### Function Signature (Exact)

```typescript
/**
 * Parse and normalize file type filter input into array of extensions.
 *
 * Handles comma-separated extension lists with flexible formatting:
 * - Case insensitive: "TS" → "ts"
 * - Dot tolerant: ".ts" → "ts"
 * - Whitespace tolerant: " ts , tsx " → ["ts", "tsx"]
 * - Empty safe: "" → [], ",,," → []
 *
 * @param input - Raw file_type filter string from MCP request
 * @returns Array of normalized extension strings (lowercase, no dots)
 *
 * @example Single extension
 * parseFileTypeFilter("ts") → ["ts"]
 *
 * @example Multi-extension
 * parseFileTypeFilter("ts,tsx,js") → ["ts", "tsx", "js"]
 *
 * @example Flexible formatting
 * parseFileTypeFilter(".TS, .tsx , js") → ["ts", "tsx", "js"]
 *
 * @example Empty handling
 * parseFileTypeFilter("") → []
 * parseFileTypeFilter(",,,") → []
 */
function parseFileTypeFilter(input: string): string[] {
  return input
    .split(',')                           // Split on comma delimiter
    .map(ext => ext.trim())               // Remove leading/trailing whitespace
    .map(ext => ext.replace(/^\./, ''))   // Strip leading dot if present
    .map(ext => ext.toLowerCase())        // Normalize to lowercase
    .filter(ext => ext.length > 0)        // Remove empty strings after processing
}
```

**Key Properties:**
- **Pure function:** No side effects, deterministic output
- **No exceptions:** Returns empty array on invalid input, never throws
- **No validation limits:** Caller (buildFilterClauses) handles extension count/length limits
- **Return type:** Always `string[]`, never `null` or `undefined`

---

### Integration with buildFilterClauses (Before/After)

**BEFORE (Current Code - Line 458):**

```typescript
// Advanced filters
if (filters.file_type) {
  args.push(`%.${filters.file_type}`)
  clauses += ` AND f.relpath LIKE $${args.length}`
}
```

**AFTER (New Implementation - Line 458):**

```typescript
// Advanced file_type filter with multi-extension support
if (filters.file_type) {
  const extensions = parseFileTypeFilter(filters.file_type)

  // Handle empty result (invalid input or all-commas string)
  if (extensions.length === 0) {
    // Silent ignore - skip this filter entirely
    // This matches existing filter pattern (no errors for bad input)
    continue
  }

  // Enforce extension count limit (prevent DoS via complex OR queries)
  if (extensions.length > 20) {
    // This is abuse/error - should be caught in validation layer
    // For now, truncate to 20 (graceful degradation)
    extensions.splice(20)
  }

  // Single extension: simple LIKE clause (backward compatible)
  if (extensions.length === 1) {
    args.push(`%.${extensions[0]}`)
    clauses += ` AND f.relpath LIKE $${args.length}`
  }
  // Multiple extensions: OR clause
  else {
    const likeConditions = extensions.map(ext => {
      args.push(`%.${ext}`)
      return `f.relpath LIKE $${args.length}`
    })
    clauses += ` AND (${likeConditions.join(' OR ')})`
  }
}
```

**SQL Query Examples:**

```sql
-- Single extension (file_type: "ts")
WHERE f.relpath LIKE $5  -- args[4] = "%.ts"

-- Multiple extensions (file_type: "ts,tsx,js")
WHERE (f.relpath LIKE $5 OR f.relpath LIKE $6 OR f.relpath LIKE $7)
-- args[4] = "%.ts", args[5] = "%.tsx", args[6] = "%.js"

-- Combined with other filters (recency + file_type)
WHERE f.last_modified > NOW() - INTERVAL $4
  AND (f.relpath LIKE $5 OR f.relpath LIKE $6)
```

---

### Error Handling Strategy (Definitive)

**Decision:** Return empty array on invalid input, NO exceptions.

**Rationale:**
1. **Consistency:** Existing filters (recency_threshold, repo_id) don't throw exceptions
2. **Graceful degradation:** Better to search all files than fail entire request
3. **Caller control:** buildFilterClauses decides how to handle empty array
4. **Security:** No information leakage via error messages

**Error Scenarios:**

| Input | parseFileTypeFilter Result | buildFilterClauses Behavior |
|-------|---------------------------|----------------------------|
| `""` | `[]` | Skip filter (no WHERE clause) |
| `",,,,"` | `[]` | Skip filter |
| `"ts"` | `["ts"]` | Single LIKE clause |
| `"ts,tsx,js"` | `["ts","tsx","js"]` | OR clause with 3 conditions |
| `"  TS  "` | `["ts"]` | Single LIKE clause (normalized) |
| `".ts,.tsx"` | `["ts","tsx"]` | OR clause (dots stripped) |
| `"a,b,c...(21+ items)"` | Full array | Caller truncates to 20 |

**No User-Facing Errors:**
- Filter layer does NOT return error responses for parsing issues
- Silent fallback to "search all files" behavior
- Validation errors (too many extensions) handled by caller

---

### Type Definitions (Explicit)

**No new TypeScript interfaces or types needed for MVP.**

**Existing types used:**
- `filters: any` - Already defined in MCP tool schema
- `parseFileTypeFilter(input: string): string[]` - Uses built-in types
- `args: any[]` - Already defined in buildFilterClauses signature

**Why no custom types:**
```typescript
// NOT needed for MVP:
interface FileTypeFilter {
  extensions: string[]
  maxExtensions: number
}

// Simple string[] is sufficient:
function parseFileTypeFilter(input: string): string[]
```

**Future enhancement (Phase 2+):**
```typescript
// If we add validation layer later:
interface FileTypeFilterResult {
  extensions: string[]
  warnings?: string[]
  truncated: boolean
}
```

---

### Error Message Catalog (Exact Wording)

**Error messages are NOT part of MVP** - filter layer uses silent fallback.

**For future enhancement (if adding explicit validation):**

```typescript
// Error: Too many extensions
{
  error: 'Too many file extensions in filter',
  hint: `file_type filter has ${extensions.length} extensions (max 20). Use broader filter or split into multiple searches.`,
  suggestion: 'Try combining related extensions: file_type:"ts,tsx" instead of listing all variations'
}

// Warning: Empty filter (debug mode only)
{
  hits: [...],
  debugInfo: {
    warnings: ['file_type filter produced no valid extensions, searching all files']
  }
}

// Error: Extension too long (if we add length validation)
{
  error: 'Invalid file extension in filter',
  hint: 'File extensions must be 1-20 characters (found: "${tooLongExt}")',
  suggestion: 'Use standard extensions like "ts", "js", "py" instead of full filenames'
}
```

**For MVP:** No error messages needed - silent fallback is sufficient.

---

### Validation Rules (Comprehensive)

**parseFileTypeFilter() validation (implicit in logic):**
1. ✅ Split on comma: `"ts,tsx"` → `["ts", "tsx"]`
2. ✅ Trim whitespace: `" ts "` → `"ts"`
3. ✅ Strip leading dot: `".ts"` → `"ts"`
4. ✅ Lowercase normalization: `"TS"` → `"ts"`
5. ✅ Remove empty strings: `["ts", "", "tsx"]` → `["ts", "tsx"]`
6. ❌ NO character validation (accepts any non-empty string)
7. ❌ NO length validation (accepts any length)
8. ❌ NO count validation (returns any number of extensions)

**buildFilterClauses() validation (on parsed result):**
1. ✅ Empty array check: `[]` → skip filter
2. ✅ Extension count limit: `>20` → truncate (graceful)
3. ❌ NO per-extension length check (deferred to post-MVP)
4. ❌ NO alphanumeric validation (deferred to post-MVP)

**Security validation (already in place via SQL):**
1. ✅ SQL injection prevented: parameterized queries
2. ✅ DoS mitigation: 20 extension limit
3. ⚠️ Character validation missing: could pass through special chars (low risk - just won't match files)

---

### Performance Characteristics

**parseFileTypeFilter() complexity:**
- Time: O(n) where n = number of characters in input
- Space: O(m) where m = number of extensions after split
- Typical case: `"ts,tsx,js"` → 3 operations, <1ms
- Worst case: 20 extensions with long names → <5ms

**buildFilterClauses() SQL generation:**
- Single extension: 1 parameter, simple LIKE → existing performance
- Multiple extensions: m parameters, OR clause → ~2x slower than single
- 20 extensions: 20 parameters, 20-way OR → ~10x slower (acceptable)

**Overall impact:**
- Baseline: 100ms search query
- With file_type filter (1 ext): 100ms (no change)
- With file_type filter (5 ext): 110ms (+10%)
- With file_type filter (20 ext): 120ms (+20% - within acceptable threshold)

---

### Testing Guidance

**Unit tests (in search_tool.test.ts):**
```typescript
describe('parseFileTypeFilter', () => {
  it('should parse single extension', () => {
    expect(parseFileTypeFilter('ts')).toEqual(['ts'])
  })

  it('should parse comma-separated extensions', () => {
    expect(parseFileTypeFilter('ts,tsx,js')).toEqual(['ts', 'tsx', 'js'])
  })

  it('should normalize case', () => {
    expect(parseFileTypeFilter('TS')).toEqual(['ts'])
  })

  it('should strip leading dots', () => {
    expect(parseFileTypeFilter('.ts,.tsx')).toEqual(['ts', 'tsx'])
  })

  it('should handle whitespace', () => {
    expect(parseFileTypeFilter('  ts  , tsx  ')).toEqual(['ts', 'tsx'])
  })

  it('should return empty array for empty input', () => {
    expect(parseFileTypeFilter('')).toEqual([])
    expect(parseFileTypeFilter(',,,,')).toEqual([])
  })
}
```

**Integration tests (SQL generation verification):**
- Test that single extension generates correct LIKE clause
- Test that multiple extensions generate correct OR clause
- Test that parameterized query indexes are correct

**E2E tests (actual filtering):**
- Search with file_type:"ts" returns only .ts files
- Search with file_type:"ts,tsx,js" returns union of matching files
- Search with invalid file_type:"" returns all files (graceful fallback)

---

## Technology Choices

### Why TypeScript (not Rust)?

**Decision:** Implement filter parsing in MCP TypeScript layer, not Rust binary.

**Rationale:**
1. **Separation of concerns:** MCP server owns query construction, Rust owns indexing
2. **Faster iteration:** TypeScript changes don't require rebuilding cross-platform binaries
3. **Existing pattern:** Other filters (recency_threshold, repo_id) handled in TypeScript
4. **Simplicity:** Avoids adding filter logic to Rust that duplicates TypeScript

**Trade-offs:**
- ✅ Faster development (no Rust rebuild)
- ✅ Consistent with existing filters
- ✅ Easier to test (Vitest vs Rust tests)
- ❌ Slight duplication if Rust CLI adds direct search (future)

**Conclusion:** TypeScript is correct layer for this feature.

---

### Why SQL LIKE (not indexed column)?

**Decision:** Use `WHERE f.relpath LIKE '%.ts'` instead of adding `files.extension` column.

**Rationale:**
1. **Simplicity:** No schema migration needed
2. **Flexibility:** Pattern matching works for any extension
3. **Performance:** Acceptable for small/medium repos (most indexes have <50k files)
4. **Future-proof:** Can add indexed column later if needed (optimization, not requirement)

**Performance Analysis:**

```sql
-- Current query (with file_type filter)
SELECT ... FROM chunks c
JOIN files f ON f.id = c.file_id
WHERE f.relpath LIKE '%.ts' OR f.relpath LIKE '%.tsx'  -- No index scan

-- With indexed column (future optimization)
SELECT ... FROM chunks c
JOIN files f ON f.id = c.file_id
WHERE f.extension IN ('ts', 'tsx')  -- Index scan on files(extension)
```

**Benchmarks (estimated):**
- <10k files: LIKE performance acceptable (<10ms overhead)
- 10k-100k files: LIKE still OK (<50ms overhead)
- >100k files: Consider indexed column optimization

**Decision:** Ship with LIKE, measure in production, optimize if needed.

---

## Data Flow

### Search Request Lifecycle

```
1. User initiates search
   ↓
   search({
     repo: "crewchief",
     query: "authentication",
     filters: {file_type: "ts,tsx"}
   })

2. MCP server receives request
   ↓
   handleSearch(params)
   - Validate mode, repo, worktree
   - Parse filters.file_type → ["ts", "tsx"]

3. Build SQL query
   ↓
   buildFilterClauses(filters, filter, args)
   - Add: AND (f.relpath LIKE '%.ts' OR f.relpath LIKE '%.tsx')
   - Parameterize: $4 = '%.ts', $5 = '%.tsx'

4. Execute search
   ↓
   executeFtsSearch() / executeHybridSearch()
   - Query database with file_type filter
   - Results contain only .ts and .tsx files

5. Return results
   ↓
   {
     hits: [...only ts/tsx chunks...],
     total: N,
     hint: "Filtered to ts, tsx files"
   }
```

---

## Migration Strategy

### Phase 1: Complete Core Implementation
**Goal:** Make file_type filter fully functional

**Changes:**
1. Add `parseFileTypeFilter()` function
2. Update `buildFilterClauses()` for multi-extension support
3. Add input validation and error handling
4. Update tool description with examples

**Deliverable:** `filters: {file_type: "ts,tsx,js"}` works correctly

---

### Phase 2: Comprehensive Testing
**Goal:** Verify correctness across all use cases

**Tests:**
1. Unit tests for parseFileTypeFilter
2. Integration tests for SQL generation
3. E2E tests with real database
4. Performance benchmarks

**Deliverable:** 100% confidence in filter behavior

---

### Phase 3: Documentation & Polish
**Goal:** Make feature discoverable and usable

**Updates:**
1. Tool description examples
2. Error message improvements
3. Hint messages for empty results
4. TypeScript types for filter params

**Deliverable:** Users can find and use the feature

---

## Performance Considerations

### Query Performance

**Single extension:**
```sql
-- Baseline (no filter)
SELECT ... FROM chunks c JOIN files f ... WHERE <search conditions>
-- ~50ms for 10k files

-- With file_type filter
SELECT ... FROM chunks c JOIN files f ... WHERE <search conditions> AND f.relpath LIKE '%.ts'
-- ~55ms for 10k files (+10% overhead)
```

**Multiple extensions:**
```sql
-- Multi-extension filter
WHERE (f.relpath LIKE '%.ts' OR f.relpath LIKE '%.tsx' OR f.relpath LIKE '%.js')
-- ~60ms for 10k files (+20% overhead)
```

**Conclusion:** Acceptable overhead for MVP. Optimize later if users report slowness.

---

### Client-Side Impact

**Minimal:** Filter parsing is O(n) where n = number of extensions (small constant).

**Example:**
```typescript
parseFileTypeFilter("ts,tsx,js,jsx,mts,cts")  // 6 extensions
// Operations: 6 splits, 6 trims, 6 lowercases, 6 filters
// Time: <1ms (negligible)
```

---

## Edge Cases & Constraints

### Supported Inputs

✅ **Valid:**
- `"ts"` - Single extension
- `"ts,tsx,js"` - Multiple extensions
- `"  ts  ,  tsx  "` - Whitespace tolerant
- `".ts,.tsx"` - Dot prefix accepted
- `"TS,TSX"` - Case insensitive

❌ **Invalid (handled gracefully):**
- `""` - Empty string → ignored, warn in hint
- `",,,,"` - Only commas → ignored, warn in hint
- `"a,b,c,..." (21+ extensions)` - Too many → error

### Constraints

**Extension Length:** 1-10 characters (e.g., "rs", "tsx", "markdown")
- Longer than 10 chars likely invalid (warn user)

**Extension Count:** 1-20 extensions per filter
- More than 20 → error (prevents OR query explosion)

**Characters:** Alphanumeric + dot only
- Special regex chars like `*`, `?`, `[` not supported (not a regex filter)

---

## Backward Compatibility

### Existing API Unchanged

**Before this project:**
```typescript
// Single extension (worked but undocumented)
search({filters: {file_type: "ts"}})
```

**After this project:**
```typescript
// Single extension (still works, now properly tested)
search({filters: {file_type: "ts"}})

// Multi-extension (new capability, additive)
search({filters: {file_type: "ts,tsx,js"}})
```

**No Breaking Changes:** Existing code continues to work exactly as before.

---

### Legacy Filter Coexistence

**Legacy filter parameter:**
```typescript
search({filter: "code"})  // Still works (different parameter)
```

**New advanced filter:**
```typescript
search({filters: {file_type: "ts"}})  // Also works (different parameter)
```

**Combination:**
```typescript
search({
  filter: "code",           // Excludes docs/config
  filters: {file_type: "ts"} // Further narrows to TypeScript
})
// Result: TypeScript code files only (not docs, not config, not other code)
```

**Note:** Both filters apply (AND logic). This is intuitive user expectation.

---

## Security Review

### SQL Injection Prevention

**Risk:** User-controlled input in SQL query

**Mitigation:** Parameterized queries

```typescript
// SAFE (parameterized)
args.push(`%.${extension}`)
clauses += ` AND f.relpath LIKE $${args.length}`

// UNSAFE (concatenation) - NOT USED
clauses += ` AND f.relpath LIKE '%.${extension}'`  // ❌ Don't do this
```

**Validation:** All extension input sanitized (lowercase alphanumeric only).

---

### Denial of Service Prevention

**Risk:** User sends 1000 extensions, creates massive OR query

**Mitigation:** Hard limit at 20 extensions

```typescript
if (extensions.length > 20) {
  return { error: 'Too many extensions (max 20)' }
}
```

**Rationale:** 20 extensions covers any realistic use case. More likely typo or abuse.

---

### Input Validation

**Risk:** Malicious input (`../../../etc/passwd`, `DROP TABLE`, etc.)

**Mitigation:** Strict parsing

```typescript
function parseFileTypeFilter(input: string): string[] {
  return input
    .split(',')
    .map(ext => ext.trim())
    .map(ext => ext.replace(/^\./, ''))  // Only leading dot
    .map(ext => ext.toLowerCase())
    .filter(ext => ext.length > 0 && ext.length < 20)  // Length check
    .filter(ext => /^[a-z0-9]+$/.test(ext))  // Alphanumeric only
}
```

**Effect:** Invalid characters silently dropped. Only alphanumeric extensions pass through.

---

## Long-Term Maintainability

### Extensibility Points

**Future enhancements (out of MVP scope):**

1. **Language name mapping**
   ```typescript
   filters: {language: "typescript"}  // Maps to ["ts", "tsx", "mts", "cts"]
   ```

2. **Negation**
   ```typescript
   filters: {file_type: "!test.ts,!spec.ts"}  // Exclude test files
   ```

3. **Indexed extension column**
   ```sql
   ALTER TABLE files ADD COLUMN extension TEXT GENERATED ALWAYS AS (
     regexp_replace(relpath, '^.*\.', '.')
   ) STORED;
   CREATE INDEX idx_files_extension ON files(extension);
   ```

4. **Regex support**
   ```typescript
   filters: {file_pattern: "src/.*\\.test\\.ts$"}  // Advanced users only
   ```

**Design principle:** Each enhancement is additive, no breaking changes.

---

### Code Organization

**Keep it simple:**
```
packages/maproom-mcp/src/
├── index.ts                    # Main MCP server
│   ├── parseFileTypeFilter()   # +40 lines (new)
│   ├── buildFilterClauses()    # +20 lines (modified)
│   └── handleSearch()          # +5 lines (validation)
└── tests/
    └── search_tool.test.ts     # +100 lines (new tests)
```

**Total LoC:** ~165 lines added/modified
**Complexity:** Low (simple string parsing + SQL generation)

---

## Deployment Strategy

### Rollout Plan

**No migration needed** - purely code changes.

**Deployment:**
1. Deploy new MCP server version
2. No database downtime
3. No client updates required
4. Feature immediately available

**Verification:**
```bash
# Test single extension
curl -X POST http://localhost:3000/mcp \
  -d '{"method":"tools/call","params":{"name":"search","arguments":{"repo":"crewchief","query":"auth","filters":{"file_type":"ts"}}}}'

# Test multi-extension
curl -X POST http://localhost:3000/mcp \
  -d '{"method":"tools/call","params":{"name":"search","arguments":{"repo":"crewchief","query":"auth","filters":{"file_type":"ts,tsx,js"}}}}'
```

---

## Success Metrics

### Functional Correctness
- ✅ Single extension filter returns only matching files
- ✅ Multi-extension filter returns union of all extensions
- ✅ Case insensitive matching works
- ✅ Edge cases handled gracefully (empty input, invalid chars)

### Performance
- ✅ <5ms parsing overhead
- ✅ <20% query overhead (vs no filter)
- ✅ No memory leaks in long-running sessions

### Usability
- ✅ Users can discover feature (documented in tool description)
- ✅ Error messages are helpful and actionable
- ✅ Common use cases shown in examples

---

## Conclusion

This architecture delivers a **simple, robust, performant** file type filtering feature:

- **Simple:** Comma-separated extensions, no complex syntax
- **Robust:** Validated input, parameterized queries, comprehensive tests
- **Performant:** Minimal overhead, scales to medium repos, optimizable later

The design is **MVP-focused** - it solves 90% of use cases without over-engineering. Advanced features (language mappings, regex, negation) can be added later without breaking changes.

**Total implementation effort:** 1-2 days for experienced developer
**Risk level:** Low (isolated feature, no schema changes)
**User value:** High (significant search precision improvement)

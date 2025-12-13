# Architecture: FTS Query Sanitization

## Overview

This is a **comprehensive bug fix** targeting incomplete FTS5 query sanitization. The current implementation uses individual `.replace()` calls for specific characters, leaving gaps for many special characters. The solution replaces this with a **regex whitelist approach** that sanitizes ALL non-alphanumeric characters (except underscore) in one operation.

**Scope:** Refactor sanitization logic to use regex whitelist + comprehensive unit tests

**Architectural improvement** - Changes from fragile character-by-character handling to robust whitelist-based sanitization, making the code more maintainable and future-proof.

## Design Decisions

### Decision 1: Use Regex Whitelist Instead of Individual Replace Calls

**Context:** Current approach uses individual `.replace()` calls for each special character:
1. Continue pattern: Add `.replace('.', " ")`, `.replace('/', " ")`, etc.
2. Switch to regex: Use `Regex::new(r"[^a-zA-Z0-9_\s]").unwrap().replace_all()`

**Decision:** Switch to regex whitelist (Option 2)

**Rationale:**
1. **Comprehensive** - Handles ALL non-alphanumeric characters, not just known problematic ones
2. **Maintainable** - Single regex pattern vs. dozens of `.replace()` calls
3. **Future-proof** - No updates needed when new special characters are discovered
4. **Consistent** - Same pattern used by PostgreSQL FTS module (lines 50-82 in src/search/fts.rs)
5. **Clear intent** - Whitelist approach makes the rule obvious: "only allow alphanumeric and underscore"

**Code location:** Lines 49-56 in `/crates/maproom/src/db/sqlite/fts.rs`

```rust
// BEFORE (incomplete)
let clean = t
    .replace('"', "")
    .replace('\'', "")
    .replace('*', "")
    .replace('(', "")
    .replace(')', "")
    .replace('-', " ")
    .replace(':', " ");

// AFTER (comprehensive)
use once_cell::sync::Lazy;
use regex::Regex;

static SPECIAL_CHAR_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[^a-zA-Z0-9_\s]").unwrap()
});

let clean = SPECIAL_CHAR_REGEX.replace_all(t, " ").to_string();
```

**Why this pattern:**
- `[^...]` - Negated character class (match anything NOT in the set)
- `a-zA-Z0-9` - All alphanumeric characters
- `_` - Underscore (valid in FTS5 barewords)
- `\s` - Whitespace (preserve existing spaces)
- Result: ALL other characters are replaced with spaces

### Decision 2: Use Lazy Static for Regex Compilation

**Context:** Regex compilation has overhead. Options:
1. Compile on every function call
2. Use `once_cell::sync::Lazy` for one-time compilation
3. Use `lazy_static!` macro

**Decision:** Use `once_cell::sync::Lazy` (Option 2)

**Rationale:**
1. **Performance** - Regex compiled once, reused for all queries
2. **Modern** - `once_cell` is the modern approach (destined for std)
3. **Already used** - CrewChief already depends on `once_cell` (via other crates)
4. **Thread-safe** - `Lazy<Regex>` is `Sync`, safe for concurrent use

### Decision 3: Comprehensive Character Table

**Context:** Document which characters are sanitized for maintainability

**Decision:** Add comprehensive table showing ALL FTS5 bareword violations

**Character sanitization table:**

| Character | Category | FTS5 Status | Sanitization |
|-----------|----------|-------------|--------------|
| `a-z A-Z` | Letters | Valid bareword | Preserved |
| `0-9` | Digits | Valid bareword | Preserved |
| `_` | Underscore | Valid bareword | Preserved |
| ` ` (space) | Whitespace | Word separator | Preserved |
| `.` | Dot | Invalid bareword | → space |
| `/` | Forward slash | Invalid bareword | → space |
| `\` | Backslash | Invalid bareword | → space |
| `[` `]` | Brackets | FTS5 column filter | → space |
| `{` `}` | Braces | Invalid bareword | → space |
| `(` `)` | Parentheses | FTS5 grouping | → space |
| `@` | At sign | Invalid bareword | → space |
| `#` | Hash | Invalid bareword | → space |
| `$` | Dollar | Invalid bareword | → space |
| `%` | Percent | Invalid bareword | → space |
| `^` | Caret | FTS5 column prefix | → space |
| `&` | Ampersand | Invalid bareword | → space |
| `*` | Asterisk | FTS5 prefix operator | → space |
| `+` | Plus | Invalid bareword | → space |
| `-` | Hyphen | FTS5 column filter | → space |
| `=` | Equals | Invalid bareword | → space |
| `!` | Exclamation | FTS5 negation (context) | → space |
| `\|` | Pipe | Invalid bareword | → space |
| `:` | Colon | FTS5 column filter | → space |
| `;` | Semicolon | Invalid bareword | → space |
| `"` | Double quote | FTS5 phrase operator | → space |
| `'` | Single quote | Invalid bareword | → space |
| `` ` `` | Backtick | Invalid bareword | → space |
| `<` `>` | Angle brackets | Invalid bareword | → space |
| `,` | Comma | Invalid bareword | → space |
| `?` | Question | Invalid bareword | → space |
| `~` | Tilde | Invalid bareword | → space |

**Note:** The regex `[^a-zA-Z0-9_\s]` handles ALL rows marked "→ space" automatically.

## Technology Choices

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Pattern matching | `regex::Regex` | Comprehensive, maintainable solution |
| Lazy initialization | `once_cell::sync::Lazy` | One-time regex compilation, thread-safe |
| Test framework | Built-in Rust `#[test]` | Matches existing test structure |
| Pattern | Negated character class `[^a-zA-Z0-9_\s]` | Whitelist approach, clear intent |

**Dependencies:**
- `regex` - Already in `Cargo.toml` (used throughout maproom)
- `once_cell` - Already in dependency tree (no explicit add needed)

## Component Design

### Modified Component: `build_fts_query()`

**Location:** `/crates/maproom/src/db/sqlite/fts.rs`, lines 43-74

**Responsibility:** Sanitize user input for FTS5 query syntax

**Interface (unchanged):**
```rust
pub fn build_fts_query(query: &str) -> String
```

**Internal logic change:**
```rust
// BEFORE (line 49-56)
let clean = t
    .replace('"', "")
    .replace('\'', "")
    .replace('*', "")
    .replace('(', "")
    .replace(')', "")
    .replace('-', " ")
    .replace(':', " ");

// AFTER (add one line)
let clean = t
    .replace('"', "")
    .replace('\'', "")
    .replace('*', "")
    .replace('(', "")
    .replace(')', "")
    .replace('-', " ")
    .replace('.', " ")      // NEW
    .replace(':', " ");
```

**Behavior:**
- Input: `"package.json"` → Output: `"package* OR json*"`
- Input: `"src/main.rs"` → Output: `"src* OR main* OR rs*"`
- Input: `"array[0]"` → Output: `"array* OR 0*"`
- Input: `"user@email.com"` → Output: `"user* OR email* OR com*"`

### New Test: `test_build_fts_query_comprehensive_sanitization`

**Location:** `/crates/maproom/src/db/sqlite/fts.rs`, after line 301 (after `test_build_fts_query_colon_handling`)

**Purpose:** Verify comprehensive special character sanitization using regex whitelist

**Test cases:**
```rust
#[test]
fn test_build_fts_query_comprehensive_sanitization() {
    // Dots (file extensions)
    let query = build_fts_query("package.json");
    assert_eq!(query, "package* OR json*");

    // Slashes (file paths)
    let query = build_fts_query("src/main.rs");
    assert_eq!(query, "src* OR main* OR rs*");

    // Brackets (array syntax)
    let query = build_fts_query("array[0]");
    assert_eq!(query, "array* OR 0*");

    // Braces (template syntax)
    let query = build_fts_query("template{value}");
    assert_eq!(query, "template* OR value*");

    // At sign (email/decorators)
    let query = build_fts_query("user@email.com");
    assert_eq!(query, "user* OR email* OR com*");

    // Backslash (Windows paths)
    let query = build_fts_query("path\\to\\file");
    assert_eq!(query, "path* OR to* OR file*");

    // Mixed special characters
    let query = build_fts_query("src/main@v2.rs");
    assert_eq!(query, "src* OR main* OR v2* OR rs*");

    // Operators
    let query = build_fts_query("a+b=c");
    assert_eq!(query, "a* OR b* OR c*");
}
```

## Data Flow

No changes to data flow - the sanitization occurs in the same location:

```
User query
    ↓
build_fts_query()
    ├─ Split whitespace
    ├─ Sanitize each term (DOT SANITIZATION HERE)
    ├─ Filter empty terms
    └─ Join with OR + prefix wildcard
    ↓
FTS5 query string
    ↓
search_fts() → SQLite FTS5 engine
```

## Integration Points

### Callers (unchanged)

1. **`search_fts()`** in `/crates/maproom/src/db/sqlite/fts.rs` (line 97)
   - Calls `build_fts_query()` to sanitize user input
   - No changes required

2. **Daemon RPC** via `SearchParams::mode = "fts"`
   - Uses `search_fts()` internally
   - No changes required

3. **CLI** via `crewchief-maproom search --mode fts`
   - Uses `search_fts()` internally
   - No changes required

### Integration with Hybrid Search

The `build_fts_query()` function is also used by hybrid search (FTS + vector fusion). This fix improves hybrid search quality for dotted queries:

- **Before:** Dotted queries fail in FTS component, causing hybrid search to fall back to vector-only
- **After:** Dotted queries work in both FTS and vector, enabling true hybrid ranking

## Search Quality Impact

### Semantic Change from Special Character Replacement

**Important behavioral note:** Converting special characters to spaces changes query semantics from "exact match" to "contains any term".

**Example:**
- Query: `package.json`
- Sanitized: `package json`
- FTS Query: `package* OR json*`
- Matches: ANY file containing "package" OR "json"

**What this matches:**
- `package.json` ✓ (desired - contains both terms)
- `package.ts` ✓ (contains "package")
- `config.json` ✓ (contains "json")
- `package_manager.rs` ✓ (contains "package")

**What this doesn't match:**
- Files without "package" or "json" ✗

**Why this is acceptable:**
1. **FTS mode is keyword search** - It's not designed for exact filename matching
2. **Vector search handles precision** - Hybrid mode uses vector similarity for ranking
3. **User intent** - User searching "package.json" likely wants files related to package configuration
4. **Better than failure** - Returning broader results is better than syntax errors

**Future consideration:** Add filename-specific search mode if exact matching is needed.

## Performance Considerations

### Runtime Performance

- **Change:** Replace 7 `.replace()` calls with 1 regex operation
- **Cost:** O(n) where n = term length (typically <20 characters)
- **Regex compilation:** One-time cost (amortized to zero via `Lazy`)
- **Impact:** Likely neutral or slight improvement (fewer allocations)
- **Queries/sec:** Expected to maintain 10,000+ queries/sec

### Memory

- **Temporary allocations:** Each `replace()` allocates a new String
- **Total overhead:** ~7 allocations per term (unchanged count, dots are just one more)
- **Impact:** Negligible (<1KB per query)

### Index Size

- **No change** - FTS5 index structure unchanged
- **No migration** - Existing indexes work without modification

## Maintainability

### Code Quality

- **Readability:** Dot sanitization follows identical pattern to existing code
- **Discoverability:** Grouped with other separator replacements
- **Documentation:** Comment mirrors existing comments

### Test Coverage

- **Before:** 10 tests for sanitization edge cases
- **After:** 11 tests (added dot handling test)
- **Coverage:** Maintains 100% coverage of sanitization logic

### Future Extensions

The regex whitelist approach makes future extensions unnecessary - ALL special characters are already handled. If FTS5 rules change (unlikely), only the regex pattern needs updating:

```rust
// Example: If underscore became invalid (hypothetical)
Regex::new(r"[^a-zA-Z0-9\s]").unwrap()  // Removed underscore from whitelist
```

**No per-character maintenance required.**

## Edge Cases Handled

All edge cases are handled by existing logic (no new code required):

| Edge Case | Input | Sanitized | FTS Query | Behavior |
|-----------|-------|-----------|-----------|----------|
| Multiple dots | `a..b` | `a  b` | `a* OR b*` | Extra spaces filtered |
| Leading dot | `.gitignore` | ` gitignore` | `gitignore*` | Leading space filtered |
| Trailing dot | `file.` | `file ` | `file*` | Trailing space filtered |
| Only dots | `...` | `   ` | `""` | Empty query (line 62-64) |
| Mixed separators | `a-b.c:d` | `a b c d` | `a* OR b* OR c* OR d*` | All separators normalized |

## Alternatives Considered

### Alternative 1: Quote the Entire Query

**Approach:** Wrap user query in double quotes: `"package.json"`

**Rejected because:**
- Requires phrase matching (less flexible)
- Breaks multi-word queries: `"search function"` would require exact phrase
- Inconsistent with existing sanitization approach
- Worse search quality (too restrictive)

### Alternative 2: Escape Dots Instead of Replacing

**Approach:** Escape dots in FTS5 syntax

**Rejected because:**
- FTS5 doesn't support escaping dots (they're not operators, they're invalid barewords)
- Would require wrapping terms in quotes anyway
- More complex than space replacement
- No benefit over space replacement

### Alternative 3: Remove Dots Entirely

**Approach:** `.replace('.', "")`

**Rejected because:**
- `package.json` → `packagejson*` is worse than `package* OR json*`
- Inconsistent with hyphen/colon handling
- Loses semantic information (dot indicates boundary)

## Migration Strategy

**No migration required** - This is a query-time fix with no database changes:

1. Deploy new binary (with fix)
2. Existing searches immediately benefit
3. No data migration
4. No downtime
5. Fully backward compatible

## Rollback Plan

If issues arise (unlikely), rollback is trivial:

1. Deploy previous binary version
2. Behavior returns to previous state
3. No data cleanup needed (no persistent changes)

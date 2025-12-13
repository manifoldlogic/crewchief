# Analysis: FTS Query Sanitization

## Problem Definition

The `build_fts_query()` function in `/crates/maproom/src/db/sqlite/fts.rs` sanitizes some special FTS5 characters to prevent syntax errors, but is missing comprehensive sanitization for ALL invalid bareword characters. According to FTS5 documentation, only alphanumeric characters (A-Z, a-z, 0-9) and underscore (`_`) are valid in bareword queries. ALL other characters cause syntax errors.

**Error manifestation:**
```
fts5: syntax error near '.'
fts5: syntax error near '/'
fts5: syntax error near '['
```

**Failed queries (common real-world patterns):**
- `package.json` → FTS5 syntax error (dot)
- `src/main.rs` → FTS5 syntax error (forward slash)
- `array[0]` → FTS5 syntax error (square brackets)
- `user@email.com` → FTS5 syntax error (at sign)
- `template{value}` → FTS5 syntax error (curly braces)
- `path\to\file` → FTS5 syntax error (backslash)
- `a+b=c` → FTS5 syntax error (plus/equals operators)
- `!important` → FTS5 syntax error (exclamation)
- `column|value` → FTS5 syntax error (pipe)
- `^anchor` → FTS5 syntax error (caret)

**Working queries:**
- `package json` → Works (space-separated)
- `package` → Works (single term)
- `user_name` → Works (underscore is valid)

## Context

### User Impact

Searches containing special characters are extremely common in code search workflows. Developers frequently search for:
- Configuration files: `package.json`, `tsconfig.json`, `.eslintrc.json` (dots)
- File paths: `src/main.rs`, `lib/utils.ts`, `tests/unit/test.py` (slashes)
- Array access: `array[0]`, `list[index]`, `map[key]` (brackets)
- Template syntax: `{config}`, `${variable}`, `{{template}}` (braces)
- Email/decorators: `user@domain.com`, `@Component`, `@override` (at signs)
- Operators: `a+b`, `x=y`, `value++`, `result+=1` (math operators)
- Windows paths: `C:\Program Files\`, `path\to\file` (backslashes)
- Logical operators: `value|default`, `!important`, `condition^flag` (various)

All of these queries currently fail in FTS mode (the default fallback when embeddings are unavailable), making search unusable for these critical patterns.

### Why This Matters

1. **FTS is the fallback mode** - When embeddings aren't available, all searches use FTS
2. **Common query pattern** - File extensions and dotted paths are fundamental search patterns
3. **Complete failure** - The error prevents any results, not just degraded ranking
4. **User confusion** - The syntax error message exposes implementation details

## Existing Solutions

### Current Sanitization (Lines 44-60)

The `build_fts_query()` function already handles several special characters:

```rust
let clean = t
    .replace('"', "")       // Remove quotes
    .replace('\'', "")      // Remove single quotes
    .replace('*', "")       // Remove wildcards
    .replace('(', "")       // Remove grouping
    .replace(')', "")       // Remove grouping
    .replace('-', " ")      // Treat hyphen as space
    .replace(':', " ");     // Treat colon as space
```

**Pattern established:**
- Remove characters that are FTS5 operators (`"`, `*`, `()`)
- Replace separators with spaces (`-`, `:`)

### FTS5 Special Characters (from sqlite.org)

According to FTS5 documentation, bareword queries can only contain:
- ASCII letters (A-Z, a-z)
- Decimal digits (0-9)
- Underscore (`_`)
- Substitute character (unicode 26)

**All other characters** must be quoted or cause syntax errors.

**Comprehensive list of problematic characters:**
- **Dots** (`.`) - Tokenized as separators, cause syntax errors in bareword queries
- **Slashes** (`/`, `\`) - Path separators, invalid barewords
- **Brackets** (`[`, `]`) - FTS5 column filters, invalid in basic queries
- **Braces** (`{`, `}`) - Invalid barewords
- **At sign** (`@`) - Invalid bareword
- **Operators** (`+`, `-`, `=`, `!`, `|`, `^`, `&`, `<`, `>`) - Various FTS5/SQL operators
- **Parentheses** (`(`, `)`) - FTS5 grouping operators (already sanitized)
- **Quotes** (`"`, `'`) - FTS5 phrase operators (already sanitized)
- **Asterisk** (`*`) - FTS5 prefix operator (already sanitized)
- **Colon** (`:`) - FTS5 column filter (already sanitized)
- **Others** (`#`, `$`, `%`, `,`, `;`, `?`, `~`, `` ` ``) - Invalid barewords

The current sanitization only handles quotes, asterisks, parentheses, hyphens, and colons. This leaves gaps for dots, slashes, brackets, braces, at-signs, and many operators.

### Comparison with PostgreSQL FTS Module

The PostgreSQL FTS executor at `/crates/maproom/src/search/fts.rs` handles dots differently - it normalizes them for exact match detection (line 69):

```rust
// Step 4: Handle kebab-case, spaces, and dots → snake_case
let re4 = Regex::new(r"[\s\-\.]").unwrap();
normalized = re4.replace_all(&normalized, "_").to_string();
```

This converts dots to underscores for symbol matching, which is appropriate for PostgreSQL's tsquery syntax.

## Current State

### Affected Code Location

**File:** `/crates/maproom/src/db/sqlite/fts.rs`
**Function:** `build_fts_query()` (lines 43-74)
**Issue:** Missing `.replace('.', " ")` between lines 55-56

### Test Coverage

The function has comprehensive test coverage for existing sanitization:
- `test_build_fts_query_sanitize_quotes` (line 260)
- `test_build_fts_query_sanitize_wildcards` (line 266)
- `test_build_fts_query_sanitize_parens` (line 272)
- `test_build_fts_query_hyphen_handling` (line 290)
- `test_build_fts_query_colon_handling` (line 297)

**Missing:** No test for dot handling

## Research Findings

### Why Dots Fail

1. **FTS5 bareword rules** - Dots are not valid bareword characters
2. **Tokenizer behavior** - Dots are separators, not searchable content
3. **Query context** - `package.json` is treated as query syntax, not a phrase

### Why Space Replacement Works

The existing pattern of replacing separators with spaces leverages FTS5's OR operator construction:

```rust
words.iter()
    .flat_map(|w| w.split_whitespace())
    .filter(|w| !w.is_empty())
    .map(|w| format!("{}*", w))
    .collect::<Vec<_>>()
    .join(" OR ")
```

**Examples of comprehensive sanitization:**
- `package.json` → `package json` → `package* OR json*`
- `src/main.rs` → `src main rs` → `src* OR main* OR rs*`
- `array[0]` → `array 0` → `array* OR 0*`
- `user@email.com` → `user email com` → `user* OR email* OR com*`

This approach:
- Searches for chunks containing ANY of the terms (OR logic)
- Uses prefix matching (the `*` wildcard)
- Returns relevant results for all types of queries
- Is semantically correct - we want to find content matching any of the extracted terms

## Constraints

### Technical Constraints

1. **Single location change** - Solution should modify only the sanitization logic in one place
2. **Pattern improvement** - Use regex whitelist for comprehensive coverage (more maintainable than individual `.replace()` calls)
3. **No breaking changes** - Must not alter behavior for existing working queries
4. **Test coverage** - Must add tests for all newly-handled character types

### Performance Constraints

- String replacement is O(n), negligible overhead
- No impact on query execution time
- No database schema changes

### Compatibility Constraints

- Must work with existing FTS5 indexes
- No migration required
- Backward compatible with all query patterns

## Success Criteria

### Functional Success

- [ ] Query `package.json` returns results (no syntax error)
- [ ] Query `src/main.rs` returns results (no syntax error)
- [ ] Query `array[0]` returns results (no syntax error)
- [ ] Query `user@email.com` returns results (no syntax error)
- [ ] Query `template{value}` returns results (no syntax error)
- [ ] Query `path\to\file` returns results (no syntax error)
- [ ] Query `a+b=c` returns results (no syntax error)

### Technical Success

- [ ] Unit test added for comprehensive special character sanitization
- [ ] Tests verify dots, slashes, brackets, braces, at-signs, operators all handled
- [ ] Existing tests continue to pass
- [ ] Performance baseline measured (queries/sec before and after)
- [ ] No measurable performance regression (<5% threshold)

### User Experience Success

- [ ] File extension searches work intuitively
- [ ] No syntax error messages exposed to users
- [ ] Behavior consistent with hyphen and colon handling
- [ ] Dotted identifiers searchable

## Edge Cases to Consider

1. **Multiple consecutive special chars** - `package..json`, `path//file` → should work (extra chars become spaces)
2. **Leading/trailing special chars** - `.gitignore`, `/path/` → should work
3. **Mixed special chars** - `package.*.json`, `src/main@v2` → should work (all sanitized)
4. **Empty terms from special chars** - `....`, `////` → should return empty query (handled by existing filter)
5. **Only special chars** - `@#$%` → should return empty query (all chars removed)
6. **Complex paths** - `C:\Program Files\app\config.json` → should extract meaningful terms
7. **Email addresses** - `user@domain.com` → should extract "user", "domain", "com"
8. **Array notation** - `arr[0]`, `map[key][0]` → should extract terms
9. **Template syntax** - `{a: b}`, `${var}` → should extract alphanumeric content
10. **Operators in code** - `x+y=z`, `a!=b`, `c|d` → should extract variable names

All edge cases are handled by the regex whitelist approach - only alphanumeric and underscore are preserved, everything else becomes spaces, and split_whitespace + filtering removes empty terms.

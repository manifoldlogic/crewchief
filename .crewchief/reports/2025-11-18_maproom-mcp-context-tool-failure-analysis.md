# Maproom MCP Failure Report: Context Tool Blocked by Search Failures

**Date:** November 18, 2025
**Reporter:** Claude Code
**Severity:** Critical - Core functionality blocked

---

## Executive Summary

I was **unable to use the `context` tool at all** because it requires a `chunk_id`, which I could only obtain through the `search` tool. The search tool failed to find the actual code I was looking for, returning only documentation, tests, or nothing at all. This created a **blocking dependency chain failure**.

The context tool is one of the most valuable features of the Maproom MCP, providing callers, imports, tests, and related code for a given chunk. However, its requirement for a chunk_id makes it completely dependent on search working correctly. When search fails, context becomes unusable.

---

## Critical Failure #1: Search Cannot Find Obvious Code

### The Function I Was Looking For

```rust
// Location: crates/maproom/src/main.rs:15-23
/// Validate provider name against supported providers.
///
/// Returns the provider name if valid, or an error message if invalid.
fn validate_provider(s: &str) -> Result<String, String> {
    match s.to_lowercase().as_str() {
        "ollama" | "openai" | "google" => Ok(s.to_lowercase()),
        _ => Err(format!(
            "Invalid provider: '{}'. Supported providers: ollama, openai, google",
            s
        )),
    }
}
```

### What I Tried

| Query | Mode | Filter | Worktree | Result |
|-------|------|--------|----------|--------|
| `"validate_provider function"` | hybrid | code | main | TypeScript functions only, not Rust |
| `"fn validate_provider Rust"` | hybrid | code | main | **ZERO RESULTS** |
| `"validate provider ollama openai google"` | fts | none | main | Documentation tickets only |
| `"Invalid provider Supported providers ollama openai google"` | fts | none | none | Documentation tickets only |
| `"main.rs validate provider supported"` | hybrid | none | main | Documentation tickets only |
| `"validate_provider"` with `filters: {file_type: "rs"}` | hybrid | code | main | Tests only, not implementation |

### Why This is a Blatant Failure

The function has **highly distinctive, unique text**:
- Function name: `validate_provider`
- Unique string literal: `"Invalid provider: '{}'. Supported providers: ollama, openai, google"`
- Comment: `"Validate provider name against supported providers"`
- Distinctive pattern: `"ollama" | "openai" | "google"`

**There is no ambiguity.** This exact text exists in exactly one place in the main codebase. Yet search couldn't find it with any combination of:
- Semantic search (hybrid mode)
- Full-text search (fts mode)
- Code filtering
- Worktree filtering
- File type filtering

### What Traditional Tools Found Instantly

```bash
# Grep found it immediately
$ grep -rn "fn validate_provider" crates/maproom/src/
crates/maproom/src/main.rs:15:fn validate_provider(s: &str) -> Result<String, String> {

# Glob found the file immediately
$ find . -name "main.rs" | grep maproom
./crates/maproom/src/main.rs
```

---

## Critical Failure #2: Context Tool Unusable Without Search

### The Dependency Chain

```
User: "Show me what maproom context returns for validate_provider"
    ↓
Assistant: Need to call context tool (requires chunk_id)
    ↓
Assistant: Must use search tool to get chunk_id for validate_provider
    ↓
Search: Returns wrong chunks (tests, docs) or no results
    ↓
Assistant: Cannot get correct chunk_id for the actual function
    ↓
BLOCKED: Cannot use context tool at all
    ↓
Assistant: Fall back to Read/Grep tools instead
```

### What I Would Have Done If Search Worked

**Ideal workflow:**
```javascript
// Step 1: Search finds the function
mcp__maproom__search({
  repo: "crewchief",
  query: "validate_provider",
  filter: "code",
  worktree: "main"
})
// Expected return:
// {
//   chunk_id: "12345",
//   relpath: "src/main.rs",
//   symbol_name: "validate_provider",
//   kind: "func",
//   start_line: 15,
//   end_line: 23
// }

// Step 2: Get context for that chunk
mcp__maproom__context({ chunk_id: "12345" })
// Would return:
// - Target chunk: validate_provider function
// - Callers: Line 94 in Scan command (value_parser)
// - Imports: clap Parser/Subcommand
// - Tests: test_validate_provider_case_insensitive, test_validate_provider_typo
// - Related: Scan command struct
```

**What actually happened:**
```javascript
// Step 1: Search fails to find the function
mcp__maproom__search({
  query: "validate_provider",
  filter: "code"
})
// Actual return:
// [
//   { chunk_id: "1570986", relpath: ".../tests/cli_test.rs", ... },  // Test file
//   { chunk_id: "1629511", relpath: ".../tests/cli_test.rs", ... },  // Test file (duplicate)
//   { chunk_id: "1746561", relpath: ".../tests/cli_test.rs", ... },  // Test file (duplicate)
//   ... 7 more duplicate test results from different worktrees
// ]

// Step 2: BLOCKED - no valid chunk_id for the implementation
// Cannot call context tool because I don't have the chunk_id for main.rs:15-23
// Only have chunk_ids for test files
```

### Why I Couldn't Use Context

**I had no valid chunk_id to pass to it.** The search results gave me chunk IDs for:
- Documentation markdown files in `.crewchief/`
- TypeScript validation functions in different packages
- Test files (`cli_test.rs`)
- Duplicate copies from 15+ stale worktrees

But **not the actual Rust implementation** in `src/main.rs`.

### Impact

The context tool is designed to show:
- **Callers**: Where a function is used
- **Imports**: Dependencies and relationships
- **Tests**: Test coverage for the code
- **Related code**: Surrounding context

Without the ability to find the correct chunk_id through search, **none of this functionality is accessible** for the code the user actually asked about.

---

## Critical Failure #3: The `open` Tool is Completely Broken

### Attempted Usage

```javascript
// Attempt 1: Full path from repo root
mcp__maproom__open({
  relpath: "crates/maproom/src/main.rs",
  worktree: "main"
})

// Error Response:
// {
//   "error": "FILE_NOT_FOUND",
//   "message": "Failed to check file size: ENOENT: no such file or directory,
//               stat '/workspace/crates/maproom/crates/maproom/src/main.rs'"
//                                        ^^^^^^^^^^^^^^^^^ PATH DUPLICATED
// }
```

```javascript
// Attempt 2: Path from worktree root
mcp__maproom__open({
  relpath: "src/main.rs",
  worktree: "main"
})

// Error Response:
// {
//   "error": "FILE_NOT_FOUND",
//   "message": "Failed to check file size: ENOENT: no such file or directory,
//               stat '/tmp/.tmpZgxDYt/src/main.rs'"
//                     ^^^^^^^^^^^^^^ WRONG BASE PATH (temp directory?)
// }
```

### Why This is a Blatant Failure

The `open` tool should work like this:
1. Query database for worktree with `name = "main"`
2. Get the `abs_path` from the result (e.g., `/workspace`)
3. Append the `relpath` parameter: `/workspace/src/main.rs`
4. Read and return that file

Instead it appears to:
- Duplicate path segments (`/workspace/crates/maproom/crates/maproom/...`)
- Use random temp directories (`/tmp/.tmpZgxDYt/`)
- Apply incorrect base paths
- Fail to read any files at all

### Attempted Workarounds

I tried multiple path variations:
- Full path from workspace root: `crates/maproom/src/main.rs`
- Path from crate root: `src/main.rs`
- Absolute path: `/workspace/crates/maproom/src/main.rs` (rejected by tool)

**None worked.** The tool is fundamentally broken.

### Impact

The `open` tool is meant to:
- Retrieve code with specific line ranges from search results
- Get context around a chunk without reading the entire file
- View code the user asked about

**This makes the tool completely unusable.** Even when search returns valid file paths, you cannot actually read them through the MCP.

---

## Critical Failure #4: Index Pollution Prevents Accurate Results

### The Problem: 100+ Stale Worktrees

```javascript
mcp__maproom__status({ repo: "crewchief" })

// Returns 100+ worktrees, most from genetic algorithm experiments:
{
  "worktrees": [
    { "name": "main", "fileCount": 13604, "chunkCount": 485031 },  // ← Only 1 real worktree
    { "name": "variant-variant-amplification-gen10-mhzgz75mbqmv-1763161922314", "fileCount": 0 },
    { "name": "variant-variant-crossover-gen9-mhzgkcdirpen-1763161743372", "fileCount": 0 },
    { "name": "variant-variant-reduction-gen8-mhzg5pagiw9i-1763160811196", "fileCount": 0 },
    { "name": "variant-variant-control-1763000691517", "fileCount": 1235 },
    { "name": "variant-variant-control-1762833032387", "fileCount": 1652 },
    ... // 95 more stale experimental worktrees
  ]
}
```

### Impact on Search Quality

When searching for code, the results include:
- **15+ duplicate hits** from different variant worktrees
- All with **identical content** (copied from main)
- Only **1-2 hits** from the actual "main" worktree
- **Signal completely buried in noise**

### Real Example from My Session

Search for `"validate_provider"` with `filters: {file_type: "rs"}`:

```javascript
{
  "hits": [
    {
      "chunk_id": "1570986",
      "relpath": "packages/cli/.crewchief/worktrees/variant-variant-reduction-gen5-mhzkebtyol4z-1762744236229/crates/maproom/tests/cli_test.rs",
      "symbol_name": "test_validate_provider_case_insensitive"
    },
    {
      "chunk_id": "1629511",
      "relpath": "packages/cli/.crewchief/worktrees/variant-variant-reduction-gen4-mhsk9vd8b3s6-1762744223593/crates/maproom/tests/cli_test.rs",
      "symbol_name": "test_validate_provider_case_insensitive"
    },
    {
      "chunk_id": "1746561",
      "relpath": "packages/cli/.crewchief/worktrees/variant-variant-crossover-gen5-mhskebtxfjci-1762744226438/crates/maproom/tests/cli_test.rs",
      "symbol_name": "test_validate_provider_case_insensitive"
    },
    // ... 7 more identical duplicates from different variant worktrees ...

    {
      "chunk_id": "1513051",
      "relpath": "crates/maproom/tests/cli_test.rs",  // ← The actual file I wanted
      "symbol_name": "test_validate_provider_case_insensitive"
    }
  ]
}
```

**All 10 results are the same test file, just indexed in different worktrees.**

### Why This Happens

The genetic algorithm competition system (`.crewchief/genetic-iterations/`) creates many temporary worktrees:
- Each generation creates multiple variant worktrees
- Each worktree gets indexed separately
- Old worktrees are never cleaned up from the index
- The index now contains 100+ worktrees, most with `fileCount: 0`

### Why This Breaks Search

1. **Ranking dilution**: The same chunk appears 15 times with slightly different scores
2. **Results saturation**: Top 10 results are all duplicates, actual content on page 2
3. **Noise >> Signal**: 95% of search results are from stale worktrees
4. **No deduplication**: Same content counted as 15 different results

---

## Root Cause Analysis

### Dependency Chain Failure

```
Context Tool (GOAL)
    ↑ requires
chunk_id
    ↑ obtained from
Search Tool
    ↑ fails due to
1. Index pollution (100+ stale worktrees)
2. Poor ranking (tests/docs before implementation)
3. Missing filters (can't filter by language)
4. Broken FTS (exact matches not found)
    ↓ results in
Wrong/missing chunk_id
    ↓ blocks
Context Tool (UNUSABLE)
```

### Why I Couldn't Complete the Task

**User request:** "Show me what maproom context returns for the validate_provider function"

**Expected workflow:**
1. Search for validate_provider → Get chunk_id
2. Call context with chunk_id → Get full context
3. Show user the context result

**Actual workflow:**
1. Search for validate_provider → Get wrong/no chunk_ids
2. **BLOCKED** - Cannot call context without valid chunk_id
3. Fall back to traditional tools (Read/Grep)
4. Manually describe what context *would* return

### The Core Problem

The context tool has **perfect functionality** but is **completely inaccessible** because:
- It requires a chunk_id parameter
- chunk_id must come from search results
- Search doesn't return the correct chunks
- Therefore context cannot be used

This is a **blocking dependency failure**: A working downstream tool is made useless by a broken upstream dependency.

---

## What Would Fix This

### Critical Fixes (Blocking Issues)

#### 1. Fix Search to Find Obvious Code
**Current:** Search for exact function name returns no results
**Required:** Exact matches in symbol names should rank first

**Example fix:**
```rust
// If query exactly matches a symbol name, boost score to 1.0
if chunk.symbol_name == query {
    score = 1.0;  // Guaranteed top result
}
```

#### 2. Clean Up Stale Worktrees from Index
**Current:** 100+ worktrees, 95% stale with 0 files
**Required:** Remove worktrees that no longer exist on disk

**Example fix:**
```sql
-- Find worktrees where directory no longer exists
DELETE FROM maproom.worktrees
WHERE NOT EXISTS (
  SELECT 1 FROM pg_stat_file(abs_path)
);

-- Or at minimum, exclude .crewchief/* from indexing
DELETE FROM maproom.worktrees
WHERE abs_path LIKE '%/.crewchief/%';
```

#### 3. Fix `open` Tool Path Resolution
**Current:** Path concatenation creates invalid paths
**Required:** Correctly join worktree abs_path + relpath

**Example fix:**
```typescript
// Current (broken):
const fullPath = path.join(worktree.abs_path, worktree.abs_path, relpath);
// Results in: /workspace/crates/maproom/crates/maproom/src/main.rs
//                                    ^^^^^^^^^^^^^^^^^ duplicated

// Fixed:
const fullPath = path.join(worktree.abs_path, relpath);
// Results in: /workspace/src/main.rs (if worktree root is /workspace)
```

#### 4. Implement Working File Type Filter
**Current:** `filters: {file_type: "rs"}` doesn't work
**Required:** Filter results by file extension

**Example fix:**
```sql
-- Add file extension to WHERE clause
WHERE file_path LIKE '%.rs'
```

### Medium-term Improvements

#### 5. Worktree Prioritization in Results
Prefer results from "main" worktree over experimental branches:
```rust
if chunk.worktree_name == "main" {
    score *= 1.5;  // 50% boost for main worktree
}
```

#### 6. Symbol Type Ranking
Functions should rank higher than tests/docs:
```rust
match chunk.kind {
    "func" | "class" | "struct" => score *= 1.3,
    "test" => score *= 0.8,
    _ => {}
}
```

#### 7. Deduplication of Results
Don't show same chunk from 15 different worktrees:
```rust
// Group by (relpath, symbol_name, start_line)
// Return highest-scoring instance only
```

---

## Impact Assessment

### What Broke

| Tool | Expected Behavior | Actual Behavior | Severity |
|------|------------------|-----------------|----------|
| **search** | Find code by name/content | Returns docs/tests/nothing | 🔴 Critical |
| **context** | Show related code for chunk | Cannot get chunk_id to use it | 🔴 Critical |
| **open** | Read file from search results | Path resolution completely broken | 🔴 Critical |
| **status** | Show indexed repos | Shows 100+ stale worktrees | 🟡 Major |

### User Experience

**User asks:** "Show me context for validate_provider"
**User expects:** Full context bundle showing callers, tests, imports
**User gets:** "I couldn't use the context tool because search couldn't find it"

This is a **complete failure** of the core value proposition: semantic code search and contextual understanding.

### Workaround Effectiveness

Traditional tools worked perfectly:
```bash
# Found function immediately
grep -rn "fn validate_provider" src/

# Read file with no issues
cat crates/maproom/src/main.rs
```

**Conclusion:** The MCP added no value and introduced significant friction compared to basic command-line tools.

---

## Recommendations

### Immediate Actions (This Week)

1. **Fix search ranking** - Exact symbol name matches must rank first
2. **Clean index** - Remove all `.crewchief/` worktrees from database
3. **Fix open tool** - Repair path resolution logic
4. **Test with real queries** - Verify search finds actual code, not just tests/docs

### Short-term Actions (This Month)

1. **Implement file_type filtering** - Make it actually work
2. **Add worktree filtering** - Default to "main", allow explicit filtering
3. **Deduplicate results** - Don't show same chunk 15 times
4. **Improve FTS** - Exact text matches should always work

### Long-term Actions (Next Quarter)

1. **Automatic index cleanup** - Detect and remove stale worktrees
2. **Result ranking algorithm** - Implementation > tests > docs
3. **Integration tests** - Verify search finds known symbols
4. **Performance optimization** - Handle large indexes better

---

## Conclusion

**I could not use the context tool** because:
1. It requires a chunk_id parameter
2. chunk_id comes from search results
3. Search failed to find the actual code
4. Wrong/missing chunk_ids made context unusable

The failures weren't about semantic search being slower than grep. They were about **core functionality being broken**:
- ❌ Search returns the wrong things (or nothing)
- ❌ Open tool cannot read any files
- ❌ 95% of index is stale duplicates
- ❌ Context tool blocked by search failures

Traditional tools (`Glob`, `Grep`, `Read`) worked perfectly because they don't depend on the index. They found the code instantly while the MCP search failed completely.

**The MCP should be better than grep, not worse.** Right now, it's worse.

---

## Appendix: Session Transcript Summary

**Task:** Find entry points and show context for `validate_provider` function

**Search attempts:** 6 different queries, multiple modes/filters
**Search successes:** 0 (found tests and docs, not implementation)

**Context tool usage:** 0 (blocked by lack of chunk_id)
**Open tool usage:** 2 attempts, 2 failures

**Traditional tools:** 3 tools (Glob, Grep, Read)
**Traditional tool success rate:** 100%

**Time spent on MCP:** ~10 minutes of failed searches
**Time spent on traditional tools:** ~30 seconds to find everything

**User satisfaction:** Frustrated enough to ask for failure report

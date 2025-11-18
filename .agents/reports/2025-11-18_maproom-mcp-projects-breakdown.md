# Maproom MCP Failures: Project Breakdown

**Date:** November 18, 2025
**Source:** Context Tool Failure Analysis Report
**Framework:** Project Boundary Evaluation (Stable Context Triangle)

---

## Overview

This document breaks down the critical failures identified in the Maproom MCP analysis into properly scoped projects. Each project is evaluated against the three core criteria: **Interface Stability**, **Context Coherence**, and **Testable Completion**.

### Source Failures Identified

From the failure analysis, four critical blocking issues were identified:

1. **Search Cannot Find Obvious Code** - Exact symbol name matches return no results
2. **Context Tool Blocked** - Cannot get chunk_id needed to use context tool
3. **Open Tool Broken** - Path resolution creates invalid file paths
4. **Index Pollution** - 100+ stale worktrees create duplicate results

### Dependency Analysis

```
Index Pollution (root cause)
    ↓ creates
Duplicate/Wrong Results
    ↓ breaks
Search Ranking
    ↓ prevents
Getting Valid chunk_id
    ↓ blocks
Context Tool Usage
    ↓ fails
User Request
```

---

## Project 1: Search Exact Match Priority

**Pattern:** Service Module
**Size:** Small (2-3 weeks, 8-12 tickets)
**Priority:** 🔴 Critical - Blocks context tool usage

### Project Summary

Implement exact symbol name matching in the search ranking algorithm. When a user searches for a specific function or symbol name, results where the symbol name exactly matches the query should rank at the top, regardless of other scoring factors. This ensures that search can find the actual code users are looking for, not just tests or documentation.

Currently, searching for "validate_provider" returns test files and documentation but not the actual `fn validate_provider` implementation. This project will fix the ranking algorithm to prioritize exact matches.

### Core Criteria Assessment

#### ✅ Interface Stability
**External Interfaces:**
- Database: `maproom.chunks` table (stable, no schema changes needed)
- MCP API: `search` tool signature (stable, no parameter changes)
- Embedding service: Not involved in ranking

**Stability Commitment:** ✅ Confirmed
**Risk Areas:** None - purely internal ranking logic

#### ✅ Context Coherence
**Domain Concepts:** 10 concepts
- Search query parsing
- Symbol name matching
- Exact vs. partial matching
- Scoring algorithm
- Rank boosting
- Result ordering
- FTS scoring
- Vector scoring
- Hybrid fusion
- Score normalization

**Core Modules:**
- `crates/maproom/src/search/mod.rs` - Main search logic
- `crates/maproom/src/search/ranking.rs` - Scoring algorithm (if exists)
- `packages/maproom-mcp/src/tools/search.ts` - MCP tool wrapper

**Context Size:** ~300 lines of code, easily fits in agent memory

#### ✅ Testable Completion
**Success Criteria:**
- [ ] Exact symbol name match ranks as top result (test: search "validate_provider" returns chunk with symbol_name="validate_provider" first)
- [ ] Partial symbol matches rank below exact matches (test: search "validate" returns exact "validate_provider" before "validate_config")
- [ ] Case-insensitive exact matching works (test: search "ValidateProvider" finds "validate_provider")
- [ ] Exact match boost doesn't break vector search (test: semantic queries still return relevant results)
- [ ] Integration test passes: search for known symbols in test repo

**Verification Method:** Automated test suite that searches for known symbols and verifies ranking

### Scope Definition

#### In Scope
- Modify search ranking algorithm to boost exact symbol name matches
- Add exact match detection logic
- Implement case-insensitive comparison
- Preserve existing FTS/vector/hybrid modes
- Add integration tests for exact matching

#### Out of Scope
- Changing search API or parameters
- Modifying embedding generation
- Altering database schema
- Fixing other search quality issues (deduplication, etc.)

#### Edge Cases
- **Query with spaces**: "validate provider" - Decision: Only match if symbol name exactly equals query with underscores
- **Multi-word symbols**: "HttpClient" - Decision: Match case-insensitively
- **Partial matches**: "validate" - Decision: Exact matches rank first, then partial matches

### Risk Assessment

| Risk | Impact on Agents | Mitigation |
|------|-----------------|------------|
| Ranking changes break existing searches | Medium - regression in search quality | Comprehensive test suite before/after |
| Exact match boost too aggressive | Low - other results still available | Tunable boost parameter |
| Case insensitive matching has edge cases | Low - standard string comparison | Use proven library functions |

### Dependencies
- None - completely independent

### Success Metrics
- Search for "validate_provider" returns implementation as #1 result
- Search for any exact symbol name in test corpus returns that symbol first
- Existing semantic searches continue to work
- Test suite passes with >95% coverage

---

## Project 2: Open Tool Path Resolution Fix

**Pattern:** Capability Layer
**Size:** XS (3-5 days, 3-4 tickets)
**Priority:** 🔴 Critical - Tool completely broken

### Project Summary

Fix the path resolution bug in the `mcp__maproom__open` tool that prevents it from reading any files. The tool currently duplicates path segments or uses incorrect base paths, making it completely unusable. This is a surgical fix to a single bug.

The tool should: (1) get worktree abs_path from database, (2) join with relpath parameter, (3) read the file. Currently it fails at step 2 with path construction errors.

### Core Criteria Assessment

#### ✅ Interface Stability
**External Interfaces:**
- MCP API: `open` tool (stable, bug fix only)
- Database: `maproom.worktrees` table (stable)
- File system: Standard Node.js fs operations (stable)

**Stability Commitment:** ✅ Confirmed
**Risk Areas:** None - purely internal bug fix

#### ✅ Context Coherence
**Domain Concepts:** 5 concepts
- Worktree absolute path
- Relative file path
- Path joining/resolution
- File reading
- Error handling

**Core Modules:**
- `packages/maproom-mcp/src/tools/open.ts` - The broken tool
- Path resolution logic (single function)

**Context Size:** ~50 lines of code, minimal

#### ✅ Testable Completion
**Success Criteria:**
- [ ] Open tool reads files with relpath from repo root (test: `open({relpath: "crates/maproom/src/main.rs", worktree: "main"})` succeeds)
- [ ] Open tool reads files with relpath from worktree root (test: `open({relpath: "src/main.rs", worktree: "main"})` succeeds)
- [ ] Open tool returns correct file contents (test: content matches actual file)
- [ ] Open tool handles line ranges correctly (test: range parameter works)
- [ ] Error handling for non-existent files (test: proper error message)

**Verification Method:** Integration test suite with known files

### Scope Definition

#### In Scope
- Fix path joining logic in open tool
- Correct worktree abs_path retrieval
- Add path resolution tests
- Verify file reading works for all valid paths

#### Out of Scope
- Changing open tool API
- Adding new features to open tool
- Optimizing file reading performance
- Handling binary files differently

#### Edge Cases
- **Symlinks in path**: Decision: Follow symlinks (standard fs.readFile behavior)
- **Relative paths with ..**: Decision: Reject paths that escape worktree root
- **Non-existent worktree**: Decision: Return clear error message

### Risk Assessment

| Risk | Impact on Agents | Mitigation |
|------|-----------------|------------|
| Path resolution edge cases | Low - clear error messages | Comprehensive path test suite |
| Breaking change to tool behavior | Low - currently non-functional | Any change is an improvement |

### Dependencies
- None - standalone fix

### Success Metrics
- All path resolution tests pass
- Open tool successfully reads files from search results
- Zero path duplication errors
- Error messages are clear and actionable

---

## Project 3: Index Stale Worktree Cleanup

**Pattern:** Service Module
**Size:** Small (1-2 weeks, 6-8 tickets)
**Priority:** 🟡 High - Significantly impacts search quality

### Project Summary

Implement automated detection and removal of stale worktrees from the maproom index. Currently, the index contains 100+ worktrees, 95% of which are from old genetic algorithm experiments that no longer exist on disk. This creates massive result duplication (same chunk appears 15 times) and buries actual results in noise.

This project will: (1) detect worktrees whose `abs_path` no longer exists on disk, (2) safely remove them from the database, (3) prevent future pollution by excluding certain paths (`.crewchief/`) from indexing.

### Core Criteria Assessment

#### ✅ Interface Stability
**External Interfaces:**
- Database: `maproom.worktrees`, `maproom.chunks` tables (stable)
- File system: Disk existence checks (stable)
- MCP API: No changes needed

**Stability Commitment:** ✅ Confirmed
**Risk Areas:** None - purely cleanup operation

#### ✅ Context Coherence
**Domain Concepts:** 12 concepts
- Worktree lifecycle
- Stale worktree detection
- Disk path validation
- Database cleanup
- Cascade deletion
- Index exclusion patterns
- Cleanup scheduling
- Path pattern matching
- Database transactions
- Cleanup verification
- Cleanup reporting
- Safety checks

**Core Modules:**
- `crates/maproom/src/db/cleanup.rs` - New module for cleanup logic
- `crates/maproom/src/indexer/exclusions.rs` - Path exclusion patterns
- Database migration for cleanup functions

**Context Size:** ~200 lines of code + SQL

#### ✅ Testable Completion
**Success Criteria:**
- [ ] Stale worktrees removed from database (test: worktree count drops from 100+ to <10)
- [ ] Only worktrees with valid disk paths remain (test: all abs_path values exist on disk)
- [ ] Cleanup is safe - no data loss for valid worktrees (test: main worktree unchanged)
- [ ] Exclusion patterns work (test: new `.crewchief/` worktrees not indexed)
- [ ] Search results no longer have duplicates (test: search returns <5 copies of same chunk)

**Verification Method:** Before/after worktree count, disk path validation, search quality tests

### Scope Definition

#### In Scope
- Detect stale worktrees (abs_path doesn't exist)
- Remove stale worktrees and their chunks
- Add exclusion patterns for `.crewchief/` paths
- Create cleanup command: `maproom db cleanup-stale`
- Add cleanup to regular maintenance

#### Out of Scope
- Automatic cleanup on every search (too expensive)
- Cleaning up other database tables
- Optimizing database size/vacuum
- Changing indexing behavior

#### Edge Cases
- **Temporarily unmounted disk**: Decision: Provide dry-run mode, require confirmation
- **Worktree created after cleanup**: Decision: Normal indexing handles it
- **Cascade delete orphans chunks**: Decision: Use database CASCADE on foreign keys

### Risk Assessment

| Risk | Impact on Agents | Mitigation |
|------|-----------------|------------|
| Accidentally delete valid worktrees | High - data loss | Dry-run mode, disk verification, user confirmation |
| Cleanup takes too long | Low - batch operation | Run as background job, show progress |
| Exclusion patterns too broad | Medium - miss valid code | Explicit whitelist, careful pattern design |

### Dependencies
- None - standalone cleanup operation

### Success Metrics
- Worktree count reduced by >90%
- All remaining worktrees have valid disk paths
- Search result duplication reduced by >90%
- Cleanup completes in <30 seconds
- Zero data loss for valid worktrees

---

## Project 4: Search Result Deduplication

**Pattern:** Capability Layer
**Size:** Small (1-2 weeks, 5-7 tickets)
**Priority:** 🟡 High - Improves search quality significantly

### Project Summary

Implement result deduplication in the search tool to prevent showing the same chunk multiple times from different worktrees. Currently, when the same code exists in 15 different worktree snapshots, all 15 appear in search results, burying the signal in noise.

This project will group results by (relpath, symbol_name, start_line) and return only the highest-scoring instance of each unique chunk. This works in conjunction with index cleanup but provides immediate benefit even before cleanup.

### Core Criteria Assessment

#### ✅ Interface Stability
**External Interfaces:**
- Database: `maproom.chunks` table (stable, no schema changes)
- MCP API: `search` tool (stable, internal deduplication)
- Search algorithm: Extends existing logic

**Stability Commitment:** ✅ Confirmed
**Risk Areas:** None - post-processing of results

#### ✅ Context Coherence
**Domain Concepts:** 8 concepts
- Result grouping
- Duplicate detection
- Chunk identity (relpath + symbol + line)
- Score comparison
- Worktree prioritization
- Result filtering
- Group-by logic
- Post-processing pipeline

**Core Modules:**
- `crates/maproom/src/search/deduplication.rs` - New module
- `crates/maproom/src/search/mod.rs` - Integration point

**Context Size:** ~150 lines of code

#### ✅ Testable Completion
**Success Criteria:**
- [ ] Same chunk appears only once in results (test: search returns max 1 result per unique chunk)
- [ ] Highest-scoring instance selected (test: main worktree result chosen over variant worktree)
- [ ] Deduplication preserves ranking (test: top result is still most relevant)
- [ ] Works across all search modes (test: FTS, vector, hybrid all deduplicate)
- [ ] Performance impact acceptable (test: search latency increase <10%)

**Verification Method:** Search test suite with known duplicate chunks

### Scope Definition

#### In Scope
- Group search results by (relpath, symbol_name, start_line)
- Select highest-scoring instance per group
- Prefer "main" worktree when scores are equal
- Apply deduplication to all search modes
- Add configuration flag to enable/disable

#### Out of Scope
- Deduplication at database level (too complex)
- Changing how chunks are stored
- Merging duplicate chunks in database
- Cross-repo deduplication

#### Edge Cases
- **Identical score ties**: Decision: Prefer "main" worktree, then alphabetically
- **Different line numbers for same symbol**: Decision: Treat as different chunks (could be different versions)
- **Empty symbol_name**: Decision: Use full relpath for grouping

### Risk Assessment

| Risk | Impact on Agents | Mitigation |
|------|-----------------|------------|
| Wrong chunk selected as representative | Medium - user sees wrong version | Clear prioritization rules, main > others |
| Performance degradation | Low - search already slow | Benchmark before/after, optimize grouping |
| Over-deduplication hides variants | Low - variants usually unwanted | Configuration flag to disable |

### Dependencies
- **Suggested after:** Project 3 (Index Cleanup) - reduces duplicates at source
- **Can run independently:** Yes, provides value even without cleanup

### Success Metrics
- Search result duplication reduced by >90%
- Average results per search drops from 15 duplicates to 1-2
- Search latency increase <10%
- User sees most relevant version of each chunk
- Configuration flag works correctly

---

## Project 5: File Type Filtering Implementation

**Pattern:** Capability Layer
**Size:** XS (3-5 days, 3-4 tickets)
**Priority:** 🟢 Medium - Quality of life improvement

### Project Summary

Implement the `file_type` filter that's already documented in the MCP API but doesn't actually work. Users should be able to filter search results by file extension (e.g., `filters: {file_type: "rs"}`) to get only Rust files, or TypeScript, Python, etc.

This is a simple feature addition that's already designed and documented, just not implemented. It will significantly improve search precision when looking for code in a specific language.

### Core Criteria Assessment

#### ✅ Interface Stability
**External Interfaces:**
- MCP API: `search` tool (filter parameter already exists, just not implemented)
- Database: `maproom.chunks` table (has file path, can extract extension)

**Stability Commitment:** ✅ Confirmed
**Risk Areas:** None - completing existing interface

#### ✅ Context Coherence
**Domain Concepts:** 5 concepts
- File extension extraction
- Filter parameter parsing
- SQL WHERE clause generation
- Result filtering
- Filter validation

**Core Modules:**
- `crates/maproom/src/search/mod.rs` - Add filter logic
- `packages/maproom-mcp/src/tools/search.ts` - Parameter validation

**Context Size:** ~50 lines of code

#### ✅ Testable Completion
**Success Criteria:**
- [ ] Filter by extension works (test: `file_type: "rs"` returns only .rs files)
- [ ] Multiple extensions work (test: `file_type: "ts,tsx,js"` returns TypeScript/JavaScript)
- [ ] Invalid extensions handled (test: `file_type: "invalid"` returns clear error or no results)
- [ ] Filter combines with other parameters (test: `file_type: "rs"` + `query: "validate"` works)
- [ ] Case insensitive matching (test: `file_type: "RS"` works same as "rs")

**Verification Method:** Integration tests with known file types

### Scope Definition

#### In Scope
- Extract file extension from chunk file path
- Add WHERE clause for file extension filtering
- Support comma-separated extension lists
- Case-insensitive extension matching
- Integration with existing search logic

#### Out of Scope
- Complex pattern matching (keep it simple: exact extension match)
- File type detection by content (just use extension)
- Adding new file types to indexer
- MIME type filtering

#### Edge Cases
- **Multiple extensions**: "ts,tsx,js" - Decision: OR logic, match any
- **Extension with dot**: ".rs" vs "rs" - Decision: Accept both, normalize to without dot
- **No extension**: Files without extension - Decision: Empty string extension

### Risk Assessment

| Risk | Impact on Agents | Mitigation |
|------|-----------------|------------|
| Extension extraction fails for edge cases | Low - most files have standard extensions | Comprehensive test suite |
| Performance impact of additional WHERE clause | Very low - indexed column | Benchmark if needed |

### Dependencies
- None - standalone feature

### Success Metrics
- All file type filter tests pass
- Filter works with all search modes (FTS, vector, hybrid)
- Performance impact <5ms per search
- Users can filter to specific languages successfully

---

## Project Sequencing and Dependencies

### Recommended Implementation Order

```
Phase 1 (Critical Blockers - Parallel)
├─ Project 2: Open Tool Fix (3-5 days)
└─ Project 1: Search Exact Match (2-3 weeks)

Phase 2 (Quality Improvements - After Phase 1)
├─ Project 3: Index Cleanup (1-2 weeks)
└─ Project 5: File Type Filter (3-5 days)

Phase 3 (Final Polish - After Phase 2)
└─ Project 4: Result Deduplication (1-2 weeks)
```

### Why This Order?

1. **Projects 1 & 2 (Parallel)** - Both are critical blockers for basic functionality
   - Open tool fix is quick and unblocks file reading
   - Search exact match unblocks context tool usage
   - No dependencies between them

2. **Projects 3 & 5 (Parallel)** - Both improve search quality
   - Index cleanup reduces duplicate pollution
   - File type filter adds precision
   - Both benefit from exact match working (Project 1)

3. **Project 4 (Last)** - Deduplication is most effective after cleanup
   - Fewer duplicates to handle if cleanup runs first
   - Still provides value independently
   - Benefits from all other improvements being in place

### Dependency Graph

```
Project 1 (Exact Match)
    ├── Enables → Context Tool Usage
    └── Improves → Project 4 effectiveness

Project 2 (Open Fix)
    └── Enables → Reading search results

Project 3 (Cleanup)
    └── Reduces → Duplication in Project 4

Project 4 (Deduplication)
    ├── Depends on (optional) → Project 3
    └── Benefits from → Project 1

Project 5 (File Type Filter)
    └── Independent → No dependencies
```

---

## Quick Reference: Projects Summary

| Project | Size | Priority | Duration | Core Benefit |
|---------|------|----------|----------|------------|
| 1. Search Exact Match | Small | 🔴 Critical | 2-3 weeks | Unblocks context tool, makes search usable |
| 2. Open Tool Fix | XS | 🔴 Critical | 3-5 days | Enables file reading from MCP |
| 3. Index Cleanup | Small | 🟡 High | 1-2 weeks | Removes 90%+ of duplicate results |
| 4. Result Deduplication | Small | 🟡 High | 1-2 weeks | Eliminates remaining duplicates |
| 5. File Type Filter | XS | 🟢 Medium | 3-5 days | Adds language filtering precision |

**Total Estimated Duration:** 6-9 weeks (with parallel work)
**Total Estimated Tickets:** 25-35 tickets across all projects

---

## Validation Against Framework

### All Projects Meet Core Criteria

✅ **Interface Stability**
- All projects work with existing stable interfaces
- No external API changes required
- Database schema remains unchanged
- MCP tool signatures stable

✅ **Context Coherence**
- All projects <20 domain concepts
- All explainable in <500 words
- Tight architectural clustering
- Single area of codebase per project

✅ **Testable Completion**
- All have specific, measurable success criteria
- All have automated test plans
- All have binary pass/fail verification
- No subjective requirements

### Decision Test Results

**New Agent Test:** ✅ Pass
Any agent could understand any of these projects from the descriptions.

**Interface Churn Test:** ✅ Pass
No external interfaces will change during any project.

**Context Overflow Test:** ✅ Pass
Each project fits easily in a single conversation.

**Script Completion Test:** ✅ Pass
Every project has automated verification criteria.

---

## Conclusion

The original monolithic failure report has been decomposed into **5 independent, well-scoped projects** that each meet the Stable Context Triangle criteria. Each project:

- Has stable interfaces
- Fits within agent memory
- Has testable completion criteria
- Delivers independent value
- Can be executed by agents with confidence

The sequencing provides clear priorities while allowing parallel execution where beneficial. Critical blockers (exact match search, open tool fix) come first, followed by quality improvements that build on the foundation.

**Next Steps:**
1. Review and approve project boundaries
2. Create project directories in `.agents/projects/`
3. Run `/create-project` for each approved project
4. Begin with Phase 1 projects (parallel execution)

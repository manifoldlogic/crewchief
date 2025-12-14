# Project: Search Result Fields Bug

**Slug:** SRCHFIX
**Status:** Planning Complete
**Created:** 2025-12-09

## Summary

Fix critical bug in Maproom MCP search results where three essential metadata fields (chunk_id, symbol_name, kind) are always missing or defaulted to zero/empty values. This bug blocks context retrieval functionality and degrades search result quality.

## Problem Statement

Search results from the Maproom MCP server are missing critical metadata:

1. **chunk_id always 0**: Prevents context retrieval (context tool requires valid chunk_id)
2. **symbol_name always empty**: Hides function/class/method names in search results
3. **kind always empty**: Loses symbol type information (function, class, method, etc.)

### Root Cause

Incomplete type synchronization during daemon migration:
- Rust daemon has all data in SearchHit struct but omits chunk_id from JSON serialization
- TypeScript interface expects chunk_index (wrong name) but omits symbol_name and kind
- Mapping code hardcodes empty strings and uses empty Map for chunk_id lookup

### Impact

- **High**: Context retrieval completely broken (chunk_id=0 is invalid)
- **Medium**: Search results lack semantic information (no function names or types)
- **Low**: Developer confusion from misleading comments about "Phase 2 enhancements"

## Proposed Solution

Complete the type synchronization by:

1. **Rust daemon**: Add `"chunk_id": hit.chunk_id` to JSON serialization (1 line change)
2. **TypeScript interface**: Rename chunk_index → chunk_id, add symbol_name and kind fields
3. **Mapping code**: Use actual daemon values instead of hardcoded defaults
4. **Cleanup**: Remove obsolete chunkIdMap and misleading comments

This is a straightforward data plumbing fix - no architecture changes, no new dependencies.

## Relevant Agents

**Planning:**
- project-planner (this document)

**Implementation:**
- rust-expert (update daemon JSON serialization)
- typescript-expert (update interfaces and mapping code)
- unit-test-runner (run existing tests)
- verify-ticket (validate acceptance criteria)
- commit-ticket (commit changes)

**Testing:**
- Integration tests for field validation
- Manual testing for context retrieval

## Planning Documents

All planning documents are complete:

- [analysis.md](planning/analysis.md) - Problem analysis and root cause
- [architecture.md](planning/architecture.md) - Solution design (data plumbing fix)
- [plan.md](planning/plan.md) - Execution plan (2 phases: fix, validate)
- [quality-strategy.md](planning/quality-strategy.md) - Testing approach (critical paths only)
- [security-review.md](planning/security-review.md) - Security assessment (no concerns)

## Key Decisions

1. **Field naming**: Use `chunk_id` everywhere (Rust convention, more accurate than "chunk_index")
2. **Null handling**: Preserve Option<String> semantics (symbol_name can be null for anonymous chunks)
3. **Backward compatibility**: Additive changes only (won't break existing clients)
4. **Testing scope**: Focus on critical paths (serialization, mapping, context retrieval)

## Tickets

Tickets will be created after project review via `/review-project SRCHFIX`.

## Next Steps

1. Run `/review-project SRCHFIX` to validate planning
2. Address any review findings
3. Run `/create-project-tickets SRCHFIX` to generate implementation tickets
4. Execute tickets via `/work-on-project SRCHFIX` or individual `/single-ticket` commands

## Custom Agents Assessment

**Recommendation**: Custom specialized agents are NOT needed for this project.

**Rationale**: This is a straightforward bug fix with well-understood scope. General-purpose agents (rust-expert, typescript-expert) are sufficient. The complexity is minimal (type synchronization), with no specialized domain expertise required.

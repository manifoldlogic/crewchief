# Ticket: SEMRANK-0002: Validate Baseline FTS Implementation

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (validation/documentation task, no code tests required)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- verify-ticket
- commit-ticket

## Summary
Test search tool with known queries, document current ranking behavior (tests vs implementations), and verify MCP protocol integration.

## Background
The search tool now exists (SEMRANK-0001), and must be validated before proceeding with semantic ranking enhancements. We need baseline metrics to measure improvement after implementing semantic ranking in Phase 2.

Current FTS issue: tests rank higher than implementations due to term frequency, making it difficult to find actual entry points. This baseline documentation will be used in SEMRANK-1005 for comparison to demonstrate improvement.

This ticket implements Phase 0 (MCP Tool Creation & Baseline) from the SEMRANK project plan.

## Acceptance Criteria
- [ ] Search tool successfully returns results for 10+ test queries
- [ ] Current ranking behavior documented: For query "validate_provider", record top 3 results (kind, relpath, score)
- [ ] Known failure cases confirmed: Tests ranking above implementations for exact function name searches
- [ ] MCP protocol integration verified: Tool callable via MCP client, proper error handling
- [ ] Debug mode confirmed functional: Returns score breakdown (base_fts score visible)
- [ ] Documentation created: `baseline-behavior.md` with current FTS ranking examples

## Technical Requirements
- Use test corpus from existing maproom codebase (don't wait for SEMRANK-1003)
- Test queries: Mix of exact function names and concept searches
- Record examples of problematic rankings (test > implementation)
- Verify Rust FTS formula: `ts_rank_cd() + 0.2 exact bonus`
- Confirm exact bonus uses substring match: `ILIKE '%query%'`

## Implementation Notes
Create `/packages/maproom-mcp/docs/baseline-behavior.md` with the following structure:

**Example queries to test:**
- "authenticate" (exact function name)
- "validate_provider" (known failure case)
- "database connection" (concept search)
- "error handling" (concept search)
- "spawn" (function name)
- "config" (common term)
- "test" (meta-search)
- "mcp tool" (multi-word concept)
- "search implementation" (concept)
- "chunk" (data structure)

**Documentation Format:**
For each query, record:
1. Query string
2. Query type (exact function name / concept search)
3. Top 3 results with:
   - `kind` (function/class/interface/test/etc)
   - `relpath` (file path)
   - `score` (FTS score)
4. Notes on ranking quality (e.g., "test ranked #1, implementation ranked #3")

**Example Entry:**
```markdown
### Query: "validate_provider"
**Type**: Exact function name

**Results:**
1. kind: test, relpath: src/auth/__tests__/provider.test.ts, score: 0.92
2. kind: test, relpath: src/auth/__tests__/integration.test.ts, score: 0.78
3. kind: function, relpath: src/auth/provider.ts, score: 0.65

**Issues**: Implementation ranked #3 below two test files. This is the problematic behavior we aim to fix.
```

**Validation Checklist:**
- Confirm tool appears in MCP server tool list
- Test with/without optional parameters (repo_filter, worktree)
- Verify limit parameter works correctly
- Verify debug mode returns score breakdown
- Test error handling (empty query, invalid repo, etc.)

## Dependencies
- **SEMRANK-0001** - Search tool must exist and be functional

## Risk Assessment
- **Risk**: Baseline may show severe ranking issues beyond expected test/implementation inversion
  - **Mitigation**: Document honestly - this justifies the project and informs Phase 2 design
- **Risk**: MCP integration bugs discovered during validation
  - **Mitigation**: Fix immediately before proceeding to Phase 1 (add issues to SEMRANK-0001 if needed)
- **Risk**: Insufficient test corpus in existing codebase
  - **Mitigation**: Use crewchief codebase itself as corpus, which has diverse code patterns

## Files/Packages Affected
- **Create**: `/packages/maproom-mcp/docs/baseline-behavior.md`
- **Reference**:
  - `/packages/maproom-mcp/src/tools/search.ts` (tool to validate)
  - `/crates/maproom/src/search/fts.rs` (FTS formula to verify)

# Execution Plan: Semantic Entry Point Ranking

## Project Overview

**Goal:** Improve FTS ranking to return correct entry points (implementations, not tests/docs) for effective context() traversal.

**Approach:** Enhance PostgreSQL FTS scoring with semantic multipliers based on chunk kind and symbol name matching.

**Timeline:** 3.5-4.5 weeks
**Ticket Estimate:** 20 tickets

## Success Criteria

- [ ] Search for exact function name returns implementation as #1 result (not test)
- [ ] Implementation ranks higher than test with same symbol name
- [ ] Implementation ranks higher than documentation
- [ ] Case-insensitive exact match works
- [ ] Null symbol_name chunks handled gracefully
- [ ] Debug mode returns score breakdown
- [ ] p95 latency increase <10%
- [ ] All existing search tests pass

## Execution Phases

### Phase 0: MCP Tool Creation & Baseline (2-3 days)

**Objective:** Create missing TypeScript search MCP tool that wraps Rust FTS implementation.

**Context:** Critical blocker identified in project review - `/packages/maproom-mcp/src/tools/search.ts` does not exist. Phase 2 assumes this tool exists for modification. Must create baseline tool first before semantic enhancements.

**Tickets:**

1. **SEMRANK-0001: Create TypeScript Search MCP Tool**
   - Create `/packages/maproom-mcp/src/tools/search.ts`
   - **ARCHITECTURE:** Call Rust binary via subprocess (avoid SQL duplication in TypeScript)
   - Follow upsert.ts pattern: spawn binary, parse NDJSON output
   - Expose parameters: query, repo_filter, worktree, limit, debug
   - Return: chunk_id, symbol_name, kind, relpath, preview, score
   - **Eliminates:** Duplicate FTS SQL in index.ts handleSearch() (lines 684-850)
   - **Benefits:** Single source of truth, Phase 2 changes only need Rust updates
   - **Agent:** general-purpose
   - **Acceptance:** Search tool calls Rust binary via subprocess, returns NDJSON results
   - **Estimate:** 2 days

2. **SEMRANK-0002: Validate Baseline FTS Implementation**
   - Test search tool with known queries
   - Document current ranking behavior (tests vs implementations)
   - Verify integration with MCP protocol
   - Confirm debug mode returns score details
   - **Agent:** verify-ticket
   - **Acceptance:** Search tool functional, baseline behavior documented
   - **Estimate:** 1 day

**Phase 0 Deliverables:**
- Functional TypeScript search MCP tool
- Baseline FTS behavior documented
- MCP protocol integration validated
- Ready for semantic enhancement in Phase 2

---

### Phase 1: Foundation & Test Infrastructure (3-4 days)

**Objective:** Establish test corpus and baseline measurements before making changes.

**Tickets:**

3. **SEMRANK-1003: Create Test Corpus Repository**
   - Create sample repository with known structure (src/, tests/, docs/)
   - Include examples in 3 languages: Rust, TypeScript, Python
   - Structure: 5 functions + 3 tests + 2 docs per language = 30 chunks minimum
   - File path examples: `src/auth/validate.rs`, `tests/auth_test.rs`, `docs/auth_guide.md`
   - **Scope Constraint:** 50 chunks maximum, 1 day time box
   - Representative samples, not full applications
   - **Fallback:** Use existing maproom codebase subset if creation exceeds time
   - **Agent:** general-purpose
   - **Acceptance:** Test repo with 30-50 chunks across 3 languages, varied kinds
   - **Estimate:** 1 day

4. **SEMRANK-1004: Index Test Corpus in Maproom**
   - Run maproom scan on test corpus
   - Validate chunk metadata (kind, symbol_name) extracted correctly
   - Verify relationships table populated (if available)
   - **Agent:** rust-indexer-engineer
   - **Acceptance:** Test corpus fully indexed, metadata correct
   - **Estimate:** 1 day

5. **SEMRANK-1005: Baseline Search Quality Metrics**
   - Document current search behavior (what ranks #1 for known queries)
   - Measure latency baselines (p50, p95, p99)
   - Create "golden dataset" of 20 representative queries across languages
   - Specify latency measurement: p50, p95, p99 over 100 runs per query
   - Document baseline format: CSV with query, latency_p50, latency_p95, top_3_kinds
   - Create benchmark script for reproducibility
   - **Agent:** database-engineer
   - **Acceptance:** Baseline metrics documented in CSV, golden dataset created, benchmark script functional
   - **Estimate:** 1 day

6. **SEMRANK-1006: Integration Test Framework Setup**
   - Set up test harness for search quality tests
   - Database seeding/teardown for tests
   - Benchmark infrastructure for latency tracking
   - **Agent:** integration-tester
   - **Acceptance:** Test framework runs, can execute search tests
   - **Estimate:** 1 day

**Phase 1 Deliverables:**
- Test corpus repository
- Indexed test data in database
- Baseline metrics documented
- Test framework operational

---

### Phase 2: Core Ranking Implementation (5-7 days)

**Objective:** Implement kind-based and exact-match multipliers in SQL query.

**Tickets:**

7. **SEMRANK-2003: Implement Kind-Based Multiplier in SQL**
   - **PREREQUISITE:** Verify actual kind enum values from database
   - Query: `SELECT DISTINCT kind FROM maproom.chunks LIMIT 20`
   - Add CASE statement for kind multiplier using ACTUAL enum values (not assumed 'function')
   - Map chunk kinds to multiplier values based on database enum
   - Update SELECT query to compute kind_mult column
   - **Agent:** database-engineer
   - **Acceptance:** SQL query includes kind multiplier using correct enum values, compiles without error, verified against database schema
   - **Estimate:** 2 days

8. **SEMRANK-2004a: Implement Exact Match SQL Logic**
   - Add CASE statement for exact match detection in SQL
   - Handle case-insensitive comparison (LOWER)
   - Apply 3.0× multiplier when LOWER(symbol_name) = LOWER($normalized_query)
   - **Agent:** database-engineer
   - **Acceptance:** SQL exact match CASE statement functional, applies 3.0× boost correctly
   - **Estimate:** 1 day

9. **SEMRANK-2004b: Implement Query Normalization (TypeScript)**
   - Create normalizeForExactMatch() function in TypeScript
   - Handle camelCase → snake_case conversion
   - Handle kebab-case → snake_case conversion
   - Handle acronyms: XMLParser → xml_parser, HTTPSHandler → https_handler
   - Handle consecutive capitals: validateHTTPRequest → validate_http_request
   - Handle mixed case with numbers: Base64Encoder → base64_encoder
   - Pass normalized query to SQL as parameter
   - Remove old exact bonus logic from Rust (fts.rs:92-95) if still present
   - **Agent:** general-purpose
   - **Acceptance:** Normalization handles all edge cases, old bonus logic removed, verified no conflicts
   - **Estimate:** 1 day

10. **SEMRANK-2005: Combine Multipliers into Final Score**
    - Multiply base FTS score by kind_mult and exact_mult
    - Update ORDER BY clause to use final_score
    - Preserve base_score for debug mode
    - **Agent:** database-engineer
    - **Acceptance:** final_score = base × kind × exact, results ordered correctly
    - **Estimate:** 1 day

11. **SEMRANK-2006: Add Debug Mode Score Breakdown**
    - Return score_breakdown object when debug=true
    - Include: base_fts, kind_multiplier, exact_match_multiplier, final
    - Update MCP tool response schema
    - Add permission check: debug mode requires admin permissions (or document as future enhancement if no auth system)
    - **Agent:** database-engineer
    - **Acceptance:** Debug mode returns detailed score breakdown, permission check implemented or documented as future work
    - **Estimate:** 1 day

12. **SEMRANK-2007: Handle Edge Cases (Null, Unknown)**
    - Null symbol_name → exact_mult = 1.0
    - Unknown kind → kind_mult = 1.0
    - Empty query → graceful error handling
    - **Agent:** database-engineer
    - **Acceptance:** Edge cases handled without crashing
    - **Estimate:** 1 day

**Phase 2 Deliverables:**
- Kind-based multiplier implemented
- Exact match multiplier implemented
- Combined scoring formula working
- Debug mode functional
- Edge cases handled

---

### Phase 3: Testing & Validation (4-5 days)

**Objective:** Validate ranking improvements and performance.

**Tickets:**

13. **SEMRANK-3003: Integration Tests for Ranking Correctness**
    - Test: Implementation ranks above test
    - Test: Implementation ranks above docs
    - Test: Exact match returns correct chunk
    - Test: Case-insensitive matching works
    - **Agent:** integration-tester
    - **Acceptance:** All ranking correctness tests pass
    - **Estimate:** 2 days

14. **SEMRANK-3004: Edge Case Testing**
    - Test: Null symbol_name chunks
    - Test: Unknown kind chunks
    - Test: Multi-word queries
    - Test: Special characters in query
    - Test: Acronym handling (XMLParser, HTTPSHandler)
    - Test: Consecutive capitals (validateHTTPRequest)
    - **Agent:** integration-tester
    - **Acceptance:** All edge case tests pass
    - **Estimate:** 1 day

15. **SEMRANK-3005: Performance Benchmarks**
    - Benchmark latency vs baseline (from SEMRANK-1005)
    - Test small corpus (1K chunks)
    - Test medium corpus (100K chunks)
    - Test concurrent load (100 queries)
    - **Agent:** integration-tester
    - **Acceptance:** p95 latency increase <10%, benchmarks pass
    - **Estimate:** 1 day

16. **SEMRANK-3006: Regression Testing**
    - Run existing search integration tests
    - Verify all tests still pass
    - Document any behavioral changes (intentional re-ranking)
    - **Agent:** verify-ticket
    - **Acceptance:** All existing tests pass or changes documented
    - **Estimate:** 1 day

**Phase 3 Deliverables:**
- Integration test suite passing
- Edge case tests passing
- Performance benchmarks passing
- Regression tests passing
- No degradation of existing functionality

---

### Phase 4: Documentation & Deployment (2-3 days)

**Objective:** Document changes, prepare for deployment, create rollback plan.

**Tickets:**

17. **SEMRANK-4003: Update Search Documentation**
    - Document kind multiplier values and rationale
    - Document exact match behavior and normalization algorithm
    - Update MCP tool documentation (debug mode, permissions)
    - Add examples to README
    - Document multiplier tuning criteria (Top-1 >85%, Avg rank <3)
    - Include metrics collection plan for post-deployment monitoring (4 weeks)
    - **Agent:** general-purpose
    - **Acceptance:** Documentation updated, examples clear, tuning criteria defined
    - **Estimate:** 1 day

18. **SEMRANK-4004: Create Deployment Runbook**
    - Document deployment steps
    - Create rollback procedure (revert SQL query)
    - List monitoring metrics to watch (Top-1 implementation rate, avg rank, latency)
    - Define success/failure criteria
    - Document migration from current exact bonus (+0.2) to multiplier (3.0×)
    - **Agent:** general-purpose
    - **Acceptance:** Runbook complete, rollback tested, migration documented
    - **Estimate:** 1 day

19. **SEMRANK-4005: CI/CD Integration**
    - Add integration tests to GitHub Actions
    - Add performance benchmarks to CI
    - Configure test failure thresholds
    - **Agent:** github-actions-specialist
    - **Acceptance:** Tests run in CI, failures block merge
    - **Estimate:** 1 day

**Phase 4 Deliverables:**
- Documentation updated
- Deployment runbook ready
- Rollback plan tested
- CI/CD integrated

---

### Phase 5: Verification & Commit (1 day)

**Objective:** Final verification and commit.

**Tickets:**

20. **SEMRANK-5003: Final Verification**
    - Run complete test suite
    - Verify all acceptance criteria met
    - Smoke test on real codebase (maproom itself)
    - **Agent:** verify-ticket
    - **Acceptance:** All criteria met, ready for commit
    - **Estimate:** 0.5 days

21. **SEMRANK-5004: Commit Changes**
    - Create conventional commit
    - Reference all tickets
    - Include performance metrics in commit message
    - **Agent:** commit-ticket
    - **Acceptance:** Changes committed with proper message
    - **Estimate:** 0.5 days

**Phase 5 Deliverables:**
- All changes verified
- Code committed to repository
- Ready for deployment

---

## Agent Assignments

| Phase | Primary Agent | Supporting Agents |
|-------|--------------|-------------------|
| Phase 0: MCP Tool Creation | general-purpose | verify-ticket |
| Phase 1: Foundation | general-purpose, rust-indexer-engineer | database-engineer, integration-tester |
| Phase 2: Implementation | database-engineer | general-purpose |
| Phase 3: Testing | integration-tester | verify-ticket |
| Phase 4: Documentation | general-purpose | github-actions-specialist |
| Phase 5: Verification | verify-ticket | commit-ticket |

## Risk Mitigation

### Risk: Breaking Existing Searches

**Mitigation:**
- Run full regression suite in Phase 3
- Feature flag for gradual rollout (future enhancement)
- Easy rollback via SQL query revert

**Contingency:**
- Revert to old SQL query immediately
- Investigate failures in staging environment
- Adjust multipliers if needed

### Risk: Performance Degradation

**Mitigation:**
- Benchmark in Phase 3 before merge
- Set hard latency SLOs in CI
- Monitor query plans (EXPLAIN ANALYZE)

**Contingency:**
- Profile slow queries
- Simplify CASE logic if needed
- Add database indices (unlikely to be necessary)

### Risk: Multipliers Poorly Tuned

**Mitigation:**
- Start with research-based values
- Monitor metrics after deployment
- Easy to adjust (just SQL constants)

**Contingency:**
- Quick-fix deployment with adjusted multipliers
- Expose configuration for tuning (future)

## Success Metrics

### Quantitative

- **Top-1 Accuracy:** >90% of exact function searches return implementation as #1
- **Implementation Rank:** Average rank of implementations <3 (top 3 results)
- **Latency:** p95 <200ms (no significant regression)
- **Test Pass Rate:** 100% of integration tests pass

### Qualitative

- Search results "feel correct" to developers
- Reduced need to scroll past tests/docs
- Effective starting points for context() traversal
- Positive user feedback on search quality

## Deployment Plan

### Pre-Deployment Checklist

- [ ] All tickets completed and verified
- [ ] Integration tests passing in CI
- [ ] Performance benchmarks within SLOs
- [ ] Documentation updated
- [ ] Rollback plan tested
- [ ] Security review completed (see security-review.md)

### Deployment Steps

1. **Deploy to Staging:**
   - Run smoke tests on staging environment
   - Verify metrics within acceptable ranges
   - Test rollback procedure

2. **Deploy to Production:**
   - Deploy during low-traffic window
   - Monitor latency and error rates
   - Watch for anomalies in first 24 hours

3. **Post-Deployment:**
   - Collect search quality metrics
   - Monitor user feedback
   - Plan multiplier tuning if needed (2-4 weeks post-launch)

### Rollback Procedure

If critical issues arise:

1. **Immediate Rollback:**
   ```sql
   -- Revert to old query (no multipliers)
   SELECT ..., ts_rank_cd(...) AS score
   FROM maproom.chunks
   WHERE ts_doc @@ to_tsquery(...)
   ORDER BY score DESC;
   ```

2. **Notify Team:**
   - Log incident in tracking system
   - Notify stakeholders of rollback
   - Schedule post-mortem

3. **Investigate:**
   - Analyze logs and metrics
   - Identify root cause
   - Fix in development environment
   - Re-deploy when ready

## Future Enhancements (Out of Scope)

These are intentionally excluded from MVP but noted for future consideration:

1. **Configurable Multipliers:**
   - Expose kind_multipliers as configuration
   - Allow tuning without code changes
   - **Effort:** 1-2 weeks

2. **Graph Signal Integration:**
   - Incorporate PageRank from relationships table
   - Boost high-centrality chunks
   - **Effort:** 2-3 weeks

3. **Learning to Rank:**
   - Collect click data on search results
   - Train ML model for optimal multipliers
   - **Effort:** 4-6 weeks

4. **Query Intent Classification:**
   - Detect "navigational" vs "exploratory" queries
   - Apply different ranking strategies
   - **Effort:** 3-4 weeks

5. **Personalized Ranking:**
   - User-specific kind preferences
   - Language-specific boosts (user's primary language)
   - **Effort:** 2-3 weeks

## Timeline Summary

| Phase | Duration | Key Deliverables |
|-------|----------|------------------|
| Phase 0: MCP Tool Creation | 2-3 days | TypeScript search tool, baseline validation |
| Phase 1: Foundation | 3-4 days | Test corpus, baseline metrics, test framework |
| Phase 2: Implementation | 6-8 days | Kind multiplier, exact match (SQL + TypeScript), combined scoring |
| Phase 3: Testing | 4-5 days | Integration tests, benchmarks, regression tests |
| Phase 4: Documentation | 2-3 days | Docs, runbook, CI integration |
| Phase 5: Verification | 1 day | Final checks, commit |
| **Total** | **18-24 days** | **3.5-4.5 weeks** |

## Ticket Dependency Graph

```
Phase 0 (Sequential):
├─ 0001: Create Search Tool ──┐
└─ 0002: Validate Baseline ───┴─> Phase 1

Phase 1 (Parallel):
├─ 1003: Test Corpus ────────┐
├─ 1004: Index Corpus ────────┤
├─ 1005: Baseline Metrics ────┼─> Phase 2
└─ 1006: Test Framework ──────┘

Phase 2 (Mostly Sequential):
├─ 2003: Kind Multiplier ─────────┐
├─ 2004a: Exact Match SQL ────────┤
├─ 2004b: Query Normalization ────┤
├─ 2005: Combined Score ──────────┼─> Phase 3
├─ 2006: Debug Mode ──────────────┤
└─ 2007: Edge Cases ──────────────┘

Phase 3 (Parallel):
├─ 3003: Ranking Tests ───────┐
├─ 3004: Edge Case Tests ─────┤
├─ 3005: Benchmarks ──────────┼─> Phase 4
└─ 3006: Regression Tests ────┘

Phase 4 (Parallel):
├─ 4003: Documentation ───────┐
├─ 4004: Runbook ─────────────┼─> Phase 5
└─ 4005: CI Integration ──────┘

Phase 5 (Sequential):
├─ 5003: Verification ────────┐
└─ 5004: Commit ──────────────┘
```

## Conclusion

This plan provides a systematic approach to implementing semantic entry point ranking:

0. **Prerequisites first:** Create missing MCP search tool before enhancements
1. **Foundation second:** Test infrastructure and baseline metrics
2. **Incremental implementation:** Build up complexity step by step (kind → exact match → combined)
3. **Rigorous testing:** Validate correctness and performance before merge
4. **Safe deployment:** Runbook, rollback plan, monitoring

**Estimated effort:** 18-24 working days (3.5-4.5 weeks)
**Confidence level:** High (critical blocker identified and resolved, well-scoped, clear requirements)

**Ready to execute:** All planning documents complete, Phase 0 added to resolve missing search tool, agent assignments clear, success criteria defined.

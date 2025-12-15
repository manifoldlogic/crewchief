# Plan: Relationship-Aware Search Ranking

## Overview

Phased execution plan to enhance graph-based search ranking with quality-weighted edge scoring. **Prerequisites must complete before Phase 1 implementation** to validate assumptions and de-risk performance concerns.

## ✅ BLOCKER RESOLVED (2025-12-14)

**Original Blocker:** Empty `chunk_edges` table - edge extraction not implemented

**Resolution:** EDGEEXT project completed
- TypeScript/JavaScript edge extraction (92.86% precision) ✓
- Rust edge extraction ✓
- Database now populated with `calls` edges

**Impact on Prerequisites:**
- Prerequisite 1 (Database Schema Validation): PARTIALLY COMPLETE
- Duration reduced from 5-7 days to 2-3 days
- Phase 1 scope simplified to `calls` edges only

See `planning/blocker-resolution.md` for details.

---

## Prerequisites (BLOCKING - Must Complete First)

**Duration:** 2-3 days (reduced from 5-7 days due to EDGEEXT completion)
**Owner:** Validation agent / database-engineer

These validation tasks are **BLOCKING** - Phase 1 implementation cannot begin until all prerequisites complete successfully.

### Prerequisite 1: Database Schema Validation - ✅ PARTIALLY COMPLETE

**Objective:** Verify that proposed SQL query can use assumed edge types and chunk fields.

**Status:** PARTIALLY COMPLETE via EDGEEXT project
- Edge types validated: `calls` edges exist and are populated ✓
- Edge types NOT available: `extends`, `implements` (architectural decision in EDGEEXT)
- `chunk_edges` table populated with call edges from TypeScript, JavaScript, Rust ✓

**Remaining Tasks:**
1. Query actual chunk `kind` values to confirm test detection patterns:
   ```sql
   SELECT DISTINCT kind FROM chunks LIMIT 100;
   ```
2. Sample chunk `relpath` patterns to validate test detection heuristic
3. **Document findings in architecture.md** (edge types already updated)

**Success Criteria:**
- [x] Confirmed: Edge data exists (EDGEEXT completed)
- [x] Confirmed: `calls` edges exist and are populated
- [ ] Confirmed: `chunks.relpath` and `files.relpath` accessible in SQL queries
- [ ] Documented: Actual `kind` values for test detection validation

**Duration:** 0.5-1 day (reduced from 1-2 days)

### Prerequisite 2: SQL Prototype & Performance Validation

**Objective:** Validate that quality-weighted SQL query meets performance budget (<30ms p95).

**Tasks:**
1. Create synthetic test database:
   - 100,000 chunks (mix of production and test code)
   - 500,000 edges (realistic distribution of edge types)
   - Populate with representative file paths and kinds
2. Implement proposed SQL query with quality weights
3. Benchmark query latency:
   - Cold cache: First query after database open
   - Warm cache: Subsequent queries
   - Measure p50, p95, p99 latencies
4. Run `EXPLAIN QUERY PLAN` to verify index usage
5. Test with different repository sizes (10K, 100K, 1M chunks)
6. **Document actual performance in architecture.md**

**Success Criteria:**
- [ ] Query latency <30ms p95 on 100K chunk database
- [ ] EXPLAIN shows index usage (no full table scans)
- [ ] Performance scales sub-linearly with database size

**Decision Point:** If latency >30ms p95:
- **Option A:** Optimize SQL (simplify CASE statements, add indexes)
- **Option B:** Pre-compute edge quality scores during indexing
- **Option C:** Defer feature to Phase 2 for more research

**Duration:** 2-3 days

### Prerequisite 3: Test Detection Validation

**Objective:** Measure accuracy of file path-based test detection heuristic.

**Tasks:**
1. Sample 200 chunks from real codebase (crewchief repository):
   - 100 known test chunks (from `/test/` directories)
   - 100 known production chunks (from `/src/` directories)
2. Apply test detection heuristic (file path patterns)
3. Calculate precision and recall:
   - True positives: Test chunks correctly identified
   - False positives: Production chunks misidentified as test
   - False negatives: Test chunks missed
4. Test edge cases:
   - Files with "test" in name but not tests
   - Test utilities in production code
5. **Document accuracy metrics in architecture.md**

**Success Criteria:**
- [ ] Precision ≥85% (few false positives)
- [ ] Recall ≥80% (few false negatives)
- [ ] Documented: Known false positive patterns and mitigation

**If criteria not met:** Tune file path patterns or add additional signals (e.g., file imports test frameworks)

**Duration:** 1 day

### Prerequisite 4: Config Integration Design Validation

**Objective:** Verify configuration loading and feature flag approach before implementation.

**Tasks:**
1. Review existing `SearchConfig` structure in `src/config/search_config.rs`
2. Design `graph_importance` field integration (Phase 2 feature, documented for future)
3. For Phase 1: Determine simplest feature flag approach:
   - **Option A:** Environment variable `MAPROOM_ENABLE_QUALITY_SCORING=true`
   - **Option B:** Boolean in existing config file `feature_flags.enable_quality_scoring`
   - **Option C:** Defer all config, hardcode flag to `false` initially
4. Write example config snippet and test deserialization
5. **Document config approach in architecture.md**

**Success Criteria:**
- [ ] Feature flag approach decided and documented
- [ ] Config loading path verified (or hardcoded approach confirmed)
- [ ] Backward compatibility verified (old configs still work)

**Duration:** 1 day

### Prerequisites Summary

| Task | Duration | Success Criteria | Status | Blocker If Failed? |
|------|----------|------------------|--------|-------------------|
| Schema Validation | ~~1-2 days~~ 0.5-1 day | Edge types confirmed | ✅ PARTIAL (calls exist) | YES - SQL won't work |
| SQL Performance | 1.5-2 days | <30ms p95 latency | ⏳ Pending | YES - exceeds budget |
| Test Detection | 1 day | ≥85% precision | ⏳ Pending | MODERATE - can tune |
| Config Design | 0.5-1 day | Approach decided | ⏳ Pending | NO - can hardcode |

**Total:** ~~5-7 days~~ **2-3 days** (reduced due to EDGEEXT completion)

**Changes from Original:**
- Schema validation now 50% complete (edge data exists)
- SQL performance can use real edge data (not synthetic)
- Test detection can validate on actual codebase
- Overall timeline reduced by ~3 days

**Checkpoint:** After prerequisites complete, review findings. If critical issues found (e.g., performance >50ms), consider alternative approaches or defer feature.

## Phases

### Phase 1: Core Implementation (MVP - Hardcoded Weights)

**Objective:** Implement quality-weighted graph scoring with minimal infrastructure. Prove algorithm works before building configuration system.

**Scope Reduction:** Phase 1 delivers working enhanced scoring with hardcoded weights and simple feature flag. Configuration infrastructure deferred to Phase 2.

**Deliverables:**
1. Modified SQL query in `calculate_graph_importance()` with quality weighting
2. Feature flag (environment variable or simple config boolean)
3. Backward compatibility (old SQL path preserved)
4. Unit tests for SQL query logic
5. Integration tests comparing old vs enhanced executor

**Detailed Tasks (2-8 hour granularity):**

#### Task 1.1: Modify Database Query (3-4 hours)
- **File:** `crates/maproom/src/db/sqlite/mod.rs`
- **Changes:**
  - Add `enable_quality: bool` parameter to `calculate_graph_importance()`
  - Implement quality-weighted SQL query (hardcoded weights)
  - Keep existing SQL as fallback when `enable_quality = false`
- **Acceptance:**
  - [ ] SQL compiles and returns results
  - [ ] Both old and new SQL paths work
  - [ ] Results differ (quality weights applied)

#### Task 1.2: Add Feature Flag Support (1-2 hours)
- **File:** `crates/maproom/src/config/search_config.rs` OR environment variable
- **Changes:**
  - Add boolean flag `enable_quality_scoring`
  - Default to `false` (safe rollout)
  - Document flag in configuration docs
- **Acceptance:**
  - [ ] Flag loads successfully
  - [ ] Flag = false → old behavior (verified)
  - [ ] Flag = true → new behavior (verified)

#### Task 1.3: Update Graph Executor (2-3 hours)
- **File:** `crates/maproom/src/search/graph.rs`
- **Changes:**
  - Update `execute()` signature to accept `Option<&SearchConfig>` (backward compatible)
  - Read feature flag from config (or environment variable)
  - Pass flag to `calculate_graph_importance()`
- **Acceptance:**
  - [ ] Executor calls database with correct flag value
  - [ ] Existing callers still work (None config = old behavior)
  - [ ] New callers can pass config

#### Task 1.4: Unit Tests for Quality SQL (2-3 hours)
- **File:** `crates/maproom/tests/graph_quality_tests.rs` (new)
- **Changes:**
  - Test: Production code edges score higher than test code edges
  - Test: Inheritance edges (extends/implements) score higher
  - Test: LOG scaling applied correctly
  - Test: Feature flag toggle works
- **Acceptance:**
  - [ ] All unit tests pass
  - [ ] Coverage >70% for modified code

#### Task 1.5: Integration Tests (2-3 hours)
- **File:** `crates/maproom/tests/integration/graph_executor_tests.rs`
- **Changes:**
  - Create test scenario (prod + test callers)
  - Compare scores: old vs enhanced
  - Verify: Prod-heavy chunk ranks higher
- **Acceptance:**
  - [ ] Integration tests pass
  - [ ] Behavior change validated

**Agent Assignments:**
- database-engineer: Task 1.1 (SQL query modification)
- rust-expert: Task 1.2, 1.3 (feature flag + executor)
- test-engineer: Task 1.4, 1.5 (unit + integration tests)

**Duration:** 3-5 days (parallelizable tasks)

**Success Criteria:**
- [ ] Enhanced SQL query returns quality-weighted scores
- [ ] Feature flag toggles between old/new implementation
- [ ] Backward compatibility preserved (existing callers work)
- [ ] Unit tests pass (SQL logic)
- [ ] Integration tests pass (executor behavior)

### Phase 1.5: Validation (Before Phase 2)

**Objective:** Validate that quality-weighted scoring improves ranking on test queries before full pipeline integration.

**Duration:** 2-3 days

**Tasks:**
1. **Baseline Measurement (1 day)**
   - Run 20 representative queries with `enable_quality=false`
   - Record top 5 results per query
   - Note which queries have architecturally important code NOT in top 3

2. **Enhanced Scoring Validation (1 day)**
   - Run same 20 queries with `enable_quality=true`
   - Compare top 5 results
   - Count: How many queries improved? How many degraded?

3. **Score Analysis (0.5 day)**
   - Compare score distributions (old vs enhanced)
   - Verify: Production code scores > test code scores
   - Check: No extreme score inflation (sanity check)

4. **Go/No-Go Decision (0.5 day)**
   - **Go:** ≥14/20 queries improved (70%), ≤2/20 degraded (10%)
   - **No-Go:** <12/20 improved OR >3/20 degraded → tune weights, re-test

**Success Criteria:**
- [ ] 70%+ of test queries show improvement (central code ranks higher)
- [ ] ≤10% of queries show degradation
- [ ] Score distributions validate quality weighting works

### Phase 2: Configuration & Pipeline Integration

**Objective:** Add YAML configuration for weights, integrate into search pipeline, prepare for production rollout.

**Scope:** Configuration infrastructure that was deferred from Phase 1.

**Deliverables:**
1. YAML configuration schema (`graph_importance` section)
2. `GraphImportanceConfig` and `EdgeQualityWeights` structs
3. Configuration loading and validation
4. Fusion weight override support
5. Search pipeline config propagation
6. Performance benchmarks on real codebases
7. Ranking quality evaluation (50+ queries)

**Detailed Tasks:**

#### Task 2.1: Configuration Schema (2-3 hours)
- Add `GraphImportanceConfig` to `SearchConfig`
- Define `EdgeQualityWeights` struct with all weight fields
- Implement `Default` trait with validated weights from Phase 1
- Add config validation (reject negative/extreme weights)

#### Task 2.2: SQL Parameterization (3-4 hours)
- Replace hardcoded weights in SQL with parameters
- Update `calculate_graph_importance()` to accept weights struct
- Pass weights from config through executor to database layer

#### Task 2.3: Pipeline Integration (3-4 hours)
- Load config in search pipeline initialization
- Pass config to `GraphExecutor::execute()`
- Implement fusion weight override logic

#### Task 2.4: Performance Benchmarking (1 day)
- Benchmark on crewchief repository (real data)
- Measure: p50, p95, p99 latencies
- Compare: Old vs enhanced executor overhead
- Validate: <35ms p95 target

#### Task 2.5: Ranking Quality Evaluation (2 days)
- Curate 50 representative test queries
- Manual evaluation: Top 3 results quality (old vs enhanced)
- Quantitative: Count improved vs degraded queries
- Target: ≥32/50 (64%) improved, ≤2/50 (4%) degraded

**Agent Assignments:**
- rust-expert: Task 2.1, 2.2 (config + SQL parameterization)
- search-engineer: Task 2.3 (pipeline integration)
- performance-engineer: Task 2.4 (benchmarking)
- test-engineer: Task 2.5 (ranking evaluation)

**Duration:** 1.5-2 weeks

**Success Criteria:**
- [ ] Configuration loads and validates successfully
- [ ] Weights configurable via YAML (verified by changing config)
- [ ] Performance within budget (<35ms p95 graph executor)
- [ ] Ranking quality meets targets (≥64% improved, ≤4% degraded)
- [ ] Fusion weight override works correctly

### Phase 3: Validation & Documentation

**Objective:** Validate ranking improvements, document configuration, prepare for rollout.

**Deliverables:**
- Ranking quality evaluation (50+ test queries)
- Configuration tuning guide
- User-facing documentation
- Monitoring/metrics setup
- Rollout plan

**Agent Assignments:**
- technical-writer: Configuration guide, tuning documentation
- search-engineer: Ranking quality evaluation, weight tuning
- ops-engineer: Metrics/monitoring setup, rollout plan
- test-engineer: Edge case testing

**Duration:** 1 week

**Success Criteria:**
- [ ] Ranking quality validated (important code ranks in top 3)
- [ ] Configuration documented with examples
- [ ] Monitoring dashboards created
- [ ] Rollout plan approved
- [ ] Edge case tests pass

## Dependencies

### External Dependencies
- **SRCHREL Phase 1 (Complete):** Edge quality heuristics validated
- **Graph Executor (Exists):** Current implementation provides baseline

### Cross-Phase Dependencies
- Phase 1 → Phase 2: Core implementation must complete before integration
- Phase 2 → Phase 3: Integration must stabilize before validation
- **No blocking dependencies between Phase 1 sub-tasks** (can parallelize)

## Risk Mitigation

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Performance exceeds budget (>30ms) | Low | High | Benchmark early (Phase 1), optimize SQL, add indexes if needed, implement timeout |
| Ranking quality doesn't improve | Medium | High | Validate on test queries (Phase 2), A/B test, make weights configurable, rollback flag |
| Edge quality heuristics inaccurate | Medium | Medium | Start with validated weights from Phase 1, tune based on metrics, add file path detection (Phase 2+) |
| Database query inefficiency | Low | Medium | EXPLAIN QUERY PLAN in Phase 1, verify index usage, optimize joins |
| Configuration complexity confuses users | Low | Low | Simple YAML schema, validation on load, clear error messages, examples in docs |
| Feature flag left enabled too long | Low | Low | Document removal plan, set calendar reminder for flag cleanup |

## Success Metrics

### Must Achieve (Phase 2)
- [ ] Graph executor latency <30ms p95 (was ~20ms, budget +10ms)
- [ ] Total search latency <100ms p95 (within initiative target)
- [ ] Architecturally important code ranks in top 3 for representative queries
- [ ] Configuration validated (loads successfully, weights applied)
- [ ] Feature flag works (can toggle between old/new)

### Should Achieve (Phase 3)
- [ ] Production code callers weighted 2× test code callers (measurable in logs)
- [ ] Inheritance edges boosted (base classes rank higher)
- [ ] Fusion weight optimized (find best value between 0.15-0.25)
- [ ] Zero performance regressions on baseline queries
- [ ] Configuration documented with tuning examples

### Nice to Have (Future)
- [ ] Hot config reload (no restart needed)
- [ ] File path-based test detection (more accurate)
- [ ] A/B test results (user satisfaction metrics)

## Timeline

**Total Duration:** 4-5 weeks

- **Week 1-2:** Phase 1 (Core Implementation)
  - Week 1: Edge quality scorer + configuration
  - Week 2: Enhanced graph executor + unit tests

- **Week 3-4:** Phase 2 (Integration & Testing)
  - Week 3: Pipeline integration + benchmarks
  - Week 4: Integration tests + performance tuning

- **Week 5:** Phase 3 (Validation & Documentation)
  - Ranking evaluation + documentation + rollout prep

**Buffer:** 1 week built into estimates for unexpected issues

## Rollout Strategy

### Stage 0: Gradual Fusion Weight Increase (Optional)

**Context:** Risk that quality weighting + fusion weight increase (0.10 → 0.15) compounds to over-weight graph signal.

**Mitigation:**
- **Stage 0a:** Enable quality weighting with fusion weight = 0.10 (current)
- Monitor for 1 week: Does graph signal dominate keyword matches?
- **Stage 0b:** If balanced, increase fusion weight to 0.15
- **If unbalanced:** Keep at 0.10, or tune quality weights down

**Decision:** Based on Phase 2 evaluation results

### Stage 1: Deploy with Flag Disabled (Week 5-6)
- Deploy enhanced executor code to production
- Feature flag: `enable_quality_scoring: false`
- **Purpose:** Validate deployment, no behavior change
- **Duration:** 2-3 days
- **Acceptance:** No errors, no performance regressions, flag toggle works

### Stage 2: Internal Testing (Week 6)
- Enable flag in staging/development environment
- Test queries against staging database
- Monitor metrics:
  - Graph executor latency (p50, p95, p99)
  - Score distributions (graph score vs FTS/vector)
  - Top result quality (manual spot check)
- **Purpose:** Smoke test before production rollout
- **Duration:** 2-3 days
- **Acceptance:** Latency <35ms p95, rankings improved on test queries

### Stage 3: Production Rollout with Monitoring (Week 6-7)
- Enable flag: `enable_quality_scoring: true` in production
- **Monitoring Period:** 24-48 hours intensive monitoring
- **Watch for:**
  - Latency spikes: Graph executor p95 >40ms → WARNING
  - Latency critical: Graph executor p95 >50ms → ROLLBACK
  - Error rate increase: >1% errors → INVESTIGATE
  - Score balance: Graph scores dominating (>50% final score) → REVIEW
- **Checkpoints:** 6 hours, 24 hours, 48 hours after enable

### Stage 4: Stabilization & Tuning (Week 7+)
- Continue monitoring for 1 week
- Gather user feedback (if available)
- Tune weights based on production metrics (if needed)
- Document learnings
- **Future:** Remove feature flag after 1 month of stability (make quality scoring default)

## Rollback Procedure

### Trigger Conditions

**Automatic Rollback** (if implemented):
- Graph executor p95 latency >50ms for 10+ minutes
- Error rate >5% for 5+ minutes

**Manual Rollback** (engineer decision):
- Graph executor p95 latency >40ms sustained
- Error rate >1% sustained
- User complaints about ranking quality degradation
- Unexpected behavior (scores extremely skewed)

### Rollback Steps

**Time Limit:** 15 minutes from decision to rollback complete

1. **Immediate Mitigation** (< 5 minutes)
   - Edit configuration file: Set `enable_quality_scoring: false`
   - OR set environment variable: `MAPROOM_ENABLE_QUALITY_SCORING=false`
   - Restart maproom search service (or hot reload if implemented)

2. **Verification** (< 5 minutes)
   - Check logs: Confirm flag is false
   - Run test query: Verify old behavior restored
   - Check metrics: Latency should return to baseline

3. **Communication** (< 5 minutes)
   - Notify team: Rollback completed, investigating issue
   - Document issue in incident log
   - Preserve metrics/logs for investigation

4. **Post-Rollback Investigation**
   - Analyze: What caused the issue?
   - Reproduce: Can we replicate in staging?
   - Fix: Tune weights, optimize SQL, or defer feature
   - Re-deploy: After fix validated in staging

### Rollback Authority

**Who can rollback:**
- Any engineer with production config access
- On-call engineer (24/7)
- Engineering manager

**No approval needed:** Rollback is safety mechanism, execute immediately if trigger conditions met.

### Rollback Testing

**Before production rollout:**
- [ ] Test rollback procedure in staging
- [ ] Verify flag toggle works without code changes
- [ ] Measure: How long does rollback take? (target: <5 minutes)
- [ ] Document procedure in runbook

## Handoff to Implementation

### Before Starting
- [ ] Review all planning documents (analysis, architecture, plan, quality, security)
- [ ] Understand edge quality heuristics from SRCHREL Phase 1
- [ ] Familiarize with existing graph executor code
- [ ] Set up local development environment

### First Tasks (Phase 1)
1. **Create graph_quality module:** Edge quality computation, test detection
2. **Add configuration schema:** YAML structure, loading, validation
3. **Implement enhanced executor:** Quality-weighted SQL query
4. **Write unit tests:** Edge quality scorer, test detection heuristic

### Success Indicators
- Edge quality correctly distinguishes production/test code
- SQL query is efficient (uses indexes)
- Configuration validates on load
- Unit tests achieve confidence in quality computation

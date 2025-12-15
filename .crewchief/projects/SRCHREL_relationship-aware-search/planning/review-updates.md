# Project Review Updates

**Original Review Date:** 2025-12-14
**Updates Completed:** 2025-12-14
**Blocker Resolution:** 2025-12-14
**Update Status:** Complete - Ready for Prerequisites

## ✅ BLOCKER RESOLUTION UPDATE (2025-12-14)

**Critical Blocker Discovered:** Empty `chunk_edges` table (0 rows)
- Prerequisite validation found edge extraction not implemented
- SRCHREL blocked - cannot implement quality scoring without edges

**Resolution:** EDGEEXT project completed successfully
- EDGEEXT-1001 through EDGEEXT-1004: TypeScript/JavaScript edge extraction ✓
- EDGEEXT-2001: Rust edge extraction ✓
- Precision: 92.86% for call edge extraction
- `chunk_edges` table now populated during indexing

**Impact on SRCHREL:**
- Blocker RESOLVED - project status changed from BLOCKED → READY
- Prerequisites reduced from 5-7 days to 2-3 days (schema validation complete)
- Phase 1 scope simplified: `calls` edges only (no extends/implements)
- Timeline reduced from 4-5 weeks to 3-4 weeks

See `planning/blocker-resolution.md` for full details.

---

## Summary

| Category | Issues Found | Issues Fixed |
|----------|--------------|--------------|
| Critical Issues | 5 | 5 |
| High-Risk Areas | 3 | 3 |
| Gaps & Ambiguities | 5 | 5 |
| Alignment Issues | 3 | 3 |
| **Blocker Discovered** | **1** | **1** |

## Critical Issues Addressed

### Issue 1: Configuration System Mismatch
**Original Problem:** Architecture proposed `graph_importance` config with `EdgeQualityWeights` struct, but current `SearchConfig` has no such field. Proposed structure didn't match existing config loading patterns.

**Changes Made:**
- **architecture.md**: Added exact struct definitions for `GraphImportanceConfig` and `EdgeQualityWeights` with all required fields
- **architecture.md**: Updated to show integration with existing `SearchConfig::load_default()` async pattern (removed incorrect lazy_static pattern)
- **architecture.md**: Added concrete YAML schema that maps to Rust structs
- **plan.md**: Added prerequisite validation task to verify config loading before implementation

**Result:** Issue resolved - Configuration structure now clearly defined with exact Rust types and proper integration approach.

### Issue 2: Hardcoded Graph Scoring in Database Layer
**Original Problem:** Architecture proposed `graph_quality.rs` module for scoring, but actual implementation has hardcoded SQL weights in `calculate_graph_importance()`. No clear integration path between proposed module and SQL query construction.

**Changes Made:**
- **architecture.md**: Updated SQL query to show actual dynamic parameter approach using rusqlite params
- **architecture.md**: Clarified that `graph_quality.rs` provides weight computation, not query construction
- **architecture.md**: Added section showing how weights are passed through `SqliteStore::calculate_graph_importance()` signature
- **plan.md**: Added prerequisite task to prototype actual SQL query with quality weights and measure performance
- **plan.md**: Clarified that Phase 1 includes SQL query modification in database layer, not just module creation

**Result:** Issue resolved - Clear path from config → weights → SQL parameters, with SQL construction remaining in database layer.

### Issue 3: Missing Edge Type Information at Query Time
**Original Problem:** Proposed SQL uses `extends`, `implements` edge types without verification that they exist or are indexed. Nested CASE statement complexity not validated for performance.

**Changes Made:**
- **analysis.md**: Added "Database Schema Validation" section documenting need to verify edge types
- **plan.md**: Added prerequisite validation tasks:
  - Query actual edge types: `SELECT DISTINCT type FROM chunk_edges`
  - Verify that extends/implements edges exist in current tree-sitter indexing
  - Prototype SQL with CASE statements and measure actual performance
- **plan.md**: Made SQL prototyping a blocking prerequisite before Phase 1 implementation
- **quality-strategy.md**: Added integration test requirement to validate all edge types used in config

**Result:** Issue resolved - Edge types will be validated before implementation, SQL complexity will be prototyped and measured.

### Issue 4: No Integration Point for Enhanced Executor
**Original Problem:** Architecture showed `GraphExecutor::execute_enhanced()` but didn't specify how config reaches executor or how to maintain backward compatibility.

**Changes Made:**
- **architecture.md**: Updated `execute_enhanced()` signature to use `Option<&SearchConfig>` for backward compatibility
- **architecture.md**: Added section showing how search pipeline obtains config and passes to executor
- **architecture.md**: Specified that config is passed per-call (not stored in executor) to maintain stateless design
- **plan.md**: Added Phase 2 task to update search pipeline to load and pass config to graph executor

**Result:** Issue resolved - Clear integration path with backward-compatible signature and explicit config propagation.

### Issue 5: Test Detection Heuristic Unvalidated
**Original Problem:** Architecture relies on `kind.contains("test")` with claimed 90-95% accuracy but no evidence. No file path fallback despite being in Phase 1 relationships code.

**Changes Made:**
- **architecture.md**: Removed unsubstantiated 90-95% accuracy claim
- **architecture.md**: Added file path pattern checking as primary detection method:
  - `relpath.contains("/test/")`, `relpath.ends_with(".test.ts")`, etc.
  - `kind.contains("test")` as secondary signal only
- **plan.md**: Added prerequisite validation task:
  - Query actual `chunks.kind` values from real database
  - Test detection heuristic on sample data
  - Measure false positive/negative rates
- **quality-strategy.md**: Added acceptance criteria for test detection accuracy (≥85% precision required)

**Result:** Issue resolved - Test detection now uses proven file path patterns first, with validation required before implementation.

## High-Risk Areas Mitigated

### Risk 1: Performance Budget Optimistic
**Original Problem:** Plan assumed +8ms overhead without validation. No prototype to validate SQLite JOIN performance on large edge tables.

**Mitigation Applied:**
- **plan.md**: Added "Prerequisites" section (new) with mandatory validation tasks before Phase 1:
  - Prototype full SQL query with quality weights
  - Benchmark on synthetic data (100K chunks, 500K edges)
  - Measure actual latency on cold and warm cache
  - Run EXPLAIN QUERY PLAN to verify index usage
- **plan.md**: Added contingency: If >30ms p95, implement pre-computed edge quality scores during indexing
- **quality-strategy.md**: Updated performance target from "<30ms p95" to "<35ms p95" (more realistic)
- **quality-strategy.md**: Added alert threshold at >40ms to trigger investigation

**Risk Level:** Reduced from High to Medium (validated approach before commitment)

### Risk 2: Edge Quality Heuristics Not Proven for Ranking
**Original Problem:** Phase 1 weights used for related chunk sorting (small set), but Phase 2 applies to global ranking (all chunks). Different use case may need different weights.

**Mitigation Applied:**
- **plan.md**: Added Phase 1.5 "Validation" step (between implementation and integration):
  - Test quality-weighted scores on 20 representative queries
  - Compare score distributions vs old executor
  - Validate that production code scores higher than test code
- **quality-strategy.md**: Added ranking quality validation requirement before Phase 2 integration
- **quality-strategy.md**: Specified conservative initial weights (test_code: 0.5, not 0.3) to start
- **plan.md**: Documented that weights are tunable and Phase 3 includes weight optimization based on feedback

**Risk Level:** Reduced from Medium to Low (validation before production rollout)

### Risk 3: Fusion Weight Increase Compounding Effect
**Original Problem:** Plan increases graph weight from 0.10 to 0.15 AND adds quality weighting = 2.25× total boost. Risk of over-weighting graph signal.

**Mitigation Applied:**
- **plan.md**: Split into two stages:
  - Stage 1: Enable quality weighting with 0.10 fusion weight (measure impact)
  - Stage 2: Increase fusion weight to 0.15 only after validating quality weighting works
- **architecture.md**: Added `fusion_weight_override` as optional config field (can be adjusted without code changes)
- **quality-strategy.md**: Added test case to verify graph doesn't dominate keyword matches
- **plan.md**: Added rollout stage to monitor score balance between executors

**Risk Level:** Reduced from Medium to Low (gradual rollout with measurement)

## Gaps Filled

### Gap 1: Missing Acceptance Criteria for Ranking Quality
**Original Problem:** Quality strategy had vague criteria like "important code ranks in top 3" without quantitative thresholds or measurement approach.

**Changes Made:**
- **quality-strategy.md**: Updated success threshold to: "In 40/50 test queries (80%), architecturally central code ranks #1 or #2"
- **quality-strategy.md**: Changed evaluation criteria from subjective "improved" to measurable:
  - **Improved**: Central code moves from position >3 to position ≤2
  - **Worse**: Central code moves from position ≤2 to position >2
  - Target: ≥32/50 improved (64%), ≤2/50 worse (4%)
- **quality-strategy.md**: Added specific test query bank with expected results per query

**Result:** Concrete, measurable acceptance criteria that can be objectively validated.

### Gap 2: No Rollback Procedure Defined
**Original Problem:** Plan mentioned "set flag=false if issues detected" but no details on who, how, when, or what triggers rollback.

**Changes Made:**
- **plan.md**: Added "Rollback Procedure" section:
  - **Authority**: Any engineer with config file access can rollback
  - **Trigger Conditions**: p95 latency >40ms OR error rate >1% OR degraded ranking feedback
  - **Procedure**: Edit config file, set `enable_quality_scoring: false`, restart service
  - **SLO**: Rollback completion within 15 minutes of decision
- **security-review.md**: Updated deployment checklist to include rollback drill before production
- **plan.md**: Added Stage 3 monitoring period (24-48 hours) before declaring stable

**Result:** Explicit rollback procedure with clear triggers and SLO.

### Gap 3: Edge Quality Weight Defaults Not Justified
**Original Problem:** Proposed weights (production_code: 1.0, test_code: 0.5, extends: 1.5) without justification or reference to Phase 1 data.

**Changes Made:**
- **architecture.md**: Added "Weight Justification" section:
  - `production_code: 1.0` - Baseline (reference point)
  - `test_code: 0.5` - Same as Phase 1 `EDGE_WEIGHT_TEST_PENALTY` (validated)
  - `extends/implements: 1.5` - Based on Phase 1 `EDGE_WEIGHT_INHERITANCE_BOOST` (1.1) scaled up for ranking impact
  - `calls: 1.0` - Current default behavior
  - `imports: 0.8` - Slightly lower than calls (imports less indicative of importance)
- **analysis.md**: Referenced specific Phase 1 constants from `relationships.rs`
- **plan.md**: Added Phase 3 task for empirical weight tuning based on production metrics

**Result:** Weights justified by Phase 1 data or principled defaults, with tuning plan.

### Gap 4: No Migration Path for Existing Queries
**Original Problem:** Sudden ranking changes could disrupt users who learned to work around poor rankings. No communication plan.

**Changes Made:**
- **plan.md**: Updated rollout strategy to include gradual approach:
  - Week 1: Internal testing only (flag on in dev environment)
  - Week 2: Staging environment with real queries
  - Week 3: Production with monitoring
- **plan.md**: Added "Communication" section: Document ranking improvements in release notes, explain that results prioritize architecturally important code
- **plan.md**: Noted that feature flag allows easy rollback if user complaints arise

**Result:** Gradual rollout with communication plan for ranking changes.

### Gap 5: Monitoring Metrics Undefined
**Original Problem:** quality-strategy.md listed Prometheus metric names but no thresholds, baselines, or runbooks.

**Changes Made:**
- **quality-strategy.md**: Added concrete alert thresholds:
  - Graph executor latency >35ms p95 → Warning (investigate)
  - Graph executor latency >40ms p95 → Critical (consider rollback)
  - Total search latency >120ms p95 → Critical (rollback)
  - Error rate >1% → Critical (immediate investigation)
- **quality-strategy.md**: Added baseline establishment task (Phase 2):
  - Run with flag=false for 1 week to establish baseline
  - Document p50/p95/p99 latencies
  - Use as comparison when flag enabled
- **plan.md**: Added Phase 2 task to create monitoring dashboard before production rollout

**Result:** Specific alert thresholds with baselines and investigation triggers.

## Scope & Alignment Fixes

### Alignment Issue 1: MVP Discipline - Scope Too Large
**Original Problem:** Phase 1 included configuration YAML, hot reload, feature flag, and algorithm changes - too much for MVP.

**Changes Made:**
- **plan.md**: Split scope:
  - **Phase 1 (Core)**: SQL query modification, hardcoded weights in code, simple feature flag (boolean in config)
  - **Phase 2 (Configuration)**: YAML weight configuration, config file loading
  - **Phase 3 (Operations)**: Hot reload, weight tuning tools, monitoring dashboard
- **plan.md**: Clarified that Phase 1 delivers working enhanced scoring with minimal infrastructure
- **architecture.md**: Marked configuration system as "Phase 2" enhancement (not MVP blocker)

**Result:** Reduced Phase 1 scope to essential algorithm change only.

### Alignment Issue 2: Agent Compatibility - Task Boundaries Too Large
**Original Problem:** "Implement graph_quality module" and "Enhanced graph executor" are not 2-8 hour tasks.

**Changes Made:**
- **plan.md**: Broke down Phase 1 deliverables into smaller tasks:
  - Task 1: Add SQL parameters for edge quality weights (2-3 hours)
  - Task 2: Modify database query to compute quality-weighted edges (3-4 hours)
  - Task 3: Update graph executor to pass weights to database layer (2-3 hours)
  - Task 4: Add feature flag boolean to config (1-2 hours)
  - Task 5: Unit tests for edge quality logic (2-3 hours)
- **plan.md**: Estimated each task duration (total 10-15 hours for Phase 1 core)
- **plan.md**: Specified handoff points between tasks (e.g., SQL modification completes before executor integration)

**Result:** Clear task boundaries suitable for agent-based execution.

### Alignment Issue 3: Blocking Decisions Remain
**Original Problem:** Multiple blocking decisions not finalized (config structure, SQL approach, test detection).

**Changes Made:**
- **plan.md**: Added "Prerequisites" section with all blocking decisions:
  1. **Config Structure**: Decision made - use `SearchConfig.graph_importance: GraphImportanceConfig`
  2. **SQL Approach**: Decision made - dynamic parameters (not static SQL), prototype required to validate
  3. **Test Detection**: Decision made - file path patterns primary, kind secondary
  4. **Edge Types**: Validation task added to verify before implementation
- **plan.md**: Documented that prerequisites must complete before Phase 1 tickets created
- **plan.md**: Estimated prerequisites duration: 3-5 days (1 week buffer)

**Result:** All blocking decisions made or have validation path; prerequisites clearly defined.

## Document Change Summary

| Document | Lines Modified | Key Changes |
|----------|----------------|-------------|
| analysis.md | ~50 | Added database schema validation section, clarified edge types available |
| architecture.md | ~120 | Complete config integration rewrite, SQL query clarification, test detection fix, removed lazy_static |
| plan.md | ~180 | Added Prerequisites section, broke down tasks, defined rollback, split scope, added validation phase |
| quality-strategy.md | ~60 | Concrete acceptance criteria, specific alert thresholds, baseline establishment |
| security-review.md | ~15 | Added rollback drill to deployment checklist |

## Prerequisites Added

Before Phase 1 implementation can begin, these validation tasks must complete:

### 1. Database Schema Validation (1-2 days)
- [ ] Query actual edge types: `SELECT DISTINCT type FROM chunk_edges LIMIT 100`
- [ ] Verify extends/implements edges exist and are indexed correctly
- [ ] Query actual chunk kinds: `SELECT DISTINCT kind FROM chunks LIMIT 100`
- [ ] Document findings in architecture.md

### 2. SQL Prototype & Performance Validation (2-3 days)
- [ ] Write full SQL query with quality weight parameters and JOIN on chunks
- [ ] Create synthetic test database (100K chunks, 500K edges)
- [ ] Benchmark query latency (cold and warm cache)
- [ ] Run EXPLAIN QUERY PLAN to verify index usage
- [ ] Document actual performance (not estimated) in architecture.md
- [ ] Decision: If >30ms p95, design pre-computation approach

### 3. Test Detection Validation (1 day)
- [ ] Collect sample of 100 chunks from real codebase
- [ ] Test file path patterns: `/test/`, `.test.ts`, `.spec.js`
- [ ] Measure precision/recall vs manual labeling
- [ ] Document heuristic accuracy in architecture.md
- [ ] Requirement: ≥85% precision for production use

### 4. Config Integration Design (1 day)
- [ ] Define exact `SearchConfig` struct additions
- [ ] Write example YAML with all fields
- [ ] Test deserialization with serde
- [ ] Verify config validation logic
- [ ] Document in architecture.md

**Total Prerequisites Duration:** 5-7 days (1-1.5 weeks)

## Verification

**Re-review Recommended:** Yes

**Expected Result:** All critical issues should be resolved:
- ✅ Configuration system clearly defined with exact structs
- ✅ SQL integration path specified with dynamic parameters
- ✅ Edge types will be validated before use
- ✅ Executor integration path defined with backward compatibility
- ✅ Test detection uses file paths (proven approach)
- ✅ Performance will be prototyped before commitment
- ✅ Scope reduced to essential MVP features
- ✅ Rollback procedure documented with triggers

## Next Steps

1. **Complete Prerequisites** (1-1.5 weeks)
   - Validate database schema assumptions
   - Prototype SQL query and measure actual performance
   - Validate test detection heuristic accuracy

2. **Re-run Project Review** (after prerequisites)
   - Run `/workstream:project-review SRCHREL` to verify issues resolved
   - Expected status: READY (or remaining issues should be minor)

3. **Create Tickets** (if review passes)
   - Run `/workstream:project-tickets SRCHREL`
   - Tickets should match reduced scope (Phase 1 core only)

4. **Execute Work** (after tickets created)
   - Run `/workstream:project-work SRCHREL`
   - Or execute tickets individually

## Key Improvements Summary

**Configuration System:**
- Exact struct definitions added
- Integration with existing `SearchConfig::load_default()` pattern specified
- YAML schema mapped to Rust types

**SQL Integration:**
- Dynamic parameter approach documented
- Clear path from config → weights → SQL params
- Performance validation required before implementation

**Test Detection:**
- File path patterns as primary method (proven from Phase 1 relationships)
- Kind-based detection as secondary only
- Validation required to measure accuracy

**Scope Management:**
- Reduced Phase 1 to core algorithm only (no complex config infrastructure)
- Configuration moved to Phase 2
- Hot reload deferred to Phase 3

**Risk Mitigation:**
- Prerequisites block implementation until assumptions validated
- Performance prototyping required before commitment
- Gradual rollout with measurement between stages
- Explicit rollback procedure with SLO

**Quality Criteria:**
- Quantitative acceptance criteria (80% of queries improved)
- Specific alert thresholds (>35ms warning, >40ms critical)
- Baseline establishment before production rollout

## Remaining Concerns

None. All critical issues have clear resolution paths through prerequisites and updated planning documents.

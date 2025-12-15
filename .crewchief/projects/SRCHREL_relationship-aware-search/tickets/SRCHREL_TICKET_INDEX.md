# SRCHREL Ticket Index

## Prerequisites (Phase 0) - BLOCKING

### SRCHREL-0001: Database Schema Validation
**Status:** Not Started
**Dependencies:** None
**Blocks:** SRCHREL-0002, SRCHREL-0003, Phase 1
**Summary:** Validate database schema supports quality-weighted scoring (edge types, chunk fields, test detection patterns)

### SRCHREL-0002: SQL Performance Validation
**Status:** Not Started
**Dependencies:** SRCHREL-0001
**Blocks:** Phase 1
**Summary:** Benchmark quality-weighted SQL query (<30ms p95 target), validate index usage

### SRCHREL-0003: Test Detection Validation
**Status:** Not Started
**Dependencies:** SRCHREL-0001
**Blocks:** SRCHREL-1001
**Summary:** Measure file path-based test detection accuracy (≥85% precision, ≥80% recall)

### SRCHREL-0004: Config Integration Design Validation
**Status:** Not Started
**Dependencies:** None
**Blocks:** SRCHREL-1002
**Summary:** Design and validate feature flag approach (environment variable vs config boolean)

---

## Phase 1: Core Implementation (MVP - Hardcoded Weights)

### SRCHREL-1001: Modify Database Query
**Status:** Not Started
**Dependencies:** SRCHREL-0001, SRCHREL-0002, SRCHREL-0003
**Blocks:** SRCHREL-1003
**Summary:** Implement quality-weighted SQL query with hardcoded weights and feature flag parameter

### SRCHREL-1002: Add Feature Flag Support
**Status:** Not Started
**Dependencies:** SRCHREL-0004
**Blocks:** SRCHREL-1003
**Summary:** Add feature flag to enable/disable quality scoring (default: disabled)

### SRCHREL-1003: Update Graph Executor
**Status:** Not Started
**Dependencies:** SRCHREL-1001, SRCHREL-1002
**Blocks:** SRCHREL-1004, SRCHREL-1005
**Summary:** Update graph executor to read feature flag and pass to database layer

### SRCHREL-1004: Unit Tests for Quality SQL
**Status:** Not Started
**Dependencies:** SRCHREL-1001, SRCHREL-1002, SRCHREL-1003
**Blocks:** SRCHREL-1005
**Summary:** Unit tests validating production code scores higher than test code

### SRCHREL-1005: Integration Tests
**Status:** Not Started
**Dependencies:** SRCHREL-1001, SRCHREL-1002, SRCHREL-1003, SRCHREL-1004
**Blocks:** None
**Summary:** End-to-end integration tests comparing old vs enhanced executor

---

## Phase 1.5: Validation (Before Phase 2)

### SRCHREL-1006: Baseline Measurement
**Status:** Not Started
**Dependencies:** Phase 1 complete
**Blocks:** SRCHREL-1007
**Summary:** Run 20 test queries with quality scoring disabled, establish baseline rankings

### SRCHREL-1007: Enhanced Scoring Validation
**Status:** Not Started
**Dependencies:** SRCHREL-1006
**Blocks:** SRCHREL-1008
**Summary:** Run same 20 queries with quality scoring enabled, compare to baseline

### SRCHREL-1008: Go/No-Go Decision
**Status:** Not Started
**Dependencies:** SRCHREL-1006, SRCHREL-1007
**Blocks:** Phase 2
**Summary:** Analyze validation results, decide whether to proceed to Phase 2 or tune weights

---

## Phase 2: Configuration & Pipeline Integration

### SRCHREL-2001: Configuration Schema
**Status:** Not Started
**Dependencies:** SRCHREL-1008 (GO decision)
**Blocks:** SRCHREL-2002
**Summary:** Add YAML configuration schema (GraphImportanceConfig, EdgeQualityWeights structs)

### SRCHREL-2002: SQL Parameterization
**Status:** Not Started
**Dependencies:** SRCHREL-2001
**Blocks:** SRCHREL-2003
**Summary:** Replace hardcoded weights with parameters from configuration

### SRCHREL-2003: Pipeline Integration
**Status:** Not Started
**Dependencies:** SRCHREL-2002
**Blocks:** SRCHREL-2004, SRCHREL-2005
**Summary:** Load config in search pipeline, pass to graph executor, implement fusion weight override

### SRCHREL-2004: Performance Benchmarking
**Status:** Not Started
**Dependencies:** SRCHREL-2003
**Blocks:** None
**Summary:** Benchmark on real CrewChief codebase (p50/p95/p99 latencies, validate <35ms p95)

### SRCHREL-2005: Ranking Quality Evaluation
**Status:** Not Started
**Dependencies:** SRCHREL-2003
**Blocks:** None
**Summary:** Evaluate 50 queries manually (≥64% improved, ≤4% degraded target)

---

## Phase 3: Validation & Documentation

### SRCHREL-3001: Configuration Documentation
**Status:** Not Started
**Dependencies:** SRCHREL-2001, SRCHREL-2005
**Blocks:** None
**Summary:** Create configuration guide with examples and tuning guidelines

### SRCHREL-3002: Monitoring Setup
**Status:** Not Started
**Dependencies:** SRCHREL-2003
**Blocks:** SRCHREL-3003
**Summary:** Set up metrics, alerts, and dashboards for latency and score distributions

### SRCHREL-3003: Rollout Plan
**Status:** Not Started
**Dependencies:** SRCHREL-3001, SRCHREL-3002
**Blocks:** None
**Summary:** Create 4-stage rollout plan with checkpoints and rollback procedures

### SRCHREL-3004: Edge Case Testing
**Status:** Not Started
**Dependencies:** Phase 1 and Phase 2 complete
**Blocks:** None
**Summary:** Test edge cases (empty DB, only test code, hub nodes, malformed paths, extreme weights)

---

## Dependency Chain

```
Prerequisites (Phase 0):
SRCHREL-0001 (Schema Validation) ──────┬─────────────────┐
                                       │                 │
SRCHREL-0002 (SQL Performance) ────────┤                 │
                                       │                 │
SRCHREL-0003 (Test Detection) ─────────┤                 │
                                       │                 │
SRCHREL-0004 (Config Design) ──────────┼──────┐          │
                                       ↓      ↓          ↓
Phase 1 (Core Implementation):                          │
SRCHREL-1001 (Database Query) ─────────────┬──────────┐ │
                                           │          │ │
SRCHREL-1002 (Feature Flag) ───────────────┤          │ │
                                           ↓          │ │
SRCHREL-1003 (Graph Executor) ─────────────┬──────────┤ │
                                           │          │ │
SRCHREL-1004 (Unit Tests) ─────────────────┤          │ │
                                           ↓          │ │
SRCHREL-1005 (Integration Tests) ──────────┼──────────┘ │
                                           │            │
Phase 1.5 (Validation):                    │            │
SRCHREL-1006 (Baseline) ───────────────────┘            │
         ↓                                              │
SRCHREL-1007 (Enhanced Validation)                     │
         ↓                                              │
SRCHREL-1008 (Go/No-Go Decision) ──────────────────────┘
         ↓
Phase 2 (Configuration):
SRCHREL-2001 (Config Schema)
         ↓
SRCHREL-2002 (SQL Parameterization)
         ↓
SRCHREL-2003 (Pipeline Integration) ─────┬─────────┐
         ↓                               ↓         ↓
SRCHREL-2004 (Performance Benchmark)    SRCHREL-2005 (Quality Eval)
         ↓                               ↓
Phase 3 (Documentation & Rollout):       │
SRCHREL-3001 (Documentation) ────────────┘
         ↓
SRCHREL-3002 (Monitoring)
         ↓
SRCHREL-3003 (Rollout Plan)

SRCHREL-3004 (Edge Case Testing) ← (Independent, runs after Phase 1+2)
```

---

## Coverage Summary

**Prerequisites:** 4 tickets
- Schema validation
- Performance validation
- Test detection validation
- Config design

**Phase 1:** 5 tickets
- Database query modification
- Feature flag
- Graph executor update
- Unit tests
- Integration tests

**Phase 1.5:** 3 tickets
- Baseline measurement
- Enhanced validation
- Go/No-Go decision

**Phase 2:** 5 tickets
- Configuration schema
- SQL parameterization
- Pipeline integration
- Performance benchmarking
- Ranking quality evaluation

**Phase 3:** 4 tickets
- Configuration documentation
- Monitoring setup
- Rollout plan
- Edge case testing

**Total:** 21 tickets

---

## Planning Document Coverage

All deliverables from plan.md are covered:

### Prerequisites ✅
- ✅ Database schema validation → SRCHREL-0001
- ✅ SQL performance validation → SRCHREL-0002
- ✅ Test detection validation → SRCHREL-0003
- ✅ Config integration design → SRCHREL-0004

### Phase 1 ✅
- ✅ Modified SQL query → SRCHREL-1001
- ✅ Feature flag → SRCHREL-1002
- ✅ Graph executor update → SRCHREL-1003
- ✅ Unit tests → SRCHREL-1004
- ✅ Integration tests → SRCHREL-1005

### Phase 1.5 ✅
- ✅ Baseline measurement → SRCHREL-1006
- ✅ Enhanced validation → SRCHREL-1007
- ✅ Score analysis → SRCHREL-1007
- ✅ Go/No-Go decision → SRCHREL-1008

### Phase 2 ✅
- ✅ Configuration schema → SRCHREL-2001
- ✅ SQL parameterization → SRCHREL-2002
- ✅ Pipeline integration → SRCHREL-2003
- ✅ Performance benchmarks → SRCHREL-2004
- ✅ Ranking quality evaluation → SRCHREL-2005

### Phase 3 ✅
- ✅ Configuration documentation → SRCHREL-3001
- ✅ Monitoring/metrics → SRCHREL-3002
- ✅ Rollout plan → SRCHREL-3003
- ✅ Edge case testing → SRCHREL-3004

**Coverage: 100%** (all plan deliverables mapped to tickets)

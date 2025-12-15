# Project: Relationship-Aware Search Ranking

**Slug:** SRCHREL
**Status:** ✅ READY FOR IMPLEMENTATION
**Created:** 2025-12-14
**Phase:** 2 (enhances SRCHREL Phase 1)
**Prerequisites Validated:** 2025-12-14 - All prerequisites pass. See `planning/prerequisite-findings.md`

## Summary

Enhance maproom's graph-based search ranking with quality-weighted edge scoring to surface architecturally significant code above peripheral utilities. Transform the graph executor from simple edge counting to intelligent quality-aware importance calculation.

**Key Improvement:** A central authentication handler with 15 callers will now rank higher than a standalone utility function with 2 callers, even if both match the search query equally well.

## Problem Statement

Current search ranking underutilizes graph signals:
- Graph executor uses fixed weights (calls: 0.3, imports: 0.2, tests: 0.1)
- All edges counted equally (test code caller = production code caller)
- Graph contributes only 10% to final score (fusion weight)
- **Result:** Important, well-connected code doesn't consistently rank higher than isolated utility functions

**User Impact:** Developers see utility functions ranked above core implementations, requiring manual filtering to find architecturally significant code.

## Proposed Solution

**Quality-Weighted In-Degree Ranking:**

1. **Edge Quality Scoring:** Weight edges by source code type and relationship type
   - Production code caller: 1.0× weight
   - Test code caller: 0.5× weight (penalty)
   - Inheritance edge (extends/implements): 1.5× boost
   - Import edge: 0.8× (slightly lower than calls)

2. **Configurable Weights:** YAML configuration enables tuning without redeployment
   ```yaml
   graph_importance:
     enable_quality_scoring: true
     edge_quality_weights:
       production_code: 1.0
       test_code: 0.5
       extends: 1.5
       calls: 1.0
       imports: 0.8
     fusion_weight: 0.15  # Increased from 0.10
   ```

3. **Feature Flag:** Safe rollout with instant rollback capability

**Formula:**
```
Current: graph_score = LOG(2 + caller_count) * 0.3 + LOG(2 + importer_count) * 0.2

Enhanced: graph_score = LOG(2 + Σ(quality_weight(edge))) for all edges
          where quality_weight = edge_type_weight × source_kind_modifier
```

**Performance Budget:** <30ms p95 (current ~20ms, budget +10ms)

## Value Proposition

**For Developers:**
- Core implementations rank in top 3 results
- Base classes/interfaces surface before concrete implementations
- Active code ranks above deprecated code
- Architectural significance reflected in ranking

**For the Product:**
- Leverages existing graph data (no schema changes)
- Reuses validated heuristics from SRCHREL Phase 1
- Configurable for different codebase characteristics
- Measurable ranking quality improvement

**Differentiator:** GitHub/Sourcegraph don't use intra-codebase graph signals for ranking. This is unique to maproom.

## Relevant Agents

### Planning Phase
- project-planner: Complete planning documents ✓
- project-reviewer: Review planning for technical feasibility

### Implementation Phase (3 phases, 4-5 weeks)

**Phase 1: Core Implementation (Weeks 1-2)**
- rust-expert: Edge quality scorer module, enhanced graph executor
- database-engineer: SQL optimization, index validation
- config-engineer: YAML schema, configuration loading
- test-engineer: Unit tests for edge quality computation

**Phase 2: Integration & Testing (Weeks 3-4)**
- search-engineer: Pipeline integration, fusion weight override
- performance-engineer: Benchmarks, latency validation
- test-engineer: Integration tests, ranking quality evaluation
- rust-expert: Performance optimization if needed

**Phase 3: Validation & Documentation (Week 5)**
- technical-writer: Configuration guide, tuning documentation
- search-engineer: Ranking quality evaluation (50+ queries), weight tuning
- ops-engineer: Monitoring setup, rollout plan
- test-engineer: Edge case testing

### Verification Phase
- verify-ticket: Validate acceptance criteria
- commit-ticket: Create commits for completed work

## Planning Documents

- [analysis.md](planning/analysis.md) - Problem definition, research findings, constraints, success criteria
- [architecture.md](planning/architecture.md) - Solution design, component architecture, integration points, performance considerations
- [plan.md](planning/plan.md) - 3-phase execution plan with agent assignments and timelines
- [quality-strategy.md](planning/quality-strategy.md) - Testing philosophy, critical paths, performance benchmarks
- [security-review.md](planning/security-review.md) - Security assessment (LOW risk, server-side optimization)

## Dependencies

### External (COMPLETE)
- **SRCHREL Phase 1** (relationship-clustering): Edge quality heuristics validated
  - Test code penalty: 0.5×
  - Inheritance boost: 1.5×
  - Proven to improve related chunk relevance

- **Graph Executor** (existing): Baseline implementation
  - Current: Simple edge counting with fixed weights
  - Target: Quality-weighted edge scoring

### Internal Phase Dependencies
- Phase 1 → Phase 2: Core implementation must complete before integration
- Phase 2 → Phase 3: Integration must stabilize before validation
- No blocking dependencies within Phase 1 (can parallelize)

## Key Metrics

**Must Achieve:**
- [ ] Architecturally important code ranks in top 3 (manual evaluation on 50+ queries)
- [ ] Performance within budget (<30ms p95 graph executor, <100ms total search)
- [ ] Configuration validated (loads successfully, weights applied correctly)
- [ ] Feature flag works (toggle between old/new implementations)
- [ ] Zero performance regressions on baseline queries

**Should Achieve:**
- [ ] Production code callers weighted 2× test code callers (measurable in logs)
- [ ] Base classes/interfaces rank higher (inheritance boost validated)
- [ ] Fusion weight optimized (find best value 0.15-0.25)
- [ ] Ranking quality improved: ≥60% queries better, ≤5% worse

**Nice to Have:**
- [ ] Hot config reload (no restart needed)
- [ ] File path-based test detection (more accurate than kind heuristic)
- [ ] A/B test results (user satisfaction metrics)

## Timeline

**Total Duration:** 4-5 weeks (with 1 week buffer)

- **Week 1-2:** Phase 1 - Core Implementation
  - Edge quality scorer + configuration (Week 1)
  - Enhanced graph executor + unit tests (Week 2)

- **Week 3-4:** Phase 2 - Integration & Testing
  - Pipeline integration + benchmarks (Week 3)
  - Integration tests + performance tuning (Week 4)

- **Week 5:** Phase 3 - Validation & Documentation
  - Ranking evaluation + documentation + rollout prep

## Next Steps

1. **Review Planning:** Run `/review-project SRCHREL` to validate planning documents
2. **Create Tickets:** Run `/create-project-tickets SRCHREL` to generate implementation tickets
3. **Begin Implementation:** Start with Phase 1 (Core Implementation)

## Architecture Highlights

**Component Structure:**
```
crates/maproom/src/search/
├── graph_quality.rs      # NEW: Edge quality scorer
├── graph.rs              # MODIFIED: Enhanced executor
└── fusion/mod.rs         # MODIFIED: Fusion weight override

crates/maproom/config/
└── maproom-search.yml    # MODIFIED: Add graph_importance section
```

**Data Flow:**
```
Search Query
  ↓
Load Config (cached)
  ↓
Parallel Executors:
├─ FTS Executor (unchanged)
├─ Vector Executor (unchanged)
├─ Graph Executor (ENHANCED)
│  ├─ Check enable_quality_scoring flag
│  ├─ If true: Quality-weighted SQL query
│  │  ├─ JOIN edges with source chunk kinds
│  │  ├─ Apply edge quality weights
│  │  ├─ SUM quality-weighted edges
│  │  └─ LOG(2 + sum) = graph_score
│  └─ Return RankedResults
└─ Temporal Executor (unchanged)
  ↓
RRF Fusion (graph weight 0.15 vs 0.10)
  ↓
Final Results (important code ranks higher)
```

**Performance Budget:**
- Edge aggregation: +5ms (JOIN with source chunks)
- Quality computation: +3ms (CASE statements)
- **Total overhead:** +8ms (well within budget)

## Testing Strategy

**Critical Paths (MUST test):**
1. Edge quality computation (all edge types, source kinds)
2. Feature flag toggle (enable/disable, rollback)
3. Performance budget (<30ms p95, no regressions)
4. Configuration validation (invalid weights rejected)
5. Ranking quality (important code in top 3)
6. SQL query efficiency (uses indexes, no full table scans)

**Test Types:**
- **Unit:** Edge quality scorer, test detection heuristic, config validation
- **Integration:** Enhanced executor vs old, feature flag, fusion weight override
- **Performance:** Benchmarks on 100K chunks, latency validation
- **Ranking Quality:** Manual evaluation on 50+ representative queries

**Quality Gates:**
- `cargo test` passes
- `cargo clippy` clean
- Benchmarks meet targets
- Ranking quality improved (≥60% better, ≤5% worse)

## Security Assessment

**Risk Level:** LOW

- No authentication/authorization changes
- No new sensitive data processing
- Server-side optimization only
- Parameterized SQL queries (injection-safe)
- Configuration validation prevents DoS
- Feature flag enables instant rollback

**Approved for MVP** with standard configuration file permissions and rollback plan.

## Agent Recommendation

**Custom agents NOT recommended** for this project.

**Rationale:** Well-suited for general agents (rust-expert, database-engineer, search-engineer). Straightforward enhancement to existing graph executor using validated heuristics from Phase 1. No deep specialized domain expertise required.

**General agents are sufficient for:**
- Rust module creation and SQL optimization
- Configuration schema and loading
- Search pipeline integration patterns
- Performance benchmarking (standard tooling)
- Ranking quality evaluation (manual process)

## References

**Related Projects:**
- [SRCHREL Phase 1](../../archive/projects/SRCHREL_relationship-clustering/) - Relationship metadata (complete)

**Codebase References:**
- Graph executor: `crates/maproom/src/search/graph.rs`
- Edge quality heuristics: `crates/maproom/src/search/relationships.rs`
- Chunk edges schema: `crates/maproom/src/db/sqlite/schema.rs`
- Search pipeline: `crates/maproom/src/search/pipeline.rs`

**Documentation:**
- Maproom Architecture: `docs/architecture/MAPROOM_ARCHITECTURE.md`
- Database Architecture: `docs/architecture/DATABASE_ARCHITECTURE.md`
- Search Ranking Guide: `packages/maproom-mcp/docs/search-ranking.md`

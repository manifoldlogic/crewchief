# Multi-Project Overview: Maproom Semantic Search Improvements

## Context

Initiative created: 2025-12-09
Reference: .crewchief/initiatives/2025-12-09_maproom-semantic-search-improvements/

This initiative enhances maproom's semantic code search with improved transparency, filtering, confidence scoring, and relationship-aware features, organized into three sequential phases for maximum early value.

## Projects (in execution order)

### Phase 1: Foundation (Immediate Value)

#### 1. SRCHTRNSP: Search Transparency and Error Diagnostics
**Priority:** Critical
**Effort:** Small (1-2 weeks)
**Dependencies:** None

Replaces generic RPC_ERROR messages with actionable diagnostics and adds query understanding feedback.

**Key Deliverables:**
- Enhanced error serialization preserving daemon details
- Client-side query validation with helpful messages
- Query understanding metadata in responses
- Suggested actions for common failures

**Value:** Directly addresses #1 user pain point (opaque errors). Immediate productivity improvement.

#### 2. SRCHFLTR: Result Type Filtering and Smart Defaults
**Priority:** Critical
**Effort:** Small-Medium (2-3 weeks)
**Dependencies:** None (can run parallel with SRCHTRNSP)

Adds intelligent filtering to focus searches on relevant content types.

**Key Deliverables:**
- Result type filtering (code/docs/tests/all)
- Path-based type inference
- Smart defaults (code-first, exclude archived)
- File extension filtering

**Value:** Directly addresses #2 user pain point (mixed result quality). 50%+ reduction in irrelevant results.

### Phase 2: Intelligence (Enhanced Ranking)

#### 3. SRCHCONF: Confidence Scoring and Progressive Results
**Priority:** High
**Effort:** Medium (2-3 weeks)
**Dependencies:** SRCHTRNSP (for metadata structure), SRCHFLTR (to reduce noise before scoring)

Adds confidence indicators and progressive result filtering.

**Key Deliverables:**
- Score normalization to 0-100 confidence scale
- Confidence bands (high/medium/low)
- Progressive cutoff (exclude low-confidence by default)
- Enhanced debug mode with confidence breakdown

**Value:** Improves ranking transparency. Helps users assess result quality and understand why results surfaced.

#### 4. SRCHREL: Relationship-Aware Search
**Priority:** Medium-High
**Effort:** Medium-Large (3-4 weeks)
**Dependencies:** SRCHCONF (confidence scoring helps rank related results), SRCHFLTR (reduces noise before clustering)

Leverages code relationships to cluster results and surface architectural context.

**Key Deliverables:**
- Related results for high-confidence matches
- Graph traversal using import/call edges
- Relationship-based scoring boost
- Clustering by file proximity and module boundaries

**Value:** Unlocks architectural discovery. Enables "show me related implementations" and cross-cutting concern identification.

### Phase 3: Validation (Comprehensive Testing)

#### 5. SRCHTST: Comprehensive Search Test Suites
**Priority:** High
**Effort:** Medium (2-3 weeks)
**Dependencies:** All Phase 1 & 2 projects (validates enhancements)

Validates semantic understanding and prevents regressions.

**Key Deliverables:**
- Semantic understanding test suite (concept vs keyword)
- Architectural discovery scenarios
- Cross-cutting concern detection tests
- "Grep-impossible" task validation
- Performance regression benchmarks

**Value:** Prevents quality regression. Validates all improvements. Establishes baseline for future enhancements.

## Dependencies

### Execution Order Rationale

**Phase 1: Foundation First**
- SRCHTRNSP and SRCHFLTR can run in parallel (no dependencies)
- Both address immediate user pain points
- Foundation for later enhancements (metadata structure, filtering patterns)

**Phase 2: Intelligence Second**
- SRCHCONF depends on SRCHTRNSP (metadata structure) and SRCHFLTR (cleaner result set to score)
- SRCHREL depends on SRCHCONF (confidence scores help rank related results) and SRCHFLTR (less noise to cluster)
- Both build on Phase 1 foundations

**Phase 3: Validation Last**
- SRCHTST validates all completed enhancements
- Test suite should target production-ready features, not in-progress work
- Establishes regression detection for future work

### Cross-Project Dependencies

```
SRCHTRNSP ─┐
           ├─→ SRCHCONF ─┐
SRCHFLTR ──┤             ├─→ SRCHTST (validates all)
           └─→ SRCHREL ──┘
```

**Dependency Details:**

1. **SRCHCONF → SRCHTRNSP**
   - Confidence metadata uses query understanding structure from SRCHTRNSP
   - Error handling patterns established by SRCHTRNSP

2. **SRCHCONF → SRCHFLTR**
   - Cleaner result set (from filtering) improves confidence calibration
   - Type-filtered results enable per-type confidence thresholds

3. **SRCHREL → SRCHCONF**
   - Confidence scores help rank related results
   - High-confidence results get related chunks, low-confidence do not

4. **SRCHREL → SRCHFLTR**
   - Filtering reduces noise before expensive graph traversal
   - Type filtering enables type-aware relationship weighting

5. **SRCHTST → All**
   - Validates query understanding (SRCHTRNSP)
   - Validates filtering effectiveness (SRCHFLTR)
   - Validates confidence accuracy (SRCHCONF)
   - Validates relationship discovery (SRCHREL)

### Optional Parallelization

**Can Run in Parallel:**
- SRCHTRNSP + SRCHFLTR (Phase 1)
  - No dependencies, different code paths
  - Coordinate on shared response metadata structure

**Must Run Sequentially:**
- Phase 1 → Phase 2 → Phase 3
  - Each phase builds on previous
  - Test suite needs completed features

## Timeline

### Estimated Schedule

**Phase 1 (Weeks 1-4):**
- Week 1-2: SRCHTRNSP (error diagnostics, query understanding)
- Week 2-4: SRCHFLTR (result filtering, smart defaults)
- Parallel development possible, coordinate on response structure

**Phase 2 (Weeks 5-10):**
- Week 5-7: SRCHCONF (confidence scoring, progressive results)
- Week 8-10: SRCHREL (relationship clustering, graph traversal)
- Sequential development recommended (SRCHCONF first)

**Phase 3 (Weeks 11-13):**
- Week 11-13: SRCHTST (test suites, validation, benchmarks)
- Validates all completed work

**Total: 13 weeks (~3 months)**

### Milestone Markers

**End of Phase 1:**
- RPC_ERROR occurrences reduced by 90%
- Result type filtering working reliably
- Query understanding in all responses
- Performance maintained (<100ms p95)

**End of Phase 2:**
- Confidence scores visible for all results
- Related results appear for high-confidence matches
- Performance still maintained (<100ms p95)

**End of Phase 3:**
- Test coverage >80% for semantic scenarios
- All performance benchmarks passing
- Regression detection in place

## Risk Mitigation

### Performance Risk

**Risk:** Enhancements add latency, breaking <100ms p95 target

**Mitigation:**
- <10ms budget per enhancement enforced
- Benchmark before merging each project
- Reject features exceeding budget
- Phase 1 & 2 tested before Phase 3

### Compatibility Risk

**Risk:** Breaking changes to MCP interface affect clients

**Mitigation:**
- Additive-only changes enforced
- New parameters always optional with defaults
- New response fields optional
- Backward compatibility tested

### Scope Creep Risk

**Risk:** Projects expand beyond defined deliverables

**Mitigation:**
- Clear deliverables per project
- Backlog captures deferred ideas
- "Nice-to-have" explicitly separated from "must-have"
- Phased approach allows re-prioritization

### Test Maintenance Risk

**Risk:** Test suite becomes stale or burdensome

**Mitigation:**
- Focus on high-value scenarios only
- Automate test execution in CI
- Golden test sets reviewed quarterly
- Performance tests run on standardized hardware

## Success Criteria (Initiative-Level)

**Must Achieve:**
- [ ] 90% reduction in generic RPC_ERROR occurrences
- [ ] Result type filtering reduces irrelevant results by 50%+
- [ ] Query understanding feedback in all responses
- [ ] Confidence scores for all results
- [ ] Performance maintained (<100ms p95)
- [ ] Test coverage >80% for semantic scenarios

**Should Achieve:**
- [ ] Related results for top search hits
- [ ] Progressive filtering excludes low-confidence results
- [ ] All performance benchmarks passing
- [ ] Zero backward compatibility breaks

**Nice to Have:**
- [ ] Relationship clustering performance <20ms overhead
- [ ] User feedback shows measurable satisfaction improvement
- [ ] Test suite catches regressions in CI

## Handoff to Project Creation

**Ready for `/create-project`:**

Each project below has a summary ready in `decomposition/project-summaries/`:

1. `/create-project SRCHTRNSP_search-transparency` (from SRCHTRNSP summary)
2. `/create-project SRCHFLTR_result-filtering` (from SRCHFLTR summary)
3. `/create-project SRCHCONF_confidence-scoring` (from SRCHCONF summary)
4. `/create-project SRCHREL_relationship-search` (from SRCHREL summary)
5. `/create-project SRCHTST_search-test-suites` (from SRCHTST summary)

**Execution Strategy:**

- Start with SRCHTRNSP and SRCHFLTR (parallel, Phase 1)
- Complete Phase 1 before starting Phase 2
- SRCHCONF before SRCHREL (sequential, Phase 2)
- SRCHTST validates all (Phase 3)

**Next Steps:**

1. Review and approve multi-project plan
2. Create project summaries (in progress)
3. Execute `/create-project` for Phase 1 projects
4. Begin implementation

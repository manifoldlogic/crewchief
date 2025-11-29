# AGENTOPT Ticket Index

## Project: AI Agent Query Optimization

**Goal**: Improve semantic search query quality for AI agents through data-driven optimization of MCP tool descriptions.

**Approach**: Empirical testing with competitive variants, statistical analysis, and continuous improvement.

---

## Phase 0: Data-Driven Testing Framework (FOUNDATIONAL)

**Purpose**: Establish empirical testing infrastructure for discovering optimal tool descriptions through competitive A/B testing and genetic algorithm iterations.

**Duration**: 2-3 weeks
**Priority**: HIGH (enables data-driven optimization)

| Ticket | Title | Status | Agent | Dependencies |
|--------|-------|--------|-------|--------------|
| [AGENTOPT-0001](./AGENTOPT-0001_create-test-query-set.md) | Create Test Query Set (100 Queries) | ⬜ Pending | general-purpose | None |
| [AGENTOPT-0002](./AGENTOPT-0002_variant-generation-system.md) | Build Variant Generation System | ⬜ Pending | general-purpose | 0001 |
| [AGENTOPT-0003](./AGENTOPT-0003_implement-automated-testing-harness.md) | Implement Automated Testing Harness | ⬜ Pending | general-purpose | 0001, 0002 |
| [AGENTOPT-0004](./AGENTOPT-0004_statistical-analysis-framework.md) | Build Statistical Analysis Framework | ⬜ Pending | general-purpose | 0003 |
| [AGENTOPT-0005](./AGENTOPT-0005_production-ab-testing-infrastructure.md) | Deploy Production A/B Testing Infrastructure | ⬜ Pending | general-purpose | 0002, 0004 |
| [AGENTOPT-0006](./AGENTOPT-0006_phase-0-integration-first-experiment.md) | Phase 0 Integration and First Experiment | ⬜ Pending | general-purpose | 0001-0005 |

**Key Deliverables**:
- 100-query test set with gold standards
- Variant generation system with genetic mutations
- Automated testing harness with agent simulation
- Statistical analysis framework (t-tests, p<0.05)
- Production A/B testing infrastructure
- First experiment results identifying optimal variant

---

## Phase 1: Enhanced Tool Description (PRIORITY)

**Purpose**: Deploy empirically-validated optimal tool description from Phase 0 testing.

**Duration**: 1 week
**Effort**: 8-12 hours
**Risk**: Minimal (easy rollback, no infrastructure changes)

| Ticket | Title | Status | Agent | Dependencies |
|--------|-------|--------|-------|--------------|
| [AGENTOPT-1001](./AGENTOPT-1001_design-enhanced-tool-description.md) | Design Enhanced Tool Description | ⬜ Pending | general-purpose | 0006 |
| [AGENTOPT-1002](./AGENTOPT-1002_implement-enhanced-tool-description.md) | Implement Enhanced Tool Description in MCP Server | ⬜ Pending | general-purpose | 1001 |
| [AGENTOPT-1003](./AGENTOPT-1003_testing-and-validation.md) | Testing and Validation (Before/After Comparison) | ⬜ Pending | general-purpose | 1002 |
| [AGENTOPT-1004](./AGENTOPT-1004_code-review-approval.md) | Code Review and Approval | ⬜ Pending | general-purpose | 1002, 1003 |
| [AGENTOPT-1005](./AGENTOPT-1005_deploy-enhanced-description-production.md) | Deploy Enhanced Description to Production | ⬜ Pending | general-purpose | 1004 |
| [AGENTOPT-1006](./AGENTOPT-1006_monitoring-validation.md) | Monitoring and Validation (1 Week) | ⬜ Pending | general-purpose | 1005 |

**Success Criteria**:
- Natural language query success: ≥70% (vs 10% baseline)
- Simple query success: ≥80% (no degradation)
- Token budget: <600 tokens
- MCP schema: Valid
- No increase in error rates

**Expected Impact**:
- +40-60 percentage points in query success rate
- $0 cost (zero infrastructure changes, zero latency)

---

## Phase 2: Server-Side Preprocessing (OPTIONAL)

**Purpose**: Add server-side query normalization for all MCP clients (not just Claude Code).

**Trigger**: Phase 1 shows ≥50% improvement but <70% target
**Duration**: 1 week
**Effort**: 16-24 hours

| Ticket | Title | Status | Agent | Dependencies |
|--------|-------|--------|-------|--------------|
| [AGENTOPT-2001](./AGENTOPT-2001_query-preprocessing-module.md) | Implement Query Preprocessing Module | ⬜ Optional | rust-indexer-engineer | 1006 |
| [AGENTOPT-2002](./AGENTOPT-2002_metadata-score-boosting.md) | Implement Metadata Score Boosting | ⬜ Optional | rust-indexer-engineer | 2001 |
| [AGENTOPT-2003](./AGENTOPT-2003_phase2-testing-deployment.md) | Phase 2 Testing and Deployment | ⬜ Optional | general-purpose | 2001, 2002 |

**Key Features**:
- Stop word removal (how, what, where, when, why)
- Whitespace normalization
- Path-based scoring boosts (src/ vs tests)
- Name matching bonus
- <5ms added latency

**Expected Impact**:
- Additional +15-25% quality improvement (additive with Phase 1)

---

## Phase 3: LLM Fallback (OPTIONAL)

**Purpose**: Handle edge cases with Haiku-based query rewriting when standard approaches fail.

**Trigger**: Phase 1+2 deployed but edge cases remain
**Duration**: 2 weeks
**Effort**: 24-32 hours

| Ticket | Title | Status | Agent | Dependencies |
|--------|-------|--------|-------|--------------|
| [AGENTOPT-3001](./AGENTOPT-3001_llm-fallback-implementation.md) | Implement LLM Fallback for Edge Cases | ⬜ Optional | rust-indexer-engineer | 2003 |
| [AGENTOPT-3002](./AGENTOPT-3002_phase3-testing-deployment.md) | Phase 3 Testing and Deployment | ⬜ Optional | general-purpose | 3001 |

**Key Features**:
- Haiku-based query rewriting
- Automatic fallback on 0 results or low confidence
- Cost monitoring and alerts
- Feature flag (opt-in initially)
- ~$1-2/month per 100 users

**Expected Impact**:
- +40-60% quality on edge cases
- Fallback triggers <15% of queries

---

## Timeline

```
Week 1-2: Phase 0 (Testing Infrastructure)
├─ Days 1-2: Test query set + variant system (0001, 0002)
├─ Days 3-4: Testing harness + statistical analysis (0003, 0004)
├─ Days 5-6: A/B testing infrastructure (0005)
└─ Day 7: First experiment (0006)

Week 3: Phase 1 (Enhanced Description)
├─ Day 1: Design (1001) + Implement (1002)
├─ Day 2: Testing (1003)
├─ Day 3: Code review (1004) + Deploy (1005)
└─ Days 4-7: Monitoring (1006)

Week 4: Phase 2 Decision
├─ Analyze Phase 1 results
├─ Go/No-Go for Phase 2
└─ If yes: Implement 2001-2003 (1 week)

Week 5+: Phase 3 Decision (if needed)
```

---

## Success Metrics

### Phase 0 (Testing Framework)
- [x] 100-query test set created
- [x] 5 initial variants generated
- [x] Testing harness runs full experiment in <30 minutes
- [x] Statistical analyzer identifies winners (p<0.05)
- [x] A/B testing infrastructure deployable

### Phase 1 (Enhanced Description)
- [ ] Natural language success: ≥70% (vs 10% baseline)
- [ ] Simple query success: ≥80% (no degradation)
- [ ] Token budget: <600 tokens
- [ ] MCP schema: Valid
- [ ] User feedback: Positive or neutral
- [ ] Deployed to production
- [ ] Metrics collected for 1 week

### Phase 2 (Server Preprocessing - if triggered)
- [ ] Additional +15-25% quality improvement
- [ ] <5ms latency added
- [ ] No regressions
- [ ] A/B test validates improvement

### Phase 3 (LLM Fallback - if triggered)
- [ ] Fallback success rate >50%
- [ ] Monthly cost <$150 at 100 users
- [ ] Fallback triggers <15% of queries
- [ ] Edge case success rate +40-60%

---

## Quick Reference

**Project Location**: `.crewchief/projects/AGENTOPT_ai-agent-query-optimization/`

**Planning Documents**:
- [README.md](../README.md) - Project overview
- [planning/analysis.md](../planning/analysis.md) - Problem analysis
- [planning/architecture.md](../planning/architecture.md) - Solution design
- [planning/plan.md](../planning/plan.md) - Phase breakdown
- [planning/quality-strategy.md](../planning/quality-strategy.md) - Testing approach

**Key Files**:
- `packages/maproom-mcp/src/index.ts` (lines 117-155) - Tool description
- `packages/maproom-mcp/test/tool-description-optimization/` - Testing framework
- `crates/maproom/src/query/` - Server-side preprocessing (Phase 2/3)

**Workflow**: /single-ticket [ticket-id] or /work-on-project AGENTOPT

---

Last Updated: 2025-01-06

# TOOLOPT Ticket Index

Project: Maproom Search Tool Optimization
Created: 2025-11-14
Total Tickets: 14 (5 Phase 1 + 5 Phase 2 + 4 Phase 3)

## Overview

This project applies genetic optimization learnings to improve the Maproom semantic search tool description, targeting >20% performance on AI agent code search benchmarks.

**Key Results from Genetic Optimization:**
- Baseline (variant-control): 17.7%
- Winner (variant-a-detailed): 19.6% (+1.9%)
- Performance plateaued at 19-20% across Gen 2-10
- Critical gap identified: task-to-query mapping

**Project Phases:**
1. **Documentation** (3-4 hours) - Preserve genetic optimization learnings
2. **Production Deployment** (2 hours) - Deploy proven winner
3. **Enhancement Creation** (1.5 hours) - Create variant targeting >20%

## Phase 1: Documentation (TOOLOPT-1xxx)

**Goal**: Create permanent documentation capturing genetic optimization insights

| Ticket ID | Title | Status | Time | Dependencies |
|-----------|-------|--------|------|--------------|
| TOOLOPT-1001 | Create documentation directory structure | ⏳ Pending | 15m | None |
| TOOLOPT-1002 | Document genetic optimization results | ⏳ Pending | 1-1.5h | 1001 |
| TOOLOPT-1003 | Create reusable pattern guide | ⏳ Pending | 45m | 1002 |
| TOOLOPT-1004 | Export variant examples with annotations | ⏳ Pending | 30m | 1002 |
| TOOLOPT-1005 | Review and refine documentation | ⏳ Pending | 45m | 1002, 1003, 1004 |

**Phase 1 Total**: 3-4 hours

**Deliverables**:
- `docs/optimization/README.md`
- `docs/optimization/genetic-optimization-results.md`
- `docs/optimization/tool-description-patterns.md`
- `docs/optimization/examples/variant-a-detailed.md`
- `docs/optimization/examples/variant-control.md`
- `docs/optimization/examples/variant-e-task-mapping.md` (placeholder)

**Success Criteria**:
- [ ] All genetic optimization insights documented
- [ ] Patterns clearly explained with examples
- [ ] Documentation standalone (no conversation context needed)
- [ ] External review approved

---

## Phase 2: Production Deployment (TOOLOPT-2xxx)

**Goal**: Deploy variant-a-detailed (19.6% winner) to production MCP server

| Ticket ID | Title | Status | Time | Dependencies |
|-----------|-------|--------|------|--------------|
| TOOLOPT-2001 | Validate winner performance | ⏳ Pending | 45m | None |
| TOOLOPT-2002 | Update tool description | ⏳ Pending | 15m | 2001 |
| TOOLOPT-2003 | Integration test | ⏳ Pending | 20m | 2002 |
| TOOLOPT-2004 | Create deployment PR | ⏳ Pending | 15m | 2001, 2002, 2003 |
| TOOLOPT-2005 | Deploy to production | ⏳ Pending | 15m | 2004 |

**Phase 2 Total**: 2 hours

**Deliverables**:
- Updated `packages/maproom-mcp/src/tools/search.ts`
- Validation test results (≥19.0%)
- Integration test confirmation
- PR with evidence
- Production deployment

**Success Criteria**:
- [ ] Validation shows ≥19.0% performance
- [ ] MCP server deploys without errors
- [ ] Tool description matches variant-a-detailed
- [ ] Post-deployment spot check confirms functionality
- [ ] No regressions detected

---

## Phase 3: Enhancement Creation (TOOLOPT-3xxx)

**Goal**: Create enhanced variant with task-to-query mapping targeting >20%

| Ticket ID | Title | Status | Time | Dependencies |
|-----------|-------|--------|------|--------------|
| TOOLOPT-3001 | Design task-to-query mapping section | ⏳ Pending | 30m | None |
| TOOLOPT-3002 | Create variant-e-task-mapping | ⏳ Pending | 15m | 3001 |
| TOOLOPT-3003 | Validate variant format | ⏳ Pending | 15m | 3002 |
| TOOLOPT-3004 | Document enhancement | ⏳ Pending | 20m | 3002, 3003 |

**Phase 3 Total**: 1.5 hours

**Deliverables**:
- `packages/maproom-mcp/test/tool-description-optimization/variants/variant-e-task-mapping.json`
- `docs/optimization/examples/variant-e-task-mapping.md`
- Enhancement rationale documented

**Success Criteria**:
- [ ] Task-to-query section properly formatted
- [ ] variant-e-task-mapping created and validated
- [ ] Token count within budget (<600 tokens)
- [ ] Enhancement documented with rationale
- [ ] Ready for next genetic optimization run

---

## Execution Plan

### Sequential Execution (Recommended)

```bash
# Execute all tickets in order
/work-on-project TOOLOPT
```

### Phase-by-Phase Execution

```bash
# Phase 1: Documentation
/single-ticket TOOLOPT-1001
/single-ticket TOOLOPT-1002
/single-ticket TOOLOPT-1003  # Can run parallel with 1004
/single-ticket TOOLOPT-1004  # Can run parallel with 1003
/single-ticket TOOLOPT-1005

# Phase 2: Production Deployment
/single-ticket TOOLOPT-2001
/single-ticket TOOLOPT-2002
/single-ticket TOOLOPT-2003
/single-ticket TOOLOPT-2004
/single-ticket TOOLOPT-2005

# Phase 3: Enhancement
/single-ticket TOOLOPT-3001
/single-ticket TOOLOPT-3002
/single-ticket TOOLOPT-3003
/single-ticket TOOLOPT-3004
```

### Critical Path

```
1001 (foundation)
  ↓
1002 (core docs) ────┬────→ 1005 (review)
  ↓                  ↓
  ├→ 1003 (patterns) ┘
  └→ 1004 (examples) ┘

[Documentation Complete]

2001 (validate) ───────┐
  ↓                    ↓
2002 (update code)     ↓
  ↓                    ↓
2003 (test)            ↓
  ↓                    ↓
2004 (PR) ←────────────┘
  ↓
2005 (deploy)

[Production Deployment Complete]

3001 (design)
  ↓
3002 (create variant)
  ↓
3003 (validate) ───┐
  ↓                ↓
3004 (document) ←──┘

[Enhancement Complete]
```

---

## Agent Assignments

All tickets use **general-purpose** agent with different focuses:

- **Phase 1**: Writing and documentation
- **Phase 2**: Testing, deployment, git operations
- **Phase 3**: Design, JSON manipulation, validation

---

## Reference Documents

### Project Planning
- [README.md](../README.md) - Project overview
- [planning/analysis.md](../planning/analysis.md) - Genetic optimization findings
- [planning/architecture.md](../planning/architecture.md) - Solution design
- [planning/plan.md](../planning/plan.md) - Detailed implementation plan
- [planning/quality-strategy.md](../planning/quality-strategy.md) - Testing approach
- [planning/security-review.md](../planning/security-review.md) - Security assessment

### Source Data
- Genetic iterations: `/workspace/packages/cli/.crewchief/genetic-iterations/ultra-run-1763154816350/`
- Variants: `ultra-run-1763154816350/variants/`
- Generation reports: `ultra-run-1763154816350/gen-*/report.txt`

### Key Files
- MCP tool: `/workspace/packages/maproom-mcp/src/tools/search.ts`
- Variants dir: `/workspace/packages/maproom-mcp/test/tool-description-optimization/variants/`
- Documentation: `/workspace/docs/optimization/`

---

## Progress Tracking

**Phase 1**: ⏳ Not Started (0/5 tickets)
**Phase 2**: ⏳ Not Started (0/5 tickets)
**Phase 3**: ⏳ Not Started (0/4 tickets)

**Overall**: 0/14 tickets completed (0%)

---

## Notes

- Phase 1 and Phase 3 can be parallelized if desired
- Phase 2 requires Phase 1 completion (documentation should exist before deployment)
- All tickets include rollback plans where applicable
- Post-deployment monitoring recommended for Phase 2
- Enhancement variant (Phase 3) ready for future genetic testing

# Implementation Plan: Maproom Search Tool Optimization

## Executive Summary

Apply genetic optimization learnings through a three-phase approach:
1. **Document learnings** (permanent knowledge capture)
2. **Deploy proven winner** (immediate +1.9% performance gain)
3. **Create enhancement** (future >20% opportunity)

**Timeline**: 4-6 hours total
**Risk**: Low (content-only change)
**Value**: Immediate performance improvement + preserved knowledge

## Phase 1: Document Genetic Optimization Learnings

**Goal**: Create permanent documentation capturing all insights from 10-generation experiment

**Agent**: Technical Documentation Specialist

### Deliverables

1. **Documentation Structure**
   - Create `docs/optimization/` directory
   - Write comprehensive README
   - Document winning patterns
   - Document anti-patterns
   - Include variant examples

2. **Content Requirements**
   - Quantitative results (scores, token counts, correlations)
   - Qualitative insights (structural patterns, tone analysis)
   - Pattern analysis (winners vs losers)
   - Enhancement recommendations
   - Future research directions

3. **Quality Criteria**
   - Standalone (no conversation context needed)
   - Actionable (clear takeaways for tool descriptions)
   - Evidence-based (all claims traceable to source data)
   - Comprehensive (covers all key findings)

### Tasks

#### Task 1.1: Create Documentation Structure
- [ ] Create `docs/optimization/` directory
- [ ] Create `docs/optimization/README.md` (overview)
- [ ] Create `docs/optimization/genetic-optimization-results.md` (detailed findings)
- [ ] Create `docs/optimization/tool-description-patterns.md` (reusable patterns)
- [ ] Create `docs/optimization/examples/` subdirectory

**Estimated Time**: 15 minutes
**Acceptance Criteria**: Directory structure exists, empty files created

#### Task 1.2: Write Genetic Optimization Results
- [ ] Document performance progression (gen 0-10)
- [ ] Include variant scores and rankings
- [ ] Analyze winning patterns (transformation workflow, structure, tone)
- [ ] Document anti-patterns (static examples, over-documentation)
- [ ] Present quantitative analysis (token counts, correlations)

**Estimated Time**: 1-1.5 hours
**Acceptance Criteria**: Comprehensive results documented with tables, examples, analysis

#### Task 1.3: Document Reusable Patterns
- [ ] Extract winning patterns as templates
- [ ] Create "how-to" guide for tool descriptions
- [ ] Document transformation workflow pattern
- [ ] Provide examples of good/bad patterns
- [ ] Include decision tree for choosing patterns

**Estimated Time**: 45 minutes
**Acceptance Criteria**: Actionable guide for future tool description work

#### Task 1.4: Create Example Variants
- [ ] Export variant-a-detailed (winner) to markdown
- [ ] Export variant-control (baseline) to markdown
- [ ] Export variant-e-task-mapping (enhancement) to markdown
- [ ] Add annotations explaining key sections
- [ ] Highlight differences between variants

**Estimated Time**: 30 minutes
**Acceptance Criteria**: 3 variant examples with annotations

#### Task 1.5: Documentation Review
- [ ] External reviewer reads documentation
- [ ] Checks for clarity and completeness
- [ ] Verifies accuracy against source data
- [ ] Provides feedback
- [ ] Author addresses feedback

**Estimated Time**: 30 minutes (reviewer) + 15 minutes (revisions)
**Acceptance Criteria**: Documentation approved by reviewer

### Phase 1 Output

- `docs/optimization/README.md` ✅
- `docs/optimization/genetic-optimization-results.md` ✅
- `docs/optimization/tool-description-patterns.md` ✅
- `docs/optimization/examples/variant-a-detailed.md` ✅
- `docs/optimization/examples/variant-control.md` ✅
- `docs/optimization/examples/variant-e-task-mapping.md` ✅

**Total Time**: 3-4 hours
**Success Metric**: Documentation published and merged to main

---

## Phase 2: Deploy Proven Winner to Production

**Goal**: Update MCP server tool description with variant-a-detailed (19.6% performer)

**Agent**: MCP Server Configuration Specialist

### Deliverables

1. **Updated Tool Description**
   - Replace control variant with detailed variant
   - Maintain backward compatibility
   - No API changes

2. **Validation Results**
   - Benchmark test showing ≥19.0% performance
   - Integration test confirming MCP server works
   - Agent interaction spot checks

3. **Deployment**
   - PR with test evidence
   - Code review approval
   - Merge and deploy

### Tasks

#### Task 2.1: Pre-Deployment Validation
- [ ] Run benchmark comparison test
  ```bash
  npx tsx src/search-optimization/run-comparison.ts \
    variant-control variant-a-detailed \
    --tasks=impl-worktree-001 --iterations=5
  ```
- [ ] Verify variant-a-detailed scores ≥19.0%
- [ ] Review test output for quality signals
- [ ] Document test results

**Estimated Time**: 45 minutes (30 min test runtime + 15 min analysis)
**Acceptance Criteria**: Test shows ≥19.0% performance, documented results

#### Task 2.2: Update Tool Description
- [ ] Locate current description in `packages/maproom-mcp/src/tools/search.ts`
- [ ] Read variant-a-detailed description from JSON
- [ ] Replace description field with new content
- [ ] Verify TypeScript compilation
- [ ] Run `pnpm build`

**Estimated Time**: 15 minutes
**Acceptance Criteria**: Code compiles, build succeeds

#### Task 2.3: Integration Testing
- [ ] Start MCP server with new description
  ```bash
  cd packages/maproom-mcp
  node dist/index.js
  ```
- [ ] Verify server starts without errors
- [ ] Check tool description in tool list
- [ ] Confirm description matches variant-a-detailed
- [ ] Test agent interaction (2-3 sample searches)

**Estimated Time**: 20 minutes
**Acceptance Criteria**: Server works, agents can use tool successfully

#### Task 2.4: Create Pull Request
- [ ] Commit changes with clear message
- [ ] Create PR with description
- [ ] Include test results in PR description
- [ ] Link to documentation
- [ ] Request code review

**Estimated Time**: 15 minutes
**Acceptance Criteria**: PR created with complete context

#### Task 2.5: Deploy to Production
- [ ] Merge approved PR
- [ ] Rebuild packages (`pnpm build`)
- [ ] Restart MCP server
- [ ] Verify deployment successful
- [ ] Run post-deployment spot check

**Estimated Time**: 15 minutes
**Acceptance Criteria**: New description live in production

### Phase 2 Output

- Updated `packages/maproom-mcp/src/tools/search.ts` ✅
- Validation test results documented ✅
- PR merged to main ✅
- Production deployment successful ✅

**Total Time**: 2 hours
**Success Metric**: Production MCP server using variant-a-detailed

---

## Phase 3: Create Enhanced Variant for Future Testing

**Goal**: Develop variant-e-task-mapping with task-to-query mapping section

**Agent**: Tool Description Optimization Specialist

### Deliverables

1. **Enhanced Variant**
   - Clone variant-a-detailed
   - Add task-to-query mapping section
   - Store as variant-e-task-mapping.json

2. **Documentation**
   - Document enhancement rationale
   - Explain task-to-query mapping pattern
   - Provide examples

3. **Test Readiness**
   - Variant ready for genetic optimization run
   - Formatted correctly
   - Token count within budget

### Tasks

#### Task 3.1: Design Task-to-Query Section
- [ ] Review analysis findings on missing gap
- [ ] Design section structure:
  ```markdown
  🎯 TASK-TO-QUERY MAPPING:

  FINDING IMPLEMENTATION:
  Task: "Find where X is implemented"
  Query: "[component] [action]"
  Examples: ...

  UNDERSTANDING ARCHITECTURE:
  Task: "Understand how X works"
  Query: "[system] [flow/architecture]"
  Examples: ...

  [etc.]
  ```
- [ ] Write content for 4 task categories
- [ ] Create concrete examples

**Estimated Time**: 30 minutes
**Acceptance Criteria**: Section designed and drafted

#### Task 3.2: Create Enhanced Variant
- [ ] Copy variant-a-detailed.json → variant-e-task-mapping.json
- [ ] Insert task-to-query section after transformation patterns
- [ ] Update metadata (id, name, generation, parent_ids)
- [ ] Calculate new token count
- [ ] Verify JSON validity

**Estimated Time**: 15 minutes
**Acceptance Criteria**: Valid JSON file created

#### Task 3.3: Validate Variant Format
- [ ] Load variant in test framework
- [ ] Verify schema compliance
- [ ] Check token count (<600 tokens)
- [ ] Review section ordering
- [ ] Confirm example quality

**Estimated Time**: 15 minutes
**Acceptance Criteria**: Variant passes validation checks

#### Task 3.4: Document Enhancement
- [ ] Add variant to `docs/optimization/examples/`
- [ ] Document design rationale
- [ ] Explain expected impact (+0.5-1.0%)
- [ ] Provide testing recommendations

**Estimated Time**: 20 minutes
**Acceptance Criteria**: Enhancement documented

### Phase 3 Output

- `packages/maproom-mcp/test/tool-description-optimization/variants/variant-e-task-mapping.json` ✅
- `docs/optimization/examples/variant-e-task-mapping.md` ✅
- Enhancement rationale documented ✅

**Total Time**: 1.5 hours
**Success Metric**: Enhanced variant ready for next genetic run

---

## Agent Assignments

### Phase 1: Documentation
**Agent**: General-purpose agent (writing-focused task)
**Capabilities Needed**:
- File read/write
- Markdown formatting
- Data analysis and synthesis
- Documentation best practices

**Alternative**: Create custom documentation-specialist agent if recurring need

### Phase 2: Production Deployment
**Agent**: General-purpose agent (code modification + testing)
**Capabilities Needed**:
- File editing
- Command execution (build, test)
- Result validation
- PR creation

**Alternative**: Use existing agents - no specialized agent needed

### Phase 3: Variant Creation
**Agent**: General-purpose agent (JSON manipulation + design)
**Capabilities Needed**:
- File read/write
- JSON formatting
- Token counting
- Schema validation

**Alternative**: Manual creation (small task, low complexity)

---

## Timeline & Dependencies

```
Week 1:
├── Day 1-2: Phase 1 (Documentation)
│   ├── Task 1.1: Structure (0.25h)
│   ├── Task 1.2: Results (1.5h)
│   ├── Task 1.3: Patterns (0.75h)
│   ├── Task 1.4: Examples (0.5h)
│   └── Task 1.5: Review (0.75h)
│   └─── Total: 3.75 hours
│
├── Day 3: Phase 2 (Deployment)
│   ├── Task 2.1: Validation (0.75h) ← BLOCKS 2.2
│   ├── Task 2.2: Update (0.25h)
│   ├── Task 2.3: Integration (0.33h)
│   ├── Task 2.4: PR (0.25h)
│   └── Task 2.5: Deploy (0.25h)
│   └─── Total: 1.83 hours
│
└── Day 4: Phase 3 (Enhancement)
    ├── Task 3.1: Design (0.5h)
    ├── Task 3.2: Create (0.25h)
    ├── Task 3.3: Validate (0.25h)
    └── Task 3.4: Document (0.33h)
    └─── Total: 1.33 hours

Grand Total: ~7 hours
```

**Critical Path**: Phase 1 → Phase 2 (documentation should exist before deploying)

**Parallelization**: Phase 3 can overlap with Phase 1-2 (independent)

---

## Success Criteria

### Phase 1 Success
- [ ] Documentation published to `docs/optimization/`
- [ ] All genetic optimization insights captured
- [ ] Patterns clearly documented
- [ ] Examples include comparisons
- [ ] External reviewer approval

### Phase 2 Success
- [ ] Validation test shows ≥19.0% performance
- [ ] MCP server deploys without errors
- [ ] Tool description matches variant-a-detailed
- [ ] Post-deployment spot check confirms functionality
- [ ] No regressions detected

### Phase 3 Success
- [ ] variant-e-task-mapping created and validated
- [ ] Task-to-query section properly formatted
- [ ] Token count within budget (<600 tokens)
- [ ] Enhancement documented with rationale
- [ ] Ready for next genetic optimization run

### Overall Project Success
All phase criteria met AND:
- [ ] +1.9% performance gain achieved in production
- [ ] Knowledge preserved for future tool description work
- [ ] Enhancement variant ready for future testing
- [ ] Team understands patterns and can apply to other tools

---

## Risk Management

### Risk: Validation Test Fails (<19.0%)

**Mitigation**:
- Run 5 iterations to account for variance
- Set threshold at 19.0% (0.6% margin)
- Use same environment as genetic experiment

**Contingency**:
- If 17.0-18.9%: Don't deploy, investigate variance
- If >18.9%: Deploy with caution, monitor closely
- If test error: Fix test setup, retry

### Risk: Documentation Goes Stale

**Mitigation**:
- Place in `/docs/` for visibility
- Link from main README
- Schedule quarterly review

**Contingency**:
- Refresh when stale
- Archive if obsolete

### Risk: Production Issues After Deployment

**Mitigation**:
- Pre-deployment validation
- Post-deployment monitoring
- Rollback plan ready

**Contingency**:
```bash
git revert <commit-sha>
pnpm build
# Restart MCP server
```

### Risk: Enhancement Doesn't Improve Performance

**Mitigation**:
- Test in genetic run before production
- Keep as optional variant
- Don't deploy if <19.6%

**Contingency**:
- Learn from failure
- Try alternative enhancements
- Document why it didn't work

---

## Post-Deployment Monitoring

### Week 1: Active Monitoring

**Daily Checks**:
- [ ] MCP server logs (error rates)
- [ ] Agent conversation samples (query quality)
- [ ] Search call frequency
- [ ] Performance metrics

**Alert Conditions**:
- Error rate increase >5%
- Success rate drop >2%
- Anomalous query patterns

**Actions**:
- Investigate anomalies immediately
- Document findings
- Rollback if critical issues

### Week 2-4: Passive Monitoring

**Weekly Checks**:
- [ ] Aggregate error logs
- [ ] Query pattern analysis
- [ ] Performance trends

**Alert Conditions**:
- Sustained performance degradation
- New error types appearing

### Month 2+: Maintenance

**Quarterly Reviews**:
- [ ] Documentation accuracy
- [ ] Performance trends
- [ ] New pattern discoveries
- [ ] Enhancement opportunities

---

## Deliverables Checklist

### Documentation Deliverables
- [ ] `docs/optimization/README.md`
- [ ] `docs/optimization/genetic-optimization-results.md`
- [ ] `docs/optimization/tool-description-patterns.md`
- [ ] `docs/optimization/examples/variant-a-detailed.md`
- [ ] `docs/optimization/examples/variant-control.md`
- [ ] `docs/optimization/examples/variant-e-task-mapping.md`

### Code Deliverables
- [ ] Updated `packages/maproom-mcp/src/tools/search.ts`
- [ ] New `packages/maproom-mcp/test/tool-description-optimization/variants/variant-e-task-mapping.json`

### Test Deliverables
- [ ] Validation test results (comparison report)
- [ ] Integration test confirmation
- [ ] Post-deployment spot check results

### Process Deliverables
- [ ] PR with test evidence
- [ ] Code review approval
- [ ] Deployment confirmation
- [ ] Monitoring setup

---

## Conclusion

This plan delivers immediate value (+1.9% performance) while preserving knowledge and setting up future improvements. The three-phase approach balances:

- **Short-term wins**: Deploy proven winner
- **Knowledge preservation**: Document learnings
- **Future opportunity**: Create enhancement

**Execution**: Sequential phases over 4-6 hours total, low risk, high value.

**Next Steps**: Begin Phase 1 (Documentation) → validate Phase 2 → execute Phase 3.

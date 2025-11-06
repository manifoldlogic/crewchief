# Project Plan: AI Agent Query Optimization

## Overview

**Goal**: Improve semantic search query quality for AI agents by enhancing MCP tool descriptions with query transformation guidance.

**Approach**: Leverage agent intelligence (zero cost) rather than server infrastructure (costly) by teaching Claude Code how to formulate optimal queries.

**Scope**: Phase 1 only (enhanced tool description). Future phases optional based on results.

## Phases

### Phase 1: Enhanced Tool Description (PRIORITY)

**Duration**: 1 week
**Effort**: 8-12 hours
**Dependencies**: None
**Risk**: Minimal

**Deliverables**:
1. Enhanced search tool description with transformation patterns
2. Before/after testing (20 queries)
3. Deployment to production
4. Success metrics collected

**Success Criteria**:
- Natural language query success: 70% (vs 10% baseline)
- Simple query success: Maintained at 80%
- Token budget: <600 tokens
- MCP schema: Valid

---

## Phase 1 Breakdown

### Step 1: Design Enhanced Description (2-3 hours)

**Owner**: MCP developer + Product

**Tasks**:
1. Review current tool description (30 min)
2. Draft transformation patterns (1 hour)
3. Write good/bad query examples (30 min)
4. Document multi-query retry strategy (30 min)
5. Internal review and iteration (30 min)

**Output**: Enhanced description draft in markdown

**Dependencies**: None

**Blockers**: None

### Step 2: Implementation (1-2 hours)

**Owner**: MCP developer

**Tasks**:
1. Update `packages/maproom-mcp/src/index.ts` (30 min)
   - Replace tool description string (lines 117-155)
   - Maintain MCP schema compatibility
2. Check token count (10 min)
3. Validate MCP schema (10 min)
4. Build and test locally (30 min)

**Output**: Code changes ready for review

**Dependencies**: Step 1 complete

**Blockers**: None

### Step 3: Testing and Validation (3-4 hours)

**Owner**: QA + Developer

**Tasks**:
1. Create test query set (1 hour)
   - 10 natural language queries
   - 5 simple 2-3 word queries
   - 5 complex multi-word queries
2. Run baseline tests (pre-enhancement) (30 min)
3. Run enhanced tests (post-enhancement) (30 min)
4. Compare results and analyze (1 hour)
5. Spot-check top-3 relevance (30 min)
6. Observe agent retry behavior (30 min)

**Output**: Test report with before/after metrics

**Dependencies**: Step 2 complete

**Blockers**: Need access to Claude Code for testing

### Step 4: Code Review and Approval (1 hour)

**Owner**: Senior developer

**Tasks**:
1. Review enhanced description for clarity (30 min)
2. Verify token budget (5 min)
3. Check MCP schema validation (5 min)
4. Review test results (20 min)

**Output**: Approved PR

**Dependencies**: Step 3 complete

**Blockers**: None

### Step 5: Deployment (1 hour)

**Owner**: DevOps + Developer

**Tasks**:
1. Git tag: `v1.X.0-agent-opt` (5 min)
2. Build production artifacts (10 min)
3. Deploy to production (10 min)
4. Verify server restart (5 min)
5. Smoke test with real queries (30 min)

**Output**: Enhanced description live in production

**Dependencies**: Step 4 complete

**Blockers**: None

### Step 6: Monitoring and Validation (1 week passive)

**Owner**: Product + Developer

**Tasks**:
1. Monitor error rates (daily)
2. Track query success metrics (daily)
3. Collect user feedback (ongoing)
4. Analyze agent retry behavior (weekly)

**Output**: Metrics report, go/no-go for Phase 2

**Dependencies**: Step 5 complete

**Blockers**: None

---

## Timeline

### Week 1

```
Day 1:
  Morning:  Design enhanced description (Step 1)
  Afternoon: Implementation (Step 2)

Day 2:
  Morning:  Create test query set
  Afternoon: Run baseline and enhanced tests (Step 3)

Day 3:
  Morning:  Analyze test results, spot-check relevance
  Afternoon: Code review and approval (Step 4)

Day 4:
  Morning:  Deployment (Step 5)
  Afternoon: Smoke testing, monitor

Day 5-7:
  Passive monitoring (Step 6)
  Collect feedback
```

**Total active effort**: 8-12 hours over 3 days

---

## Resource Allocation

### Personnel

**Primary**:
- 1 MCP developer (8-10 hours)
- 1 QA engineer (3-4 hours)

**Secondary**:
- 1 Senior developer (1 hour for code review)
- Product owner (1 hour for description review)

**Total**: ~13-16 person-hours

### Infrastructure

**Required**:
- Development environment (existing)
- Claude Code instance for testing (existing)
- Git repository (existing)
- MCP server (existing)

**New**: None

**Cost**: $0

---

## Dependencies

### External Dependencies

1. **MCP Protocol**: Stable (no changes expected)
2. **Claude Code**: Stable (API unchanged)
3. **PostgreSQL**: No changes needed

### Internal Dependencies

1. **Existing MCP server**: Must be functional
2. **Search functionality**: Must be working at baseline
3. **Deployment pipeline**: Must be operational

### Blocked By

- None (no external blockers)

---

## Risks and Mitigation

### Risk 1: Agent Doesn't Follow Guidance

**Likelihood**: Medium (30%)
**Impact**: High (no improvement)

**Indicators**:
- Test results show no improvement
- Agent ignores transformation patterns
- Query quality unchanged

**Mitigation**:
- Test with real Claude Code early (Day 2)
- Iterate on description clarity
- Get feedback from Anthropic if possible
- Fallback: Pivot to server-side preprocessing

**Contingency**: If agent doesn't follow guidance, pivot to Phase 2 (server-side) immediately

### Risk 2: Degradation of Simple Queries

**Likelihood**: Low (10%)
**Impact**: High (user complaints)

**Indicators**:
- Simple queries return fewer results
- Test shows <80% success rate
- User complaints about search quality

**Mitigation**:
- Before/after testing (Step 3)
- Include "when NOT to transform" guidance
- Monitor simple query success rate
- Immediate rollback if degradation

**Contingency**: Roll back to previous version

### Risk 3: Token Budget Exceeded

**Likelihood**: Very Low (5%)
**Impact**: Medium (context overflow)

**Indicators**:
- Token count >600
- Claude Code reports context issues

**Mitigation**:
- Automated token check in Step 2
- Keep description concise
- Use bullet points, not paragraphs

**Contingency**: Simplify description, reduce examples

### Risk 4: MCP Schema Invalid

**Likelihood**: Very Low (5%)
**Impact**: High (search broken)

**Indicators**:
- Schema validation fails
- Claude Code can't discover tool

**Mitigation**:
- Automated schema validation (Step 2)
- Manual testing before deploy (Step 5)

**Contingency**: Fix schema, redeploy

---

## Success Metrics

### Primary Metrics (Phase 1)

**Query Success Rate**:
- **Baseline**: 35% (queries with ≥3 results)
- **Target**: 75%
- **Measure**: Before/after comparison, 20 queries

**Natural Language Success**:
- **Baseline**: 10%
- **Target**: 70%
- **Measure**: 10 "how/what/where" questions

**Simple Query Success**:
- **Baseline**: 80%
- **Target**: 80% (no degradation)
- **Measure**: 5 simple 2-3 word queries

### Secondary Metrics

**Agent Retry Rate**:
- **Target**: <30% (most queries succeed first try)
- **Measure**: Log analysis, manual observation

**Top-3 Relevance**:
- **Target**: 80% have ≥2 relevant in top 3
- **Measure**: Manual evaluation of 10 queries

### Phase 1 Success Criteria

- [x] Natural language success: ≥70%
- [x] Simple query success: ≥80%
- [x] No increase in error rate
- [x] Token budget: <600
- [x] User feedback: Positive or neutral

**Go/No-Go for Phase 2**: If ≥3 of 5 criteria met

---

## Phase 2: Server-Side Preprocessing (OPTIONAL)

**Trigger**: Phase 1 shows ≥50% improvement but <70% target

**Duration**: 1 week
**Effort**: 16-24 hours
**Dependencies**: Rust development environment

**Deliverables**:
1. Query preprocessing module (Rust)
2. Metadata score boosting
3. Unit + integration tests
4. A/B testing results

**Success Criteria**:
- Additional +15-25% quality improvement (additive with Phase 1)
- <5ms latency added
- No regressions

**Decision Point**: End of Phase 1 monitoring (Day 7)

---

## Phase 0: Data-Driven Testing Framework (FOUNDATIONAL)

**Purpose**: Establish empirical testing infrastructure for discovering optimal tool descriptions through competitive A/B testing and genetic algorithm iterations.

**Duration**: 2-3 weeks (parallel with Phase 1)
**Effort**: 16-24 hours
**Dependencies**: Access to Claude API (for agent simulation)
**Priority**: HIGH (enables data-driven optimization)

### Deliverables

1. **Test Query Set** (100 queries)
   - 40 natural language queries ("How does X work?")
   - 30 simple queries ("error handling")
   - 20 complex queries ("cart checkout validation")
   - 10 edge cases (camelCase, file paths)
   - Gold standard expected results for each

2. **Variant Generation System**
   - Manual variant templates (4-5 initial variants)
   - Mutation engine (crossover, amplification, reduction, reframing, specialization)
   - Variant metadata tracking (generation, parent_ids, mutation_type)

3. **Automated Testing Harness**
   - Variant tester (runs 100 queries per variant)
   - Agent simulation (API-based, LLM-based, or rule-based)
   - Metrics collector (success_rate, avg_results, transformations)
   - Parallel execution for faster results

4. **Statistical Analysis Framework**
   - Pairwise t-tests (p<0.05 threshold)
   - Confidence interval calculation
   - Winner detection with statistical significance
   - Mutation recommendation engine

5. **Production A/B Testing Infrastructure**
   - Variant assignment (user_id → variant hash)
   - Metrics collection (query logs with variant_id)
   - Analysis dashboard (success rates, p-values)
   - Multi-armed bandit (optional, for continuous optimization)

### Success Criteria

- [ ] Test query set created with 100 representative queries
- [ ] 4-5 initial variants drafted and validated
- [ ] Testing harness runs full experiment in <30 minutes
- [ ] Statistical analyzer correctly identifies winners (validated with synthetic data)
- [ ] A/B testing infrastructure deployable to production
- [ ] Documented process for running experiments

### Testing Infrastructure Steps

**Step 1: Create Test Query Set** (4 hours)

Tasks:
1. Analyze current search logs for common query patterns
2. Draft 100 queries across 4 categories
3. Define expected results and gold standards
4. Validate query set with manual testing
5. Document query set in JSON format

Output: `test-queries.json` with 100 annotated queries

**Step 2: Build Variant System** (6 hours)

Tasks:
1. Design variant data structure
2. Create 4-5 manual variant templates
3. Implement variant storage (JSON files or database)
4. Build mutation engine with 5 mutation types
5. Create variant validator (token count, schema)

Output: Variant generation system with 5 initial variants

**Step 3: Implement Testing Harness** (8 hours)

Tasks:
1. Build agent simulation (start with rule-based)
2. Implement variant tester (run queries, collect metrics)
3. Add parallel execution for speed
4. Create metrics collector and storage
5. Build result formatter (JSON + human-readable)

Output: Automated testing harness executable

**Step 4: Statistical Analysis** (4 hours)

Tasks:
1. Implement t-test for pairwise comparisons
2. Build confidence interval calculator
3. Create winner detection logic
4. Add mutation recommendation engine
5. Generate experiment report template

Output: Statistical analyzer with clear winner detection

**Step 5: A/B Testing Infrastructure** (6 hours)

Tasks:
1. Implement variant assignment (consistent hashing)
2. Build metrics collection in MCP server
3. Create analysis queries (SQL)
4. Add A/B test dashboard (simple HTML/JSON)
5. Document deployment process

Output: Production-ready A/B testing system

### Experimental Process

**Experiment 1: Initial Variants** (Week 1)

```
Day 1-2: Create 4 variants + control
Day 3: Run tests (20 queries for speed)
Day 4: Analyze results, pick winner
Day 5: Deploy winner to Phase 1
```

**Experiment 2: First Mutations** (Week 2)

```
Day 1: Generate 4 mutations from winner
Day 2-3: Run full test (100 queries each)
Day 4: Statistical analysis
Day 5: Deploy if significant improvement
```

**Experiment 3+: Continuous Improvement** (Ongoing)

```
Weekly:
  - Generate 3-5 mutations
  - Test offline
  - Deploy best to A/B test (10% traffic)
  - Monitor for 1 week
  - Promote winner or iterate
```

### Agent Simulation Strategy

**Phase 0.1: Rule-Based** (Free, fast iteration)
- Extract transformation patterns from variant description
- Apply heuristic rules
- Good for rapid prototyping
- Expected accuracy: 60-70%

**Phase 0.2: API-Based** (Accurate, validates winners)
- Use Claude Sonnet API for actual agent simulation
- Cost: ~$1 per experiment (5 variants × 100 queries × $0.0002)
- Use for final validation before deployment
- Expected accuracy: 95%

**Phase 0.3: Haiku-Based** (Cheaper alternative)
- Use Claude Haiku for simulation
- Cost: ~$0.10 per experiment
- Good balance of cost and accuracy
- Expected accuracy: 85-90%

**Recommendation**: Start with rule-based for iterations 1-3, switch to Haiku for validation, use Sonnet for final deployment decision.

### Metrics and KPIs

**Primary Metrics**:
- Success rate: % queries with ≥3 results
- Natural language success: % NL queries successful
- Simple query success: % simple queries successful (no degradation)

**Secondary Metrics**:
- Avg results per query
- Top-3 relevance (manual spot check)
- Transformation consistency
- Token count

**Statistical Thresholds**:
- Significance: p<0.05
- Minimum sample: n≥100 per variant
- Confidence intervals: 95%
- Effect size: Cohen's d >0.3 (medium)

### Budget

**Development**: 20 hours * $100/hour = $2,000
**LLM costs** (experiments): $10-20 (optional, if using API simulation)
**Total**: ~$2,000-$2,020

**ROI**: Enables continuous 2-5% improvements per iteration, compounds over time

### Risk Mitigation

**Risk**: Testing framework too slow
**Mitigation**: Start with 20 queries, expand to 100 only for final validation

**Risk**: Agent simulation inaccurate
**Mitigation**: Validate rule-based results against real Claude API on subset

**Risk**: A/B testing infrastructure complex
**Mitigation**: Start with simple JSON logging, build dashboard later

**Risk**: No clear winners in experiments
**Mitigation**: Generate more diverse variants, adjust mutation strategies

### Integration with Phase 1

**Phase 0 runs parallel to Phase 1**:
- While Phase 1 implements "best guess" variant, Phase 0 builds testing framework
- Use Phase 0 framework to validate Phase 1 results
- If Phase 0 discovers better variant, quick iteration to Phase 1.1

**Decision Point**: End of Week 2
- If testing framework working: Continue iterating variants
- If clear winner found: Deploy via Phase 1 process
- If no improvement: Pivot to Phase 2 (server-side)

---

## Phase 3: LLM Fallback (OPTIONAL)

**Trigger**: Phase 1+2 show improvement but edge cases remain

**Duration**: 2 weeks
**Effort**: 24-32 hours
**Dependencies**: Anthropic API key, cost budget

**Deliverables**:
1. Haiku integration for query rewriting
2. Automatic fallback logic
3. Cost monitoring and alerts
4. Feature flag deployment

**Success Criteria**:
- +40-60% quality on edge cases
- Monthly cost <$150 at 100 users
- Fallback triggers <15% of queries

**Decision Point**: End of Phase 2 monitoring (Week 3)

---

## Relevant Agents

### Primary Agent

**mcp-developer** (custom, if exists):
- Implements tool description changes
- Validates MCP schema
- Deploys to production

**Alternative**: General TypeScript developer

### Support Agents

**qa-engineer** (custom, if exists):
- Creates test query sets
- Runs before/after comparisons
- Evaluates result relevance

**Alternative**: Manual testing by developer

### Workflow Agents

**verify-ticket** (existing):
- Verifies Phase 1 completion
- Checks acceptance criteria

**commit-ticket** (existing):
- Creates deployment commit
- Tags release version

---

## Communication Plan

### Stakeholders

**Internal**:
- Development team (daily updates)
- Product team (weekly summary)
- QA team (test results)

**External**:
- Claude Code users (if public)
- Documentation team (update docs)

### Status Updates

**Daily** (during active development):
- Slack: Progress on current step
- Blockers or risks identified

**Weekly** (during monitoring):
- Email: Metrics summary
- Decision on Phase 2 go/no-go

### Milestones

**M1: Enhanced description deployed** (Day 4)
**M2: 1 week monitoring complete** (Day 11)
**M3: Phase 2 go/no-go decision** (Day 11)

---

## Documentation

### Required Documentation

1. **README update**: Document enhanced tool description
2. **CHANGELOG entry**: Note query optimization feature
3. **Tool description**: Self-documenting (in MCP schema)
4. **Test report**: Before/after metrics
5. **Rollback procedure**: Quick reference

### Optional Documentation

- User guide: How to formulate good queries
- Developer guide: How to modify tool descriptions
- Metrics dashboard: Query success over time

---

## Rollback Plan

### Triggers

**Immediate rollback if**:
- Error rate increases >10%
- Query success rate decreases
- Critical user complaints

### Procedure

```bash
# 1. Revert to previous git tag
git checkout v1.X.0  # Previous stable
pnpm build

# 2. Restart MCP server
# (or wait for auto-restart by Claude Code)

# 3. Verify
curl http://localhost:3000/health

# 4. Monitor for 1 hour
tail -f logs/mcp.log
```

**Time to rollback**: <5 minutes
**Rollback owner**: Any developer with deploy access

---

## Post-Phase 1 Options

### Option 1: Ship It (Success)

**Condition**: Phase 1 meets all success criteria

**Next steps**:
- Mark Phase 1 complete
- Document learnings
- Consider Phase 2 (optional enhancement)
- Monitor long-term metrics

### Option 2: Iterate (Partial Success)

**Condition**: Phase 1 shows improvement but <70% target

**Next steps**:
- Refine tool description
- Add more examples
- Test additional query patterns
- Retry in 1 week

### Option 3: Pivot (Failure)

**Condition**: Phase 1 shows no improvement

**Next steps**:
- Roll back changes
- Analyze why agent didn't follow guidance
- Pivot to Phase 2 (server-side preprocessing)
- Consider different approach

### Option 4: Stop (Waste of Time)

**Condition**: Problem not actually important

**Next steps**:
- Roll back if deployed
- Document why we stopped
- Archive project
- Focus on higher priorities

---

## Budget

### Phase 1

**Development**: 10 hours * $100/hour = $1,000
**QA**: 4 hours * $80/hour = $320
**Code review**: 1 hour * $120/hour = $120
**Total**: $1,440

**Infrastructure**: $0 (no new costs)

**Estimated ROI**:
- User time saved: ~10 hours/month (100 users)
- Value: ~$2,000/month in improved productivity
- Payback: <1 month

### Phase 2 (Optional)

**Development**: 20 hours * $100/hour = $2,000
**QA**: 8 hours * $80/hour = $640
**Total**: $2,640

**Infrastructure**: $0

### Phase 3 (Optional)

**Development**: 30 hours * $100/hour = $3,000
**QA**: 8 hours * $80/hour = $640
**LLM costs**: ~$150/month (100 users)
**Total**: $3,640 + $150/month ongoing

---

## Key Decisions

### Decision 1: Proceed with Phase 1?

**Status**: ✅ YES
**Rationale**: Low risk, high potential impact, minimal cost
**Approver**: Tech lead

### Decision 2: Proceed with Phase 2?

**Status**: ⏸️ PENDING (after Phase 1 monitoring)
**Criteria**: Phase 1 shows ≥50% improvement but <70% target
**Approver**: Product owner

### Decision 3: Proceed with Phase 3?

**Status**: ⏸️ PENDING (after Phase 2 results)
**Criteria**: Edge cases remain AND budget approved
**Approver**: Engineering manager (due to ongoing costs)

---

## Summary

**Phase 1 is a low-risk, high-impact opportunity** to improve query quality for AI agents by teaching Claude Code how to formulate better queries through enhanced tool descriptions.

**Timeline**: 1 week (3 days active, 4 days monitoring)
**Effort**: 8-12 hours
**Cost**: ~$1,400
**Risk**: Minimal (easy rollback, no infrastructure changes)
**Expected Impact**: +40-60 percentage points in query success rate

**Recommendation**: Proceed with Phase 1 immediately. Evaluate Phase 2/3 based on results.

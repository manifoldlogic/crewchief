# Quality Strategy: Maproom Search Tool Optimization

## Testing Philosophy

**MVP Mindset**: Tests should provide confidence that changes work as expected, not achieve ceremonial coverage metrics. Focus on validation that prevents rework and catches regressions.

**Risk-Based Testing**: Prioritize tests for:
1. **High-impact changes** (production tool description update)
2. **Experimental claims** (verify 19.6% performance replicates)
3. **Integration points** (MCP server, agent interaction)

**Pragmatic Scope**: This is a content change (tool description text), not a code change. Traditional unit tests don't apply. Focus on **functional validation** and **performance testing**.

## Test Pyramid

### Level 1: Benchmark Validation (Critical)

**Purpose**: Verify experimental results replicate in clean environment

**Approach**: Run the same benchmark that produced 19.6% score

**Test**: Comparison Test
```bash
npx tsx src/search-optimization/run-comparison.ts \
  variant-control \
  variant-a-detailed \
  --tasks=impl-worktree-001 \
  --iterations=5 \
  --parallel=false
```

**Success Criteria**:
- variant-a-detailed: ≥19.0% (allow 0.6% margin for test variance)
- variant-control: 17-18% (baseline reference)
- Confidence interval: p < 0.05 (statistically significant improvement)

**Why Critical**: If this fails, the entire premise (variant-a-detailed is better) is invalidated.

**Effort**: 30-60 minutes runtime, automated

### Level 2: Integration Validation (Important)

**Purpose**: Ensure tool description integrates correctly with MCP server

**Test**: MCP Server Startup
```bash
# Start MCP server with new description
cd packages/maproom-mcp
pnpm build
node dist/index.js
```

**Success Criteria**:
- Server starts without errors
- Tool description appears in tool list
- Description text matches variant-a-detailed
- No JSON parsing errors
- No type validation errors

**Manual Verification**:
```bash
# Use MCP inspector tool to view available tools
echo '{"jsonrpc":"2.0","method":"tools/list","id":1}' | node dist/index.js
```

**Why Important**: Catches formatting errors, type mismatches, or build issues.

**Effort**: 5 minutes, mostly automated

### Level 3: Agent Interaction Testing (Validation)

**Purpose**: Confirm agents can successfully use the tool with new description

**Test**: Sample Search Task
```bash
# Spawn agent with task
npx tsx scripts/test-mcp-config.ts /workspace
```

**Success Criteria**:
- Agent successfully calls search tool
- Query formulated matches transformation patterns
- Results returned and processed correctly
- No tool call errors or timeouts

**Sample Tasks** (spot check 2-3):
1. "Find where git worktrees are created"
2. "Locate error handling for database connections"
3. "Find React component state management"

**Why Validation**: Ensures real-world usage works, not just benchmark scenarios.

**Effort**: 15-30 minutes manual testing

### Level 4: Regression Prevention (Nice-to-Have)

**Purpose**: Ensure we didn't break existing functionality

**Test**: Existing Agent Workflows
- Check recent agent conversations using maproom search
- Verify query patterns still work
- Confirm no error rate increase

**Success Criteria**:
- No new error types introduced
- Search call frequency unchanged or improved
- Query quality stable or improved

**Why Nice-to-Have**: Production usage will reveal issues faster than pre-deployment testing.

**Effort**: Passive monitoring, 0 upfront cost

## Test Scenarios

### Scenario 1: Happy Path - Transformation Workflow

**Given**: Agent receives task "Find where authentication is implemented"

**Expected Behavior**:
1. Agent reads tool description
2. Applies transformation patterns: "Find where X is implemented" → "authentication"
3. Calls search tool with query="authentication"
4. Receives results
5. Identifies implementation file from results
6. Returns correct file path

**Validation**: Check agent conversation for transformation evidence

**Success Signal**: Query is concise (1-3 words), not full question

### Scenario 2: Recovery - Multi-Query Strategy

**Given**: Agent's first query returns <3 results

**Expected Behavior**:
1. Agent recognizes insufficient results
2. Applies multi-query strategy from description
3. Tries variation: "auth" or "authenticate"
4. Gets better results
5. Succeeds on retry

**Validation**: Check for multiple search calls with related queries

**Success Signal**: Agent doesn't give up after first query, uses variations

### Scenario 3: Boundary - Anti-Patterns

**Given**: Agent task involves finding TODO comments

**Expected Behavior**:
1. Agent reads "NOT FOR: Exact string matching" section
2. Recognizes this is anti-pattern
3. Uses Grep instead of semantic search
4. Successfully finds TODOs

**Validation**: Check agent tool choice (should use Grep, not search)

**Success Signal**: Agent self-corrects to appropriate tool

### Scenario 4: Edge Case - Very Long Query

**Given**: Agent formulates query >4 words

**Expected Behavior**:
1. Agent reads "Avoid: Very long queries" guidance
2. Shortens query to 2-3 core terms
3. Calls search with concise query
4. Gets relevant results

**Validation**: Check query length in tool call

**Success Signal**: Query is ≤4 words despite complex user question

## Performance Testing

### Benchmark Configuration

**Task Set**: Use existing benchmark suite
- impl-worktree-001 (primary)
- Additional tasks from suite (future)

**Iterations**: 5 runs per variant (capture variance)

**Environment**:
- Same git worktree
- Same database state
- Same Claude model (Sonnet 4.5)
- Same MCP configuration

**Measurement**:
- Success rate (% of runs that find correct file)
- Query quality (manual inspection of formulated queries)
- Turn count (efficiency metric)
- Search call count (retry frequency)

### Performance Criteria

| Metric | Current (Control) | Target (Detailed) | Threshold |
|--------|------------------|-------------------|-----------|
| Success Rate | 17.7% | 19.6% | ≥19.0% |
| Avg Query Length | ~5 words | ~2-3 words | ≤4 words |
| Avg Turn Count | ~8 turns | ~6-8 turns | ≤10 turns |
| Search Calls | ~2-3 | ~1-2 | ≤3 |

**Statistical Significance**: Use t-test to confirm improvement is real, not variance (p < 0.05)

## Documentation Testing

### Accuracy Verification

**Test**: Cross-reference documentation against source data

**Checks**:
- [ ] Reported scores match generation reports
- [ ] Variant descriptions match JSON files
- [ ] Token counts accurate
- [ ] Examples are real (not invented)
- [ ] Patterns identified actually exist in winners

**Method**: Spot check 10-15 claims against source files

**Effort**: 30 minutes manual review

### Completeness Verification

**Test**: Ensure all key learnings are documented

**Checks**:
- [ ] Winning patterns explained
- [ ] Anti-patterns documented
- [ ] Quantitative results included
- [ ] Qualitative insights captured
- [ ] Enhancement recommendation clear

**Method**: Compare documentation TOC against analysis findings

**Effort**: 15 minutes review

### Usability Verification

**Test**: External reviewer can understand documentation

**Checks**:
- [ ] Stands alone without conversation context
- [ ] Technical terms defined
- [ ] Examples are clear
- [ ] Recommendations actionable

**Method**: Fresh-eyes review by team member

**Effort**: 30 minutes review + feedback

## Risk Mitigation

### Risk 1: Experimental Results Don't Replicate

**Probability**: Medium (20%)
**Impact**: High (invalidates entire approach)

**Mitigation**:
- Run validation test BEFORE merging
- Use same environment as genetic experiment
- Multiple iterations to account for variance
- Statistical significance testing

**Contingency**:
- If <19.0%: Don't deploy, investigate variance
- If 19.0-19.3%: Deploy with caution, monitor closely
- If ≥19.3%: Deploy with confidence

### Risk 2: Production Environment Differs from Test

**Probability**: Low (10%)
**Impact**: Medium (works in test, fails in production)

**Mitigation**:
- Test in actual MCP server, not mocked
- Use real Claude API, not simulated
- Run test in dev container matching production

**Contingency**:
- Rollback immediately if production issues
- Investigate environment differences
- Adjust test environment to match production

### Risk 3: Agent Behavior Variance

**Probability**: Medium (25%)
**Impact**: Low (results vary ±1-2% per run)

**Mitigation**:
- Run 5 iterations, average results
- Set threshold at 19.0% (0.6% margin below 19.6% target)
- Accept natural variance in agent performance

**Contingency**:
- If variance >2%: Investigate non-determinism
- If consistent low performance: Re-evaluate variant

### Risk 4: Documentation Goes Stale

**Probability**: Medium (30% over 6 months)
**Impact**: Low (learnings forgotten, but no breakage)

**Mitigation**:
- Place docs in `/docs/` (high visibility)
- Link from README
- Include in onboarding materials
- Schedule quarterly review

**Contingency**:
- If stale: Refresh documentation
- If obsolete: Archive and document new learnings

## Quality Gates

### Gate 1: Pre-Merge (Blocking)

**Must Pass**:
- [ ] Benchmark validation shows ≥19.0% performance
- [ ] MCP server starts without errors
- [ ] Tool description loads correctly
- [ ] Documentation complete and accurate

**Review Required**:
- [ ] Code review approved (description change)
- [ ] Documentation review approved
- [ ] Test results reviewed

**Timeline**: Complete before merging PR

### Gate 2: Post-Deployment (Monitoring)

**Must Monitor**:
- [ ] Agent success rates (first 24 hours)
- [ ] Error rates (first week)
- [ ] Query quality (spot checks)

**Alert Conditions**:
- Error rate increase >5%
- Success rate drop >2%
- Anomalous query patterns

**Timeline**: Passive monitoring for 1 week

### Gate 3: Enhancement Readiness (Future)

**Before Testing variant-e-task-mapping**:
- [ ] variant-a-detailed deployed and stable
- [ ] Baseline performance confirmed
- [ ] Enhancement variant properly formatted
- [ ] Test environment prepared

**Timeline**: When ready for next genetic run

## Test Automation

### Automated Tests

**Comparison Test Script**: `run-comparison.ts`
- Loads variants from JSON
- Spawns agents in parallel
- Scores results
- Generates report
- **Maintainer**: Existing, no changes needed

**MCP Server Validation**: `test-mcp-config.ts`
- Starts MCP server
- Checks tool list
- Validates description
- **Maintainer**: Existing, no changes needed

### Manual Tests

**Agent Interaction**: Spot check 2-3 sample tasks
- **Frequency**: Pre-deployment
- **Owner**: Developer deploying change
- **Duration**: 15-30 minutes

**Documentation Review**: External reviewer checks clarity
- **Frequency**: Pre-merge
- **Owner**: Team member (not author)
- **Duration**: 30 minutes

## Success Metrics

### Quantitative

- **Performance**: variant-a-detailed achieves 19.0-20.0% on benchmark (validated)
- **Stability**: MCP server starts and runs without errors
- **Compatibility**: Existing agent workflows continue working

### Qualitative

- **Documentation Quality**: External reviewer confirms clarity and completeness
- **Agent Behavior**: Spot checks show improved query formulation
- **Knowledge Transfer**: Team understands patterns and can apply to other tools

### Timeline

- **Phase 1 (Documentation)**: 2-3 hours total
- **Phase 2 (Validation)**: 1-2 hours testing + review
- **Phase 3 (Deployment)**: 30 minutes (build + restart)
- **Phase 4 (Monitoring)**: Passive, 1 week

### Acceptance Criteria

All must be true to consider project successful:
- [ ] Documentation published and reviewed
- [ ] Validation test passes (≥19.0%)
- [ ] Production deployment successful
- [ ] No post-deployment regressions detected
- [ ] Enhancement variant created for future

## Test Environment

### Required Infrastructure

- **Database**: PostgreSQL with pgvector (existing)
- **MCP Server**: Maproom MCP (existing)
- **Test Framework**: CLI search optimization suite (existing)
- **Claude API**: Anthropic API access (existing)

### Setup Requirements

```bash
# 1. Ensure database is running
docker-compose -f packages/maproom-mcp/config/docker-compose.yml up -d

# 2. Build packages
pnpm install
pnpm build

# 3. Verify test suite works
cd packages/cli
npx tsx src/search-optimization/run-comparison.ts variant-control variant-a-detailed
```

**No new infrastructure needed** - leverage existing test setup.

## Maintenance Plan

### During Development

- Run validation test after any description changes
- Verify MCP server starts after builds
- Spot check agent interactions before committing

### Post-Deployment

- **Week 1**: Active monitoring (daily spot checks)
- **Week 2-4**: Passive monitoring (error logs only)
- **Month 2+**: Quarterly review of documentation

### Long-term

- Update documentation when new patterns discovered
- Re-run validation when major MCP or Claude updates occur
- Archive outdated variants (keep for historical reference)

## Conclusion

This quality strategy prioritizes **validation over verification** - we're adopting proven experimental results, not building new functionality. The critical test is confirming the 19.6% performance replicates, everything else is hygiene.

Focus: Run the benchmark, check the results, deploy with confidence.

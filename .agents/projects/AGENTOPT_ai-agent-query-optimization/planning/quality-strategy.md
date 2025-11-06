# Quality Strategy: AI Agent Query Optimization

## Testing Philosophy

**Pragmatic MVP Approach**: This project modifies a single string (tool description) in one file. Over-testing would waste time. Focus on validation that prevents rework and ensures the enhancement actually helps.

**Core Principle**: Manual evaluation is appropriate here since we're optimizing for AI agent behavior (subjective quality), not algorithmic correctness.

## Quality Tiers

### Tier 1: Critical (Must Have) ✅

**What could cause rework or production issues**

#### 1. Before/After Query Comparison

**Test**: Run 20 diverse queries before and after enhancement

**Sample queries**:
```
Natural language (currently failing):
1. "How does authentication work in this codebase?"
2. "What handles WebSocket disconnections?"
3. "Find error handling logic"
4. "Where is cart validation?"
5. "Show me the checkout process"

Simple 2-3 word (currently working):
6. "error handling"
7. "cart checkout"
8. "auth middleware"
9. "WebSocket disconnect"
10. "message bus"

Complex multi-word:
11. "shopping cart total calculation"
12. "user authentication middleware handler"
13. "database connection pool manager"
14. "API endpoint validation logic"
15. "async task queue processor"

Edge cases:
16. "processCheckout" (camelCase)
17. "validate_cart_items" (snake_case)
18. "src/cart/checkout.ts" (file path)
19. "TODO: fix auth bug" (comment-like)
20. "e" (single letter)
```

**Success criteria**:
- Natural language (1-5): 70% find ≥3 results (vs 10% baseline)
- Simple queries (6-10): Maintain 80% success (no degradation)
- Complex queries (11-15): 50% improvement
- Edge cases (16-20): Handle gracefully (don't crash)

**How to run**:
```bash
# Save queries to file
cat > test_queries.txt << EOF
How does authentication work?
error handling
shopping cart total calculation
src/cart/checkout.ts
EOF

# Run before enhancement
node packages/maproom-mcp/test/manual-query-test.js test_queries.txt > before.json

# Apply enhancement, rebuild
pnpm build

# Run after enhancement
node packages/maproom-mcp/test/manual-query-test.js test_queries.txt > after.json

# Compare
node packages/maproom-mcp/test/compare-results.js before.json after.json
```

**Pass/Fail**: Manual review of results, must show clear improvement

#### 2. Token Budget Validation

**Test**: Ensure tool description fits in Claude's context

**Measurement**:
```typescript
import { encode } from 'gpt-3-encoder'  // Approximation

const description = `...` // Enhanced description
const tokens = encode(description).length

console.log(`Tool description tokens: ${tokens}`)
console.log(`Max acceptable: 600`)
console.log(`Pass: ${tokens < 600}`)
```

**Success criteria**: <600 tokens (< 1% of Claude's 100K context)

**Pass/Fail**: Automated check

#### 3. MCP Schema Validation

**Test**: Ensure tool still validates against MCP protocol

**Run**:
```bash
# Start MCP server
node packages/maproom-mcp/dist/index.js &
MCP_PID=$!

# Call tools/list
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | \
  node packages/maproom-mcp/dist/index.js

# Verify response is valid JSON-RPC
kill $MCP_PID
```

**Success criteria**: Valid JSON-RPC response, no errors

**Pass/Fail**: Automated check

### Tier 2: Important (Should Have) 📊

**What gives confidence but isn't critical**

#### 4. Result Relevance Spot Check

**Test**: Manual evaluation of top-3 results for 10 queries

**Queries**:
```
1. "authentication"
2. "cart checkout"
3. "error handler"
4. "WebSocket disconnect"
5. "database query"
6. "user validation"
7. "message bus"
8. "async task"
9. "API endpoint"
10. "file upload"
```

**For each query, check**:
- Are top 3 results actually relevant?
- Do they match user's likely intent?
- Are there obvious false positives?

**Success criteria**: ≥8 of 10 queries have at least 2 relevant results in top 3

**Pass/Fail**: Manual judgment

#### 5. Retry Behavior Observation

**Test**: Observe if agent retries with variations

**Method**:
1. Give Claude Code a failing query: "How does really complex authentication work?"
2. Watch if it tries multiple formulations
3. Check MCP logs for multiple search calls

**Expected behavior**:
```
User: "How does really complex authentication work?"
Claude: [searches "authentication complex"]
Claude: Only 1 result, trying variation...
Claude: [searches "authentication"]
Claude: Found 5 results ✓
```

**Success criteria**: Agent retries on poor results (shows it read the guidance)

**Pass/Fail**: Manual observation (sample size: 3-5 queries)

### Tier 3: Nice to Have (Optional) 🎯

**What we'd measure in production but can skip for MVP**

#### 6. Latency Impact

**Test**: Measure if enhanced description slows agent thinking

**Method**:
```bash
# Before: Time 10 queries
time node test/query-latency.js queries.txt

# After: Time same 10 queries
time node test/query-latency.js queries.txt
```

**Acceptable**: <500ms average increase (agent thinking time varies anyway)

**Skip if**: Time-constrained (latency is not critical for MVP)

#### 7. Long-Term Metrics

**Metrics** (would collect in production):
- Query success rate over time
- Agent retry rate
- User satisfaction (proxy: repeat searches)

**Skip for MVP**: These require production usage to measure

## Testing Approach by Phase

### Phase 1: Enhanced Tool Description

**Must have**:
- ✅ Before/after query comparison (20 queries)
- ✅ Token budget check
- ✅ MCP schema validation

**Should have**:
- 📊 Result relevance spot check (10 queries)
- 📊 Retry behavior observation (3-5 queries)

**Skip**:
- ⏭️ Automated integration tests (overkill for string change)
- ⏭️ Performance benchmarks (no code logic added)
- ⏭️ Load testing (no infrastructure change)

**Total effort**: 4 hours (mostly manual evaluation)

### Phase 2: Server Preprocessing (Future)

**Must have**:
- ✅ Unit tests for preprocessing function
- ✅ Unit tests for stop word filtering
- ✅ Integration tests for full search flow

**Example unit test**:
```rust
#[test]
fn test_preprocess_removes_question_words() {
    assert_eq!(
        preprocess_query("How does authentication work?"),
        Ok("authentication work".to_string())
    );
}

#[test]
fn test_preprocess_handles_empty_result() {
    assert!(preprocess_query("how what where").is_err());
}

#[test]
fn test_preprocess_preserves_good_queries() {
    assert_eq!(
        preprocess_query("error handling"),
        Ok("error handling".to_string())
    );
}
```

**Should have**:
- 📊 Performance benchmarks (<5ms)
- 📊 Before/after quality comparison

**Skip**:
- ⏭️ Exhaustive edge case testing (pragmatic coverage sufficient)

### Phase 3: LLM Fallback (Future)

**Must have**:
- ✅ Mock LLM responses (for testing without API calls)
- ✅ Error handling tests (API failures, timeouts)
- ✅ Cost monitoring

**Example test**:
```rust
#[tokio::test]
async fn test_llm_fallback_on_zero_results() {
    let mock_llm = MockLlm::new()
        .with_response(vec![
            "error handler".to_string(),
            "exception handler".to_string()
        ]);

    let results = search_with_fallback("how to handle errors", mock_llm).await?;
    assert!(results.hits.len() > 0);
    assert_eq!(results.fallback_used, true);
}
```

**Should have**:
- 📊 Fallback success rate measurement
- 📊 Cost tracking per query type

## Test Automation

### What to Automate

1. **MCP schema validation** - Fast, deterministic
2. **Token counting** - Fast, deterministic
3. **Server startup** - Prevents regressions

### What NOT to Automate

1. **Query quality evaluation** - Subjective, requires human judgment
2. **Result relevance** - Context-dependent
3. **Agent behavior observation** - Non-deterministic

### Automation Script (packages/maproom-mcp/test/validate.sh)

```bash
#!/bin/bash
set -e

echo "🧪 Running MCP validation tests..."

# 1. Token budget check
echo "Checking token budget..."
node test/check-tokens.js || exit 1

# 2. MCP schema validation
echo "Validating MCP schema..."
node test/validate-schema.js || exit 1

# 3. Server startup test
echo "Testing server startup..."
timeout 5s node dist/index.js > /dev/null 2>&1 &
PID=$!
sleep 2
if ps -p $PID > /dev/null; then
    kill $PID
    echo "✅ Server starts successfully"
else
    echo "❌ Server failed to start"
    exit 1
fi

echo "✅ All validation tests passed"
```

**Run on**: Pre-commit hook, CI pipeline

## Manual Testing Checklist

### Pre-Deployment Checklist

```yaml
Before merging:
  ☐ Tool description enhanced with transformation patterns
  ☐ Examples added for good/bad queries
  ☐ Multi-query retry strategy documented
  ☐ Token count < 600 tokens
  ☐ MCP schema validates
  ☐ 20 test queries run before/after
  ☐ Natural language queries: 70% success
  ☐ Simple queries: No degradation
  ☐ Top-3 relevance spot checked
  ☐ Agent retry behavior observed
  ☐ Git tagged: v1.X.0-agent-opt
  ☐ Rollback plan documented

Before announcing:
  ☐ Deployed to production
  ☐ Monitored for 24 hours
  ☐ No increase in error rates
  ☐ User feedback positive
  ☐ Documentation updated
```

## Quality Gates

### Gate 1: Code Review

**Reviewers**: 1 senior dev (focus on tool description clarity)

**Checklist**:
- [ ] Tool description is clear and concise
- [ ] Examples are realistic and helpful
- [ ] Transformation patterns are easy to follow
- [ ] No typos or grammar errors
- [ ] Formatting is readable

**Reject if**: Description is confusing or overly complex

### Gate 2: Manual Testing

**Tester**: Developer or QA

**Checklist**:
- [ ] 20 test queries show improvement
- [ ] No regressions on simple queries
- [ ] Token budget acceptable
- [ ] MCP schema valid

**Reject if**: Quality metrics not met

### Gate 3: Deployment Approval

**Approver**: Tech lead or product owner

**Checklist**:
- [ ] Rollback plan in place
- [ ] Monitoring configured
- [ ] Documentation ready
- [ ] User communication prepared

**Reject if**: Risk mitigation insufficient

## Monitoring and Validation (Post-Deploy)

### Week 1: Close Monitoring

**Daily checks**:
- Error rate (should be stable)
- Query success rate (should improve)
- User feedback (via support tickets)

**Metrics to collect**:
```sql
SELECT
  DATE(created_at) as date,
  COUNT(*) as total_queries,
  AVG(result_count) as avg_results,
  PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY result_count) as median_results
FROM query_logs
WHERE created_at > NOW() - INTERVAL '7 days'
GROUP BY DATE(created_at)
ORDER BY date;
```

### Month 1: Trend Analysis

**Compare**:
- Query success rate: Before vs After
- Natural language query handling: Improvement
- User satisfaction: Feedback sentiment

**Goal**: Validate that enhancement delivers expected value

## Regression Prevention

### What Could Break

1. **Tool description too long** → Token budget exceeded → Context overflow
   - **Prevention**: Automated token check in CI

2. **Invalid MCP schema** → Claude Code can't discover tool → Search broken
   - **Prevention**: Schema validation test

3. **Degraded simple queries** → Users lose existing functionality → Complaints
   - **Prevention**: Before/after comparison, maintain baseline

4. **Agent ignores guidance** → No improvement → Wasted effort
   - **Prevention**: Manual observation of agent behavior

### Rollback Triggers

**Immediate rollback if**:
- Error rate increases >10%
- Query success rate decreases
- Critical user complaints (search broken)

**Rollback process**:
```bash
# Revert to previous version
git checkout v1.X.0  # Previous stable tag
pnpm build
# Restart MCP server (or wait for auto-restart)
```

**Time to rollback**: <5 minutes

## Success Metrics (Definition of Done)

### Phase 1 Complete When:

- [x] Tool description enhanced and deployed
- [x] Natural language query success: 70% (vs 10% baseline)
- [x] Simple query success: Maintained at 80%
- [x] No increase in error rates
- [x] User feedback positive (or neutral)
- [x] Git tagged and documented

### Quality Bar:

**Minimum acceptable**:
- +40 percentage points in natural language query success
- No degradation in simple queries
- <600 tokens in tool description
- Valid MCP schema

**Stretch goal**:
- +60 percentage points in natural language query success
- Agent consistently retries on poor results
- User testimonials of improved search

## Tools and Infrastructure

### Testing Tools

**Manual testing**:
- `test/manual-query-test.js` - Run queries, collect results
- `test/compare-results.js` - Compare before/after
- `test/check-tokens.js` - Count tokens in description

**Automation**:
- `test/validate-schema.js` - Validate MCP protocol
- `test/validate.sh` - Run all checks
- `.github/workflows/test.yml` - CI pipeline

### Monitoring

**Logs**:
```typescript
// Enhanced logging for query analysis
log.info({
  query_original: req.query,
  query_tokens: tokenCount(req.query),
  results_count: results.length,
  top_score: results[0]?.score,
  agent_retry: detectRetry(req.headers)
}, 'Search completed')
```

**Dashboards** (future):
- Query success rate over time
- Natural language vs simple query distribution
- Agent retry rate

## Risk Mitigation

### Risk 1: Agent Doesn't Follow Guidance

**Likelihood**: Medium
**Impact**: High (no improvement, waste of effort)

**Mitigation**:
- Test with real Claude Code instance
- Observe agent behavior on 5+ queries
- Iterate on tool description clarity
- Get feedback from Claude team (if possible)

**Fallback**: If agent ignores guidance, this approach won't work. Pivot to server-side preprocessing instead.

### Risk 2: Tool Description Too Complex

**Likelihood**: Low
**Impact**: Medium (agent confused, inconsistent behavior)

**Mitigation**:
- Keep examples simple and clear
- Use bullet points, not paragraphs
- Test readability with other developers

**Fallback**: Simplify description, reduce examples

### Risk 3: Degradation of Simple Queries

**Likelihood**: Low
**Impact**: High (users complain, must rollback)

**Mitigation**:
- Before/after testing
- Include "when NOT to transform" guidance
- Monitor simple query success rate

**Fallback**: Immediate rollback if degradation detected

## Quality Summary

**Testing effort**: ~4 hours
- 2 hours: Manual query testing (20 queries)
- 1 hour: Token budget + schema validation
- 1 hour: Agent behavior observation

**Confidence level**: High
- Simple change (string modification)
- Low risk (easy rollback)
- Clear success criteria
- Manual validation appropriate

**Not testing**:
- ❌ Automated integration tests (overkill)
- ❌ Load testing (no infrastructure change)
- ❌ Security testing (no security surface)
- ❌ Cross-browser testing (server-side only)

**Pragmatic approach**: Test what matters, skip ceremony

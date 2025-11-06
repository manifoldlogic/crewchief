# Tool Description Optimization Framework: Data-Driven Iterative Improvement

## Executive Summary

**Problem**: We don't know which tool description variant will work best for AI agents. Guessing wastes time.

**Solution**: Treat tool description optimization as a data-driven, iterative experiment. Use A/B testing + genetic algorithm approach to discover optimal patterns.

**Approach**:
1. Generate multiple competing variants
2. Test each against same query set
3. Measure objective success metrics
4. Keep winners, generate new variants from them
5. Repeat until convergence

**Expected outcome**: Discover optimal tool description through empirical evidence, not assumptions.

---

## Core Principles

### 1. Data Over Vibes

**Bad**:
```
"I think this description will work better"
→ Deploy
→ Hope for the best
```

**Good**:
```
"Variant A: 72% success rate (n=100)"
"Variant B: 68% success rate (n=100)"
→ Deploy A
→ Generate new variants from A's patterns
```

### 2. Competition Drives Discovery

**Approach**: Multiple variants compete on same test set
- Variant A: Detailed transformation patterns
- Variant B: Simple bullet points
- Variant C: Conversational examples
- Variant D: Code-like syntax

**Winner**: Empirically determined by success rate

### 3. Continuous Improvement

**Not**: One-time optimization
**Instead**: Ongoing evolution
- Week 1: Test 5 variants → Winner: A
- Week 2: Mutate A → Test 5 new variants → Winner: A2
- Week 3: Mutate A2 → Test 5 new variants → Winner: A2.1
- Convergence after ~4-6 weeks

### 4. Statistical Rigor

**Require**:
- Minimum sample size (n≥50 per variant)
- Statistical significance (p<0.05)
- Confidence intervals on metrics
- A/A testing to validate framework

---

## Framework Architecture

### Phase 1: Manual Testing (Week 1)

**Goal**: Validate approach, establish baseline

**Process**:
```
1. Create 3 variants manually
2. Test each on same 20 queries
3. Measure success rate
4. Pick winner
5. Deploy to production
```

**Effort**: 8 hours
**Confidence**: Low (small sample)
**Value**: Prove concept

### Phase 2: Automated Testing (Week 2-3)

**Goal**: Scale to larger test set, automate measurement

**Process**:
```
1. Create 5 variants (manual + mutations)
2. Automated testing: 100 queries per variant
3. Statistical analysis of results
4. Deploy winner
5. Generate next generation
```

**Effort**: 16 hours (setup) + 4 hours/iteration
**Confidence**: Medium (n=100)
**Value**: Reliable optimization

### Phase 3: Live A/B Testing (Week 4+)

**Goal**: Real-world validation, continuous optimization

**Process**:
```
1. Deploy 2 variants (50/50 split)
2. Collect metrics from real usage
3. Analyze after 1000 queries
4. Deploy winner
5. Generate challenger variant
6. Repeat continuously
```

**Effort**: 24 hours (infrastructure) + 2 hours/week
**Confidence**: High (real users)
**Value**: Production-grade optimization

---

## Variant Generation Strategy

### Initial Generation (Manual)

**Create 3-5 variants with different approaches**:

**Variant A: Detailed Patterns**
```typescript
description: `Semantic search for AI agents.

AI AGENT QUERY FORMULATION GUIDE:

TRANSFORMATION PATTERNS:
1. Extract core technical terms (2-3 words)
2. Remove question words: how, what, where, when, why
3. Prefer code-like terminology over natural language

EXAMPLES:
Input: "How does authentication work?"
Process: Extract "authentication" → Remove "How does" → Result: "authentication"

Input: "What handles errors?"
Process: Extract "error", "handles" → Transform to "error handler"

[10+ examples...]

MULTI-QUERY STRATEGY:
If <3 results, try variations:
- Original: "error handling"
- Variation 1: "exception handler"
- Variation 2: "try catch"
`
```

**Variant B: Simple Bullets**
```typescript
description: `Semantic search for AI agents.

QUERY TIPS FOR AI AGENTS:
• Use 2-3 technical terms, not full sentences
• Remove question words (how, what, where)
• Try variations if first query fails

Good: "error handling", "cart checkout", "auth middleware"
Bad: "How do I handle errors?", "function_that_validates_cart"
`
```

**Variant C: Conversational Examples**
```typescript
description: `Semantic search for AI agents.

When the user asks a question, transform it to simple search terms:

User asks: "How does authentication work?"
You search: "authentication"

User asks: "Find error handling"
You search: "error handler"

User asks: "Where is cart validation?"
You search: "cart validation"

If first search returns <3 results, try a variation.
`
```

**Variant D: Code-Like Instructions**
```typescript
description: `Semantic search for AI agents.

function transformQuery(userQuestion: string): string {
  const terms = extractTechnicalTerms(userQuestion)  // 2-3 words
  const cleaned = removeQuestionWords(terms)  // how, what, where...
  return preferCodeTerminology(cleaned)  // "processCheckout" > "checkout process"
}

function search(userQuestion: string): Results {
  let query = transformQuery(userQuestion)
  let results = semanticSearch(query)

  if (results.length < 3) {
    query = generateVariation(query)  // "error handling" → "exception handler"
    results = semanticSearch(query)
  }

  return results
}
`
```

**Variant E: Control (Baseline)**
```typescript
description: `Semantic code search.
Use simple terms. Works best with 2-3 words.
Examples: "error handling", "cart checkout"
`
```

### Mutation Strategy (Genetic Algorithm)

**After initial testing, generate new variants from winners**:

**Mutation Types**:

1. **Crossover**: Combine elements from top 2 variants
   ```
   Winner A: Detailed patterns + 10 examples
   Winner B: Simple bullets + conversational tone

   Offspring: Detailed patterns + conversational tone (hybrid)
   ```

2. **Amplification**: Double down on what works
   ```
   Winner: 5 examples worked well
   Mutation: 15 examples (more is better?)
   ```

3. **Reduction**: Simplify successful variant
   ```
   Winner: 500 tokens, 72% success
   Mutation: 300 tokens, same patterns (leaner)
   ```

4. **Reframing**: Same content, different presentation
   ```
   Winner: Bullet points
   Mutation: Numbered steps
   ```

5. **Specialization**: Optimize for specific query types
   ```
   Winner: General patterns
   Mutation A: Optimized for natural language questions
   Mutation B: Optimized for code-like searches
   ```

### Example Mutation Cycle

**Generation 0** (Manual):
- A: Detailed patterns (500 tokens)
- B: Simple bullets (200 tokens)
- C: Conversational (300 tokens)
- D: Code-like (400 tokens)
- E: Control (100 tokens)

**Results**:
- A: 72% success ← Winner
- B: 65% success ← Runner-up
- C: 60% success
- D: 58% success
- E: 35% success (baseline)

**Generation 1** (Mutations from A+B):
- A1: A with simplified language (-100 tokens)
- A2: A with more examples (+200 tokens)
- AB1: A's patterns + B's bullet format (crossover)
- AB2: A's examples + B's simplicity (crossover)
- A-NL: A specialized for natural language queries

**Results**:
- A2: 75% success ← Winner (more examples helped!)
- A1: 73% success
- AB1: 71% success
- AB2: 68% success
- A-NL: 70% success

**Generation 2** (Mutations from A2):
- A2.1: A2 with even more examples (+100 tokens)
- A2.2: A2 with better example selection (same tokens)
- A2.3: A2 with different formatting
- A2-short: A2 compressed to 400 tokens
- A2-long: A2 expanded to 800 tokens

**Continue until convergence** (3-4 iterations typically)

---

## Testing Infrastructure

### Test Query Set

**Requirements**:
- Representative of real usage
- Diverse query types
- Stable (same queries across variants)
- Ground truth (known correct results)

**Composition** (100 queries):
```yaml
Natural language questions: 30 queries
  - "How does X work?"
  - "What handles Y?"
  - "Where is Z?"
  - "Find the A in B"

Simple 2-3 word queries: 30 queries
  - "error handling"
  - "cart checkout"
  - "auth middleware"

Complex multi-word: 20 queries
  - "shopping cart total calculation"
  - "user authentication middleware handler"

Edge cases: 20 queries
  - camelCase: "processCheckout"
  - snake_case: "validate_cart_items"
  - File paths: "src/cart/checkout.ts"
  - Single words: "authentication"
```

**Storage**:
```json
{
  "test_queries": [
    {
      "id": "NL-001",
      "type": "natural_language",
      "query": "How does authentication work?",
      "expected_results": ["auth.ts", "middleware/auth.ts"],
      "min_results": 3
    },
    {
      "id": "SIMPLE-001",
      "type": "simple",
      "query": "error handling",
      "expected_results": ["error.ts", "handlers/error.ts"],
      "min_results": 5
    }
    // ... 98 more
  ]
}
```

### Testing Harness

**Automated testing script** (`test/variant-tester.js`):

```javascript
async function testVariant(variantId, toolDescription, testQueries) {
  const results = {
    variant_id: variantId,
    total_queries: testQueries.length,
    successful_queries: 0,
    failed_queries: 0,
    avg_result_count: 0,
    avg_top_score: 0,
    query_results: []
  }

  for (const testQuery of testQueries) {
    // Simulate agent with this tool description
    const agentQuery = await simulateAgentTransformation(
      testQuery.query,
      toolDescription
    )

    // Execute search
    const searchResults = await executeSearch(agentQuery)

    // Evaluate results
    const success = searchResults.length >= testQuery.min_results
    const relevance = checkRelevance(searchResults, testQuery.expected_results)

    results.query_results.push({
      query_id: testQuery.id,
      original_query: testQuery.query,
      transformed_query: agentQuery,
      result_count: searchResults.length,
      top_score: searchResults[0]?.score || 0,
      success: success,
      relevance: relevance
    })

    if (success) results.successful_queries++
    else results.failed_queries++

    results.avg_result_count += searchResults.length
    results.avg_top_score += searchResults[0]?.score || 0
  }

  results.success_rate = results.successful_queries / results.total_queries
  results.avg_result_count /= results.total_queries
  results.avg_top_score /= results.total_queries

  return results
}

// Run all variants
async function runExperiment(variants, testQueries) {
  const results = []

  for (const variant of variants) {
    console.log(`Testing ${variant.id}...`)
    const result = await testVariant(variant.id, variant.description, testQueries)
    results.push(result)
  }

  // Statistical analysis
  const analysis = performStatisticalAnalysis(results)

  // Determine winner
  const winner = selectWinner(results, analysis)

  return { results, analysis, winner }
}
```

### Agent Simulation

**Challenge**: How to simulate Claude's transformation behavior?

**Option 1: API-based (Accurate but Expensive)**
```javascript
async function simulateAgentTransformation(query, toolDescription) {
  const response = await anthropic.messages.create({
    model: 'claude-sonnet-4-5',
    messages: [{
      role: 'user',
      content: `You have this search tool:

${toolDescription}

User asks: "${query}"

What query would you send to the search tool? Reply with ONLY the query, nothing else.`
    }]
  })

  return response.content[0].text.trim()
}
```

**Cost**: ~$0.002 per query × 100 queries × 5 variants = $1 per experiment
**Accuracy**: High (actual Claude behavior)

**Option 2: LLM-based (Cheaper)**
```javascript
async function simulateAgentTransformation(query, toolDescription) {
  // Use Haiku instead of Sonnet
  const response = await anthropic.messages.create({
    model: 'claude-haiku',
    // ... same as above
  })

  return response.content[0].text.trim()
}
```

**Cost**: ~$0.0002 per query × 100 × 5 = $0.10 per experiment
**Accuracy**: Medium (Haiku may behave differently than Sonnet)

**Option 3: Rule-based (Free but Inaccurate)**
```javascript
function simulateAgentTransformation(query, toolDescription) {
  // Parse description for patterns
  const patterns = extractPatterns(toolDescription)

  // Apply patterns to query
  let transformed = query.toLowerCase()

  // Remove question words if description mentions it
  if (toolDescription.includes('Remove question words')) {
    transformed = removeQuestionWords(transformed)
  }

  // Extract terms if description mentions it
  if (toolDescription.includes('Extract technical terms')) {
    transformed = extractTechnicalTerms(transformed)
  }

  return transformed
}
```

**Cost**: $0
**Accuracy**: Low (doesn't capture agent's reasoning)

**Recommended**: Start with Option 3 for rapid iteration, validate with Option 1 for final winner

### Metrics Collection

**Primary Metrics**:
```typescript
interface VariantMetrics {
  // Success rates
  overall_success_rate: number     // % with ≥3 results
  natural_language_success: number // % of NL queries successful
  simple_query_success: number     // % of simple queries successful

  // Quality
  avg_result_count: number         // Mean results per query
  avg_top_score: number            // Mean score of top result
  top3_relevance: number           // % with ≥2 relevant in top 3

  // Behavior
  transformation_rate: number      // % of queries transformed
  retry_simulation: number         // % that would retry (estimated)

  // Efficiency
  token_count: number              // Tool description size

  // Statistical
  confidence_interval_95: [number, number]
  p_value_vs_control: number
  sample_size: number
}
```

**Statistical Analysis**:
```javascript
function performStatisticalAnalysis(results) {
  return {
    // Pairwise comparisons
    comparisons: results.map((a, i) =>
      results.slice(i + 1).map(b => ({
        variant_a: a.variant_id,
        variant_b: b.variant_id,
        difference: a.success_rate - b.success_rate,
        p_value: tTest(a.query_results, b.query_results),
        significant: tTest(a, b) < 0.05
      }))
    ).flat(),

    // Overall ranking
    ranking: results
      .sort((a, b) => b.success_rate - a.success_rate)
      .map((r, i) => ({
        rank: i + 1,
        variant_id: r.variant_id,
        success_rate: r.success_rate,
        ci_95: calculateCI(r.query_results, 0.95)
      })),

    // Recommendations
    clear_winner: detectClearWinner(results),
    requires_more_data: checkSampleSize(results)
  }
}
```

---

## Experimentation Process

### Experiment 1: Initial Variants (Week 1)

**Goal**: Establish baseline, validate framework

**Variants**:
- A: Detailed patterns (500 tokens)
- B: Simple bullets (200 tokens)
- C: Conversational (300 tokens)
- D: Control (100 tokens)

**Test set**: 20 queries (quick validation)

**Process**:
```bash
# 1. Create variant files
cat > variants/A-detailed.json << EOF
{
  "id": "A-detailed",
  "description": "...",
  "tokens": 500
}
EOF

# 2. Run test
node test/run-experiment.js \
  --variants variants/*.json \
  --queries test-queries-20.json \
  --output results/exp-001.json

# 3. Analyze
node test/analyze-results.js results/exp-001.json

# Output:
# Variant A: 75% success (15/20)
# Variant B: 65% success (13/20)
# Variant C: 60% success (12/20)
# Variant D: 35% success (7/20)
#
# Winner: Variant A (p<0.05 vs control)
# Recommendation: Proceed with A, generate mutations
```

**Time**: 4 hours
**Cost**: $0 (rule-based simulation)

### Experiment 2: Mutations (Week 2)

**Goal**: Improve on winner from Exp 1

**Variants** (generated from A):
- A1: A with fewer examples
- A2: A with more examples
- A3: A with different formatting
- A4: A compressed to 300 tokens
- A5: Control (baseline)

**Test set**: 100 queries (statistical rigor)

**Process**:
```bash
# 1. Generate mutations
node test/generate-mutations.js \
  --parent variants/A-detailed.json \
  --count 4 \
  --output variants/generation-2/

# 2. Run experiment
node test/run-experiment.js \
  --variants variants/generation-2/*.json \
  --queries test-queries-100.json \
  --simulation llm-haiku \
  --output results/exp-002.json

# 3. Analyze with statistical rigor
node test/analyze-results.js \
  --results results/exp-002.json \
  --min-confidence 0.95 \
  --output results/exp-002-analysis.json

# Output:
# Variant A2: 78% success (78/100) CI: [70-86%]
# Variant A1: 74% success (74/100) CI: [66-82%]
# Variant A3: 72% success (72/100) CI: [64-80%]
# Variant A4: 70% success (70/100) CI: [62-78%]
# Variant A5: 38% success (38/100) CI: [30-46%]
#
# Clear winner: A2 (p=0.03 vs A1, p<0.001 vs A5)
# Recommendation: Deploy A2, generate next iteration
```

**Time**: 6 hours
**Cost**: $0.10 (Haiku simulation)

### Experiment 3: Convergence (Week 3)

**Goal**: Fine-tune winner, check for convergence

**Variants** (generated from A2):
- A2.1: A2 with refined examples
- A2.2: A2 with better formatting
- A2.3: A2 with simplified language
- A2: Parent (to check for regression)
- Control: Baseline

**Test set**: 100 queries

**Expected outcome**:
```
A2.2: 79% success (marginal improvement)
A2.1: 78% success (no improvement)
A2.3: 77% success (slight regression)
A2:   78% success (baseline)

Conclusion: Converged (improvements <2% not significant)
Decision: Deploy A2, monitor in production
```

---

## Production A/B Testing

### Infrastructure Setup

**Requirements**:
1. Variant assignment (50/50 split or multi-armed bandit)
2. Metrics collection from real usage
3. Statistical analysis engine
4. Automated winner promotion

**Implementation** (Phase 3):

```typescript
// Variant assignment middleware
class VariantAssigner {
  assign(userId: string): string {
    // Consistent assignment (same user = same variant)
    const hash = hashUserId(userId)

    if (this.strategy === 'ab-test') {
      return hash % 2 === 0 ? 'variant-a' : 'variant-b'
    } else if (this.strategy === 'multi-armed-bandit') {
      return this.bandit.selectArm(userId)
    }
  }
}

// Metrics collector
class MetricsCollector {
  async recordQuery(userId: string, variant: string, query: string, results: SearchResults) {
    await db.insert('query_metrics', {
      timestamp: Date.now(),
      user_id: userId,
      variant: variant,
      query_original: query,
      result_count: results.length,
      top_score: results[0]?.score,
      success: results.length >= 3
    })
  }
}

// Analysis engine (runs daily)
class ABTestAnalyzer {
  async analyze(variantA: string, variantB: string, minSamples: number = 1000) {
    const metricsA = await db.query('SELECT * FROM query_metrics WHERE variant = ?', [variantA])
    const metricsB = await db.query('SELECT * FROM query_metrics WHERE variant = ?', [variantB])

    if (metricsA.length < minSamples || metricsB.length < minSamples) {
      return { status: 'insufficient_data', recommendation: 'continue' }
    }

    const successRateA = metricsA.filter(m => m.success).length / metricsA.length
    const successRateB = metricsB.filter(m => m.success).length / metricsB.length

    const pValue = tTest(metricsA, metricsB)
    const significant = pValue < 0.05

    const winner = successRateA > successRateB ? variantA : variantB
    const improvement = Math.abs(successRateA - successRateB)

    return {
      status: 'complete',
      winner: winner,
      improvement: improvement,
      p_value: pValue,
      significant: significant,
      recommendation: significant ? 'promote_winner' : 'continue'
    }
  }
}
```

### Multi-Armed Bandit (Advanced)

**Alternative to 50/50 A/B**: Dynamically allocate traffic to better variant

```typescript
class ThompsonSampling {
  // Beta distribution for each variant
  variants: Map<string, { alpha: number, beta: number }>

  selectArm(userId: string): string {
    // Sample from each variant's beta distribution
    const samples = Array.from(this.variants.entries()).map(([id, params]) => ({
      id,
      sample: betaRandom(params.alpha, params.beta)
    }))

    // Select highest sample (exploration vs exploitation balance)
    return samples.sort((a, b) => b.sample - a.sample)[0].id
  }

  update(variantId: string, success: boolean) {
    const params = this.variants.get(variantId)
    if (success) {
      params.alpha += 1  // Reward
    } else {
      params.beta += 1   // Penalty
    }
  }
}
```

**Benefit**: Automatically allocates more traffic to better variant, faster convergence

---

## Continuous Optimization Pipeline

### Automated Pipeline (Future State)

```
Weekly cycle:
  Monday: Generate 5 new mutations from current champion
  Tuesday: Run automated tests (100 queries per variant)
  Wednesday: Analyze results, select new champion
  Thursday: Deploy champion to 10% of production traffic
  Friday-Sunday: Monitor, collect metrics

Monthly review:
  Analyze trends
  Identify new query patterns
  Generate specialized variants
  Report to team
```

### Implementation

**CI/CD integration**:
```yaml
# .github/workflows/optimize-tool-descriptions.yml
name: Tool Description Optimization

on:
  schedule:
    - cron: '0 0 * * 1'  # Every Monday

jobs:
  optimize:
    runs-on: ubuntu-latest
    steps:
      - name: Generate mutations
        run: node scripts/generate-mutations.js --count 5

      - name: Run experiments
        run: node scripts/run-experiments.js --queries 100

      - name: Analyze results
        run: node scripts/analyze-results.js --min-confidence 0.95

      - name: Create PR for winner
        if: steps.analyze.outputs.improvement > 2%
        run: |
          git checkout -b optimize/week-${{ github.run_number }}
          cp results/winner.json src/tool-description.json
          git commit -m "feat: optimize tool description (+${{ steps.analyze.outputs.improvement }}%)"
          gh pr create --title "Tool description optimization" --body "..."
```

---

## Success Criteria

### Framework Success

- [ ] Can run experiments reliably (reproducible results)
- [ ] Can measure statistical significance
- [ ] Can generate mutations automatically
- [ ] Can identify clear winners (p<0.05)
- [ ] Finds improvements >5% within 3 iterations

### Optimization Success

- [ ] Week 1: Establish baseline
- [ ] Week 2: Find +10% improvement vs baseline
- [ ] Week 3: Find +5% improvement vs Week 2
- [ ] Week 4: Convergence (<2% further improvement)
- [ ] Deploy champion with +15% total improvement

---

## Risks and Mitigations

### Risk 1: Overfitting to Test Set

**Problem**: Variants optimize for test queries, not real usage

**Mitigation**:
- Diverse test set (100+ queries)
- Regular test set refreshes (add new real queries monthly)
- Live A/B testing validates lab results
- Monitor production metrics vs test metrics

### Risk 2: Measurement Noise

**Problem**: Results vary due to random chance

**Mitigation**:
- Require statistical significance (p<0.05)
- Minimum sample size (n≥100)
- Confidence intervals on all metrics
- A/A testing to measure noise floor

### Risk 3: Local Optima

**Problem**: Converge on local maximum, miss global maximum

**Mitigation**:
- Periodic "exploration" iterations (random mutations)
- Test radically different variants every 4th iteration
- Track diversity score of variant pool
- Annual "reset" with fresh approaches

---

## Recommended Start

### MVP (This Week)

1. **Create 3 variants** (4 hours)
   - Detailed patterns
   - Simple bullets
   - Control

2. **Test with 20 queries** (2 hours)
   - Manual or rule-based simulation
   - Record results in spreadsheet

3. **Analyze and pick winner** (1 hour)
   - Simple % comparison
   - Deploy winner

4. **Generate 3 mutations** (2 hours)
   - Iterate on winner
   - Repeat test

**Total**: 9 hours, $0 cost, proof of concept

### Next Steps (Week 2+)

1. Build automated testing harness
2. Expand to 100 query test set
3. Implement LLM-based simulation
4. Statistical analysis tooling
5. Production A/B testing infrastructure

---

## Conclusion

**The framework transforms tool description optimization from art to science**:

- **Before**: Guess what works, hope for the best
- **After**: Empirical optimization, continuous improvement

**Expected outcomes**:
- Week 1: +10% improvement vs baseline
- Week 3: +15% total improvement
- Week 6: Convergence at +18-20% improvement
- Ongoing: +1-2% per quarter through continuous optimization

**Investment**:
- MVP: 9 hours, $0
- Full framework: 40 hours, $50 in LLM costs
- Continuous: 2 hours/week, $10/month

**ROI**: If tool description improves query success by 20%, and 100 users save 30 minutes/week each, that's 50 hours/week = $5,000/week value created. Payback in <1 week.

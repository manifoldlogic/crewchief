# Quality Strategy: Test Design Validation

## Philosophy

Testing a test framework requires meta-level thinking. We're not testing code behavior—we're testing whether our tasks accurately measure what we claim they measure. This is closer to psychometric test validation than software testing.

**Core Principle**: Pragmatic rigor over ceremonial coverage. Every validation step must provide actionable insight that improves task quality or proves real-world utility.

## Quality Dimensions

### 1. Construct Validity
**Question**: Do tasks measure what they claim to measure?

**What We're Testing**:
- Task labeled "grep-impossible" actually defeats grep
- Task labeled "relationship discovery" requires code graph understanding
- Difficulty ratings match actual agent performance

**How We Test**:
```typescript
// Validation: Run grep baseline
const grepResult = await runTask(task, { tools: ['grep', 'glob', 'read'] })

// Claim: "Grep-impossible"
if (task.category === 'grep-impossible') {
  expect(grepResult.success).toBeLessThan(0.3)  // <30% success
}

// Claim: "Grep-hard"
if (task.category === 'grep-hard') {
  expect(grepResult.success).toBeBetween(0.3, 0.6)  // 30-60% success
  expect(grepResult.time).toBeGreaterThan(searchResult.time * 1.5)  // >50% slower
}
```

**Pass Criteria**: 80% of tasks match their difficulty classification

### 2. Discriminant Validity
**Question**: Do search and grep tools perform differently on tasks?

**What We're Testing**:
- Search significantly outperforms grep on search-favorable tasks
- Grep performs adequately on grep-appropriate tasks
- There's a clear performance gap, not noise

**How We Test**:
```typescript
// Run both conditions
const grepOnly = await runTaskSuite(tasks, { enableSearch: false })
const searchAvailable = await runTaskSuite(tasks, { enableSearch: true })

// Statistical test
const diff = searchAvailable.avgScore - grepOnly.avgScore

// Require significant difference
expect(diff).toBeGreaterThan(0.30)  // 30% improvement
expect(tTest(grepOnly.scores, searchAvailable.scores).p).toBeLessThan(0.05)
```

**Pass Criteria**: p < 0.05 for score difference on grep-hard/impossible tasks

### 3. Ecological Validity
**Question**: Do tasks reflect real-world developer activities?

**What We're Testing**:
- Tasks are based on actual scenarios (code review, debugging, refactoring)
- Developers would recognize and perform these tasks
- Task frequency matches real-world occurrence

**How We Test**:
```typescript
// Validation survey (manual)
interface TaskReview {
  task: SearchTask
  reviewers: Developer[]
  questions: {
    wouldActuallyDo: boolean  // "Would you do this in real work?"
    howOften: 'daily' | 'weekly' | 'monthly' | 'rarely'
    isRealistic: 1-5  // "How realistic is this scenario?"
    wouldHelpMe: boolean  // "Would semantic search help here?"
  }
}

// Pass criteria
const avgWouldDo = reviews.filter(r => r.wouldActuallyDo).length / reviews.length
expect(avgWouldDo).toBeGreaterThan(0.7)  // 70%+ say they'd do it
```

**Pass Criteria**: 70%+ developer approval for realism (manual review)

### 4. Test-Retest Reliability
**Question**: Do tasks produce consistent results across runs?

**What We're Testing**:
- Same task + same tools → same outcome (within variance)
- Results aren't dependent on random seed, agent mood, etc.
- Objective criteria produce deterministic scores

**How We Test**:
```typescript
// Run same task 5 times
const runs = await Promise.all(
  Array(5).fill(task).map(t => runTask(t, config))
)

// Calculate variance
const scores = runs.map(r => r.score)
const mean = scores.reduce((a, b) => a + b) / scores.length
const variance = scores.map(s => (s - mean) ** 2).reduce((a, b) => a + b) / scores.length

// Low variance = high reliability
expect(variance).toBeLessThan(0.05)  // <5% variance
```

**Pass Criteria**: Variance < 10% for each task across 5 runs

### 5. Predictive Validity
**Question**: Do task results predict real-world tool performance?

**What We're Testing**:
- High scores on benchmark → actual usefulness in practice
- Low scores on benchmark → tool isn't helpful for those scenarios
- Benchmark rankings match user satisfaction

**How We Test**:
```typescript
// Long-term validation (manual)
// 1. Deploy tool to users
// 2. Track actual usage patterns
// 3. Survey satisfaction
// 4. Correlate with benchmark performance

interface ValidationStudy {
  tool: 'semantic-search' | 'grep'
  users: Developer[]
  usageMetrics: {
    frequency: number  // uses per week
    successRate: number  // % of searches that led to desired outcome
    satisfaction: 1-5
  }
  benchmarkScore: number
}

// Correlation test
const correlation = pearsonCorrelation(
  studies.map(s => s.benchmarkScore),
  studies.map(s => s.usageMetrics.satisfaction)
)

expect(correlation).toBeGreaterThan(0.6)  // Strong positive correlation
```

**Pass Criteria**: r > 0.6 correlation between benchmark scores and user satisfaction (future validation)

## Testing Strategy

### Phase 1: Unit Validation (Individual Tasks)

**Goal**: Ensure each task is well-formed and measures what it claims

**Tests**:
```typescript
describe('Task: Find Transitive Dependencies', () => {
  it('has objective success criteria', () => {
    const task = TASK_FIND_TRANSITIVE_DEPS
    expect(task.successValidator).toBeDefined()
    expect(task.successCriteria).toBeObjective()  // No "good explanation" fuzziness
  })

  it('defeats grep baseline', async () => {
    const result = await runTask(task, { tools: ['grep', 'glob', 'read'] })
    expect(result.success).toBeLessThan(0.3)
  })

  it('succeeds with semantic search', async () => {
    const result = await runTask(task, { tools: ['grep', 'glob', 'read', 'search'] })
    expect(result.success).toBeGreaterThan(0.7)
  })

  it('has significant performance difference', async () => {
    const grep = await runGrepBaseline(task)
    const search = await runSearchCondition(task)
    const improvement = search.score - grep.score
    expect(improvement).toBeGreaterThan(0.3)
  })

  it('is deterministic', async () => {
    const runs = await runTaskMultiple(task, 5)
    const variance = calculateVariance(runs.map(r => r.score))
    expect(variance).toBeLessThan(0.1)
  })
})
```

**Pass Criteria**: All tasks pass their unit validation

### Phase 2: Suite Validation (Task Collection)

**Goal**: Ensure task collection is balanced, comprehensive, diverse

**Tests**:
```typescript
describe('Benchmark Suite: Tier 1 Grep-Impossible', () => {
  it('covers all major categories', () => {
    const suite = TIER1_SUITE
    const categories = new Set(suite.tasks.map(t => t.category))

    expect(categories).toContain('relationship-discovery')
    expect(categories).toContain('negative-space')
    expect(categories).toContain('architectural-understanding')
  })

  it('has difficulty distribution', () => {
    const suite = TIER1_SUITE
    const difficulties = suite.tasks.map(t => t.difficulty)

    // Want mix of easy, medium, hard within "grep-impossible"
    expect(difficulties).toContain('easy')
    expect(difficulties).toContain('medium')
    expect(difficulties).toContain('hard')
  })

  it('all tasks defeat grep', async () => {
    const suite = TIER1_SUITE
    const results = await runSuite(suite, { tools: ['grep', 'glob', 'read'] })

    const failureRate = results.filter(r => r.success < 0.3).length / results.length
    expect(failureRate).toBeGreaterThan(0.8)  // 80%+ defeat grep
  })

  it('significant search advantage', async () => {
    const grepResults = await runSuite(TIER1_SUITE, { enableSearch: false })
    const searchResults = await runSuite(TIER1_SUITE, { enableSearch: true })

    const avgImprovement = mean(searchResults.map(s => s.score)) - mean(grepResults.map(g => g.score))
    expect(avgImprovement).toBeGreaterThan(0.40)  // 40%+ improvement
  })
})
```

**Pass Criteria**: All suite-level tests pass

### Phase 3: Integration Validation (With Optimization)

**Goal**: Ensure tasks work correctly with genetic optimization framework

**Tests**:
```typescript
describe('Integration: Genetic Optimization', () => {
  it('optimizes for correct tool selection', async () => {
    // Run 3 generations
    const result = await runGeneticIterations({
      tasks: [TASK_GREP_IMPOSSIBLE_1, TASK_GREP_IMPOSSIBLE_2],
      maxIterations: 3,
      populationSize: 5
    })

    // Best variant should trigger more search usage
    const gen1SearchUsage = result.generations[0].avgSearchUsage
    const gen3SearchUsage = result.generations[2].avgSearchUsage

    expect(gen3SearchUsage).toBeGreaterThan(gen1SearchUsage * 1.5)  // 50%+ increase
  })

  it('scores reflect search effectiveness', async () => {
    const goodSearchTask = TASK_RELATIONSHIP_DISCOVERY
    const badSearchTask = TASK_SIMPLE_FILE_FIND  // Grep is fine for this

    const goodScore = await evaluateTask(goodSearchTask, variantWithSearchGuidance)
    const badScore = await evaluateTask(badSearchTask, variantWithSearchGuidance)

    // Good task should score higher (search helps)
    // Bad task should score same/lower (search overhead)
    expect(goodScore).toBeGreaterThan(badScore)
  })
})
```

**Pass Criteria**: Integration tests pass, optimization improves search usage on appropriate tasks

### Phase 4: Cross-Project Validation

**Goal**: Ensure tasks generalize across codebases

**Tests**:
```typescript
describe('Generalization: Cross-Project', () => {
  it('works on different TypeScript projects', async () => {
    const codebases = ['crewchief', 'vscode', 'typescript']
    const task = TASK_FIND_ERROR_HANDLING

    const results = await Promise.all(
      codebases.map(cb => runTaskOnCodebase(task, cb))
    )

    const successRate = results.filter(r => r.success > 0.7).length / results.length
    expect(successRate).toBeGreaterThan(0.6)  // 60%+ generalization
  })

  it('adapts to different languages', async () => {
    const task = TASK_FIND_RETRY_LOGIC

    const ts = await runTaskOnCodebase(task, 'ts-project')
    const py = await runTaskOnCodebase(task, 'py-project')
    const rust = await runTaskOnCodebase(task, 'rust-project')

    // All should work (concept is universal)
    expect(ts.success).toBeGreaterThan(0.6)
    expect(py.success).toBeGreaterThan(0.6)
    expect(rust.success).toBeGreaterThan(0.6)
  })
})
```

**Pass Criteria**: 60%+ success rate across 3+ different codebases

## What We DON'T Test

### Anti-Pattern 1: Exhaustive Coverage
**Don't**: Test every possible task variation
**Why**: Diminishing returns, ceremonial testing
**Instead**: Test representative samples from each category

### Anti-Pattern 2: Perfect Reliability
**Don't**: Require 100% deterministic results
**Why**: LLM agents have inherent variance
**Instead**: Accept <10% variance, focus on trends

### Anti-Pattern 3: Premature Optimization
**Don't**: Optimize task design before validation
**Why**: Don't know what works yet
**Instead**: Iterate based on failure analysis

### Anti-Pattern 4: Enterprise Theatre
**Don't**: Test for the sake of test coverage metrics
**Why**: Tests should inform decisions, not check boxes
**Instead**: Each test must answer a specific quality question

## Pragmatic Testing Approach

### Tier 1: Must-Have Tests (Block Deployment)

1. **Grep Baseline**: Task actually defeats grep
2. **Search Advantage**: Semantic search significantly helps
3. **Determinism**: Results are reproducible within variance
4. **Objective Criteria**: Success is measurable, not subjective

**When to Run**: Before adding task to benchmark suite

### Tier 2: Should-Have Tests (Warn, Don't Block)

1. **Category Coverage**: Suite covers all major task types
2. **Difficulty Balance**: Mix of easy/medium/hard within tier
3. **Integration**: Works with genetic optimization
4. **Ecological Review**: Developer feedback positive

**When to Run**: Before major releases, quarterly reviews

### Tier 3: Nice-to-Have Tests (Long-Term Validation)

1. **Cross-Project**: Generalizes to other codebases
2. **Predictive Validity**: Correlates with user satisfaction
3. **Trend Analysis**: Performance improves over time
4. **Cost-Benefit**: ROI of using semantic search

**When to Run**: After deployment, ongoing monitoring

## Failure Analysis Framework

When a task fails validation, categorize the failure:

### Failure Type 1: Task Too Easy
**Symptom**: Grep succeeds >60%
**Root Cause**: Task has obvious keywords or simple file structure
**Fix**: Add anti-keyword constraints, increase conceptual complexity
**Example**: "Find worktree.ts" → "Find code that manages parallel git repositories"

### Failure Type 2: Task Too Hard
**Symptom**: Both grep AND search fail
**Root Cause**: Task requires knowledge outside codebase, ambiguous criteria
**Fix**: Simplify task, add context, or clarify success criteria
**Example**: "Find security vulnerabilities" → "Find code that doesn't validate user input"

### Failure Type 3: Insufficient Advantage
**Symptom**: Search only marginally better than grep (<20% improvement)
**Root Cause**: Task isn't leveraging search strengths
**Fix**: Redesign to emphasize relationships, concepts, or complexity
**Example**: "Find auth.ts" → "Find all code that depends on authentication state"

### Failure Type 4: Unreliable Results
**Symptom**: High variance across runs (>20%)
**Root Cause**: Subjective success criteria, random agent behavior
**Fix**: Make criteria objective, binary checks
**Example**: "Good explanation" → "Mentions specific function names X, Y, Z"

### Failure Type 5: Ecological Invalid
**Symptom**: Developers say "I wouldn't actually do this"
**Root Cause**: Synthetic task, not based on real scenarios
**Fix**: Ground in actual code review, debugging, or refactoring needs
**Example**: "Find all variables named X" → "Find where state is modified during checkout"

## Continuous Improvement

### Monthly Review Process

1. **Analyze Failures**: Review all failed task validations
2. **Categorize**: Map failures to types above
3. **Identify Patterns**: Are certain categories consistently problematic?
4. **Update Guidelines**: Refine task design principles
5. **Deprecate**: Remove tasks that don't provide value
6. **Create**: Design new tasks addressing gaps

### Quarterly Validation

1. **Cross-Project Test**: Run suite on 3 new codebases
2. **Developer Survey**: Get feedback on task realism
3. **Usage Analysis**: If deployed, check actual tool usage patterns
4. **Benchmark Update**: Refresh tasks based on new patterns

### Annual Review

1. **Predictive Validity**: Correlate benchmark scores with user satisfaction
2. **Industry Comparison**: How do our tasks compare to other IR benchmarks?
3. **Framework Evolution**: Should we change fundamental approach?
4. **Publication**: Document learnings, contribute to research

## Success Metrics

### Short-Term (3 months)
- [ ] 20 tasks pass Tier 1 validation
- [ ] 3 benchmark suites (Tier 1, 2, 3) defined
- [ ] 80%+ construct validity
- [ ] Integration tests pass

### Medium-Term (6 months)
- [ ] 50 tasks across all categories
- [ ] Cross-project validation on 3 codebases
- [ ] Developer survey shows 70%+ ecological validity
- [ ] Genetic optimization shows consistent improvement

### Long-Term (12 months)
- [ ] 100 task library
- [ ] Predictive validity r > 0.6
- [ ] Published research/blog post
- [ ] Industry adoption (other tools use our benchmark)

## Tooling

### Test Infrastructure
```typescript
// Validation runner
class TaskValidator {
  async validateTask(task: SearchTask): Promise<ValidationResult> {
    const checks = await Promise.all([
      this.checkGrepBaseline(task),
      this.checkSearchAdvantage(task),
      this.checkDeterminism(task),
      this.checkObjectiveCriteria(task)
    ])

    return {
      task,
      passed: checks.every(c => c.passed),
      checks,
      recommendations: this.generateRecommendations(checks)
    }
  }
}

// Usage
const validator = new TaskValidator()
const result = await validator.validateTask(TASK_NEW)

if (!result.passed) {
  console.log('Validation failed:', result.checks)
  console.log('Recommendations:', result.recommendations)
}
```

### Reporting
```typescript
// Generate validation report
function generateValidationReport(results: ValidationResult[]): string {
  const summary = {
    totalTasks: results.length,
    passed: results.filter(r => r.passed).length,
    failed: results.filter(r => !r.passed).length,

    failuresByType: groupBy(
      results.filter(r => !r.passed),
      r => r.failureCategory
    )
  }

  return formatReport(summary)
}
```

## Conclusion

Quality strategy for test design is inherently meta—we're testing our tests. The approach must be:

1. **Rigorous**: Validated against multiple quality dimensions
2. **Pragmatic**: Only test what provides actionable insights
3. **Iterative**: Continuous improvement based on failures
4. **Scientific**: Statistical validation, not gut feeling
5. **Practical**: Grounded in real-world scenarios

The goal isn't perfect tests—it's tests that reliably distinguish between genuinely useful semantic search and false claims of utility. If we can build a benchmark where high scores predict real-world value, we've succeeded.

# Validation Guide: Ensuring Task Quality

## Overview

This guide explains how to validate grep-impossible tasks across five quality dimensions. Validation ensures tasks measure what they claim to measure, produce consistent results, and reflect real-world developer scenarios.

### Why Validation Matters

Testing a test framework requires meta-level thinking. We're not testing code behavior—we're testing whether our tasks accurately measure semantic search value. This is closer to psychometric test validation than software testing.

**Philosophy**: Pragmatic rigor over ceremonial coverage. Every validation step must provide actionable insight that improves task quality or proves real-world utility.

### Quality Philosophy

We care about:
1. **Does it measure correctly?** (Construct & discriminant validity)
2. **Is it realistic?** (Ecological validity)
3. **Is it consistent?** (Test-retest reliability)
4. **Is it statistically sound?** (Statistical power)

We don't care about:
- Perfect reliability (LLMs have inherent variance)
- Exhaustive coverage (test representative samples)
- Enterprise theater (tests must inform decisions)

## The Five Quality Dimensions

### Dimension 1: Construct Validity

**Question**: Does the task measure what it claims to measure?

**What We're Testing**:
- Task labeled "grep-impossible" actually defeats grep
- Task labeled "relationship discovery" requires code graph understanding
- Difficulty ratings match actual performance

**Why It Matters**: If a "grep-impossible" task has 70% grep success, it's mislabeled. Our framework loses credibility.

**How to Test**:

```typescript
import { validateTask, DEFAULT_THRESHOLDS } from '../validation/task-validator.js'

// Run validation in mock mode (fast, no LLM execution)
const result = await validateTask({
  task: myTask,
  tier: 'tier1-impossible',
  useMockData: true
})

// Check construct validity
const constructResult = result.dimensions.constructValidity

if (constructResult.passed) {
  console.log('✓ Task defeats grep as expected')
  console.log(`  Grep success: ${constructResult.actual}`)
} else {
  console.log('✗ Task is too easy for grep')
  console.log(`  Grep success: ${constructResult.actual}, expected: ${constructResult.expected}`)
  console.log(`  Recommendation: ${constructResult.details}`)
}
```

**Thresholds by Tier**:

| Tier | Max Grep Success | Interpretation |
|------|-----------------|----------------|
| Tier 1 (Impossible) | 30% | Grep should mostly fail |
| Tier 2 (Hard) | 60% | Grep might succeed sometimes |
| Tier 3 (Real-world) | 80% | More about realism than difficulty |

**Pass Criteria**: 80% of tasks in a suite match their difficulty classification

**Fixing Failed Construct Validity**:

If grep succeeds more than threshold:
1. Apply anti-keyword pattern more aggressively
2. Require transitive relationships, not direct matches
3. Add semantic complexity (conceptual queries)
4. Consider moving to easier tier

Example:
```typescript
// Failed construct validity (grep succeeds 70%)
{
  description: "Find the WorktreeManager class",
  expectedGrepSuccess: 0.70  // Too high for Tier 1
}

// Fixed (grep succeeds 20%)
{
  description: "Find code responsible for managing parallel Git checkouts",
  expectedGrepSuccess: 0.20  // Appropriate for Tier 1
}
```

### Dimension 2: Discriminant Validity

**Question**: Do semantic search and grep perform differently on this task?

**What We're Testing**:
- Semantic search significantly outperforms grep
- The performance gap is large enough to matter
- Improvement is statistically significant

**Why It Matters**: If search is only marginally better than grep, the task doesn't demonstrate search value. We need clear differentiation.

**How to Test**:

```typescript
const result = await validateTask({
  task: myTask,
  tier: 'tier1-impossible',
  useMockData: true
})

const discriminantResult = result.dimensions.discriminantValidity

if (discriminantResult.passed) {
  console.log('✓ Semantic search provides clear advantage')
  console.log(`  Search success: ${discriminantResult.actual}`)
  console.log(`  Improvement: ${discriminantResult.details}`)
} else {
  console.log('✗ Search advantage insufficient')
  console.log(`  ${discriminantResult.details}`)
}
```

**Thresholds by Tier**:

| Tier | Min Search Success | Min Advantage | Min p-value |
|------|-------------------|---------------|-------------|
| Tier 1 | 70% | 30pp | 0.05 |
| Tier 2 | 70% | 20pp | 0.05 |
| Tier 3 | 60% | 10pp | 0.05 |

**Statistical Significance**:

In real mode (with actual executions), we use a t-test:

```typescript
import { tTest } from '../reporting/statistics.js'

// Run task 5 times with grep-only
const grepScores = await Promise.all(
  Array(5).fill(task).map(() => runGrepBaseline(task))
)

// Run task 5 times with search available
const searchScores = await Promise.all(
  Array(5).fill(task).map(() => runWithSearch(task))
)

// Test for significant difference
const testResult = tTest(grepScores, searchScores)

if (testResult.p < 0.05) {
  console.log('✓ Difference is statistically significant')
  console.log(`  p-value: ${testResult.p.toFixed(4)}`)
  console.log(`  Effect size: ${testResult.effectSize.toFixed(2)}`)
} else {
  console.log('✗ Difference not statistically significant')
  console.log(`  p-value: ${testResult.p.toFixed(4)} (need < 0.05)`)
}
```

**Pass Criteria**: p < 0.05 for score difference on grep-hard/impossible tasks

**Fixing Failed Discriminant Validity**:

**Problem 1: Search success too low (<60%)**
- Task may be too hard even for semantic search
- Success criteria may be too strict
- Available tools may not support this task

Solutions:
1. Simplify task scope
2. Relax success criteria
3. Add clarifying context
4. Provide more guidance

**Problem 2: Search advantage too small (<20% improvement)**
- Task doesn't leverage search strengths
- Grep is "good enough" for this task
- Search overhead negates benefits

Solutions:
1. Add transitive relationships (grep can't traverse)
2. Require pattern recognition across variations
3. Add conceptual ambiguity
4. Consider moving to easier tier

Example:
```typescript
// Failed discriminant validity (only 15% improvement)
{
  description: "Find files that import WorktreeManager",
  expectedGrepSuccess: 0.60,
  expectedSearchSuccess: 0.75  // Only 15% better
}

// Fixed (50% improvement)
{
  description: "Find code that would break if WorktreeManager API changes",
  expectedGrepSuccess: 0.25,
  expectedSearchSuccess: 0.75  // 50% better, clear advantage
}
```

### Dimension 3: Ecological Validity

**Question**: Does this task reflect real-world developer scenarios?

**What We're Testing**:
- Task is based on actual scenarios (code review, debugging, refactoring)
- Developers would recognize and perform this task
- Task frequency matches real-world occurrence
- Clear practical value

**Why It Matters**: Synthetic tasks might not generalize. If developers never do this task, benchmark scores don't predict real-world utility.

**How to Test**:

```typescript
const result = await validateTask({
  task: myTask,
  tier: 'tier1-impossible',
  useMockData: true
})

const ecologicalResult = result.dimensions.ecologicalValidity

if (ecologicalResult.passed) {
  console.log('✓ Task reflects realistic scenarios')
  console.log(`  Score: ${ecologicalResult.actual}`)
  console.log(`  ${ecologicalResult.details}`)
} else {
  console.log('✗ Task may not be realistic')
  console.log(`  ${ecologicalResult.details}`)
}
```

**Ecological Checklist**:

The validator checks these indicators:

```typescript
interface EcologicalChecks {
  // Realism
  basedOnRealScenario: boolean  // From task.basedOnRealScenario
  hasConcretePurpose: boolean   // Description explains "why"

  // Frequency
  frequency: 'daily' | 'weekly' | 'monthly' | 'rare'

  // Clarity
  objectiveSuccessCriteria: boolean
  noSubjectiveJudgment: boolean
  deterministicOutcome: boolean

  // Fairness
  noToolCoercion: boolean  // Doesn't force tool choice
  naturalLanguage: boolean  // Not artificially obscure
}
```

**Scoring**:
- Each check: 10-20 points
- Total score ≥ 60% to pass
- Real scenario + objective criteria + clear purpose = core requirements

**Pass Criteria**: 70%+ developer approval for realism (manual survey)

**Developer Survey Template**:

For critical tasks, run a developer survey:

```markdown
# Task Realism Survey: [Task Name]

**Task**: [Task description]

**Questions**:

1. Would you actually do this in real work?
   - [ ] Yes, frequently (weekly or more)
   - [ ] Yes, occasionally (monthly)
   - [ ] Yes, rarely (few times a year)
   - [ ] No, this is not a realistic task

2. How realistic is this scenario? (1-5)
   - 1 = Completely artificial
   - 3 = Somewhat realistic
   - 5 = Extremely realistic

3. Would semantic search help you with this task?
   - [ ] Yes, significantly
   - [ ] Yes, somewhat
   - [ ] No, grep is fine
   - [ ] No, I'd use a different approach

4. When would you do this task?
   - [ ] Code review
   - [ ] Debugging
   - [ ] Refactoring
   - [ ] Documentation
   - [ ] Security audit
   - [ ] Other: __________

**Results**: Need 70%+ "Yes, frequently/occasionally" on Q1
```

**Fixing Failed Ecological Validity**:

**Problem: Synthetic task (not based on real scenario)**

Solutions:
1. Link to actual PR, issue, or code review
2. Set `basedOnRealScenario: true`
3. Add context explaining when developers do this
4. Interview developers about real needs

**Problem: Artificially obscure language**

Solutions:
1. Use natural developer language
2. Avoid overly academic or formal phrasing
3. Test: "Would I say this to a colleague?"

**Problem: Rare edge case (frequency: rare)**

Solutions:
1. Find more common variant of the task
2. Generalize to broader use case
3. Consider dropping task if truly rare

Example:
```typescript
// Failed ecological validity (synthetic)
{
  name: "Find all functions with exactly 3 parameters using async/await",
  basedOnRealScenario: false,
  frequency: 'rare'
}
// Developers: "Why would I ever need this?"

// Fixed (realistic)
{
  name: "Find async operations without error handling",
  basedOnRealScenario: true,
  linkedScenario: "Issue #234 - Unhandled promise rejections",
  frequency: 'monthly'
}
// Developers: "Yes, I do this during security audits"
```

### Dimension 4: Test-Retest Reliability

**Question**: Do tasks produce consistent results across runs?

**What We're Testing**:
- Same task + same tools → same outcome (within variance)
- Results aren't dependent on random factors
- Objective criteria produce deterministic scores

**Why It Matters**: High variance means we can't trust results. If a task passes sometimes and fails sometimes (with same tools), we're measuring noise, not tool capability.

**How to Test**:

```typescript
// Mock mode (checks validator objectivity)
const result = await validateTask({
  task: myTask,
  tier: 'tier1-impossible',
  iterations: 5,
  useMockData: true
})

const reliabilityResult = result.dimensions.reliability

if (reliabilityResult.passed) {
  console.log('✓ Task has objective validator')
  console.log(`  ${reliabilityResult.details}`)
} else {
  console.log('✗ Task may have high variance')
  console.log(`  ${reliabilityResult.details}`)
}
```

```typescript
// Real mode (measures actual variance)
async function measureReliability(task: SearchTask, runs: number = 5) {
  const scores: number[] = []

  for (let i = 0; i < runs; i++) {
    const result = await runTask(task, { tools: ['grep', 'glob', 'read'] })
    scores.push(result.score)
  }

  // Calculate coefficient of variation (CV)
  const mean = scores.reduce((a, b) => a + b) / scores.length
  const variance = scores.map(s => (s - mean) ** 2).reduce((a, b) => a + b) / scores.length
  const stdDev = Math.sqrt(variance)
  const cv = stdDev / mean

  return {
    scores,
    mean,
    stdDev,
    cv,
    passed: cv < 0.10  // Less than 10% variation
  }
}

const reliability = await measureReliability(myTask, 5)
console.log(`Mean score: ${reliability.mean.toFixed(2)}`)
console.log(`Std dev: ${reliability.stdDev.toFixed(2)}`)
console.log(`CV: ${(reliability.cv * 100).toFixed(1)}%`)
console.log(`Reliable: ${reliability.passed ? 'YES' : 'NO'}`)
```

**Threshold**: Coefficient of variation (CV) < 10%

**Pass Criteria**: Variance < 10% for each task across 5 runs

**Understanding Reliability Indicators**:

**High reliability indicators**:
- ✓ Validator type: `code_change` or `file_creation` (most objective)
- ✓ Validator type: `explanation` with specific regex/file requirements
- ✓ Binary checks (file exists? pattern matches?)
- ✓ Clear, unambiguous success criteria

**Low reliability indicators**:
- ✗ Validator type: `explanation` with vague criteria
- ✗ Subjective judgments ("good", "thorough", "comprehensive")
- ✗ Scalar assessments without thresholds
- ✗ Multiple interpretation possibilities

**Fixing Failed Reliability**:

**Problem: High variance (>10% CV)**

Solutions:
1. Make validator more objective
2. Convert from explanation to code_change validator
3. Add specific file/pattern requirements
4. Remove subjective criteria

Example:
```typescript
// Failed reliability (subjective validator)
{
  followUpTask: {
    validator: {
      type: 'explanation',
      criteria: "Agent provides comprehensive explanation"
      // Too subjective, high variance expected
    }
  }
}

// Fixed (objective validator)
{
  followUpTask: {
    validator: {
      type: 'explanation',
      mentionsFiles: ['worktree-manager.ts', 'git-operations.ts'],
      mentionsPattern: /create.*worktree|initialize/i,
      minimumLength: 100
      // Objective, binary checks, low variance
    }
  }
}
```

### Dimension 5: Statistical Power

**Question**: Is the sample size adequate for detecting differences?

**What We're Testing**:
- Enough iterations to detect meaningful differences
- Statistical power to avoid false negatives
- Sample size supports valid inference

**Why It Matters**: With only 1-2 runs, we can't distinguish real differences from random variation. Need adequate sample size for statistical validity.

**How to Test**:

```typescript
const result = await validateTask({
  task: myTask,
  tier: 'tier1-impossible',
  iterations: 5,  // Minimum recommended
  useMockData: true
})

const powerResult = result.dimensions.statisticalPower

if (powerResult.passed) {
  console.log('✓ Adequate sample size')
  console.log(`  ${powerResult.details}`)
} else {
  console.log('✗ Sample size too small')
  console.log(`  ${powerResult.details}`)
}
```

**Minimum Sample Sizes**:

| Purpose | Minimum n | Recommended n |
|---------|-----------|---------------|
| Quick validation | 3 | 5 |
| Standard testing | 5 | 10 |
| Publication | 10 | 30 |

**Power Analysis**:

For detecting a medium effect size (d = 0.5) with 80% power:
- Minimum n = 5 per group (grep vs search)
- Recommended n = 10 per group for robust results
- Large studies: n = 30+ per group

**Pass Criteria**: n ≥ 5 for basic statistical power

**Practical Considerations**:

**Cost vs. Power Trade-off**:
- Each run costs API credits
- More runs = more confidence, higher cost
- Balance based on task importance

**When to use higher n**:
- Critical tasks (foundational to framework)
- Publication or external validation
- High variance tasks (need more samples)
- Cross-project validation

**When n = 5 is sufficient**:
- Initial task validation
- Low variance tasks
- Internal development
- Mock mode testing (free)

## Running Baseline Comparisons

Baseline comparison is the core validation procedure: run tasks with grep-only, then with search available, and compare results.

### Quick Validation (Mock Mode)

For rapid development, use mock mode:

```typescript
import { validateTask } from '../validation/task-validator.js'

// Fast validation using task's expectedGrepSuccess/expectedSearchSuccess
const result = await validateTask({
  task: myTask,
  tier: 'tier1-impossible',
  iterations: 5,
  useMockData: true  // No LLM execution, uses expected metrics
})

console.log('Validation result:', result.passed ? 'PASSED' : 'FAILED')
console.log('Recommendations:', result.recommendations)
```

**When to use mock mode**:
- CI/CD pipeline validation
- Rapid task development
- Checking task structure and metadata
- Before expensive LLM runs

### Full Validation (Real Mode)

For final validation, run actual benchmarks:

```typescript
import { runBaseline } from '../evaluation/baseline-runner.js'
import { runWithSearch } from '../evaluation/search-runner.js'

// Run with grep-only tools
const grepResult = await runBaseline({
  task: myTask,
  availableTools: ['Grep', 'Glob', 'Read', 'Bash'],
  timeout: 300,  // 5 minutes
  worktreePath: process.cwd()
})

// Run with search available
const searchResult = await runWithSearch({
  task: myTask,
  timeout: 300,
  worktreePath: process.cwd()
})

// Compare results
console.log('Grep success:', grepResult.success)
console.log('Grep time:', grepResult.metrics.durationSeconds)
console.log('Search success:', searchResult.success)
console.log('Search time:', searchResult.metrics.durationSeconds)

// Calculate advantage
const advantage = {
  successImprovement: searchResult.success ? 1 : 0 - grepResult.success ? 1 : 0,
  timeSaved: grepResult.metrics.durationSeconds - searchResult.metrics.durationSeconds
}

console.log('Advantage:', advantage)
```

**When to use real mode**:
- Final task validation before adding to suite
- Cross-project validation
- Publication or external reporting
- Calibrating expectedGrepSuccess/expectedSearchSuccess

### Running Multiple Iterations

For statistical significance:

```typescript
async function runMultipleIterations(task: SearchTask, n: number = 5) {
  console.log(`Running ${n} iterations for statistical power...`)

  // Grep-only runs
  const grepResults = []
  for (let i = 0; i < n; i++) {
    console.log(`  Grep iteration ${i + 1}/${n}`)
    const result = await runBaseline({ task, timeout: 300 })
    grepResults.push(result)
  }

  // Search available runs
  const searchResults = []
  for (let i = 0; i < n; i++) {
    console.log(`  Search iteration ${i + 1}/${n}`)
    const result = await runWithSearch({ task, timeout: 300 })
    searchResults.push(result)
  }

  // Aggregate metrics
  const grepScores = grepResults.map(r => r.success ? 1 : 0)
  const searchScores = searchResults.map(r => r.success ? 1 : 0)

  const grepSuccessRate = grepScores.reduce((a, b) => a + b) / n
  const searchSuccessRate = searchScores.reduce((a, b) => a + b) / n

  return {
    grep: {
      successRate: grepSuccessRate,
      results: grepResults
    },
    search: {
      successRate: searchSuccessRate,
      results: searchResults
    },
    advantage: searchSuccessRate - grepSuccessRate
  }
}

const comparison = await runMultipleIterations(myTask, 5)
console.log('Grep success rate:', comparison.grep.successRate)
console.log('Search success rate:', comparison.search.successRate)
console.log('Improvement:', comparison.advantage)
```

## Interpreting Statistical Results

### Understanding p-values

**What p-value means**:
- p < 0.05: Less than 5% chance this difference is random
- p < 0.01: Less than 1% chance (stronger evidence)
- p ≥ 0.05: Cannot conclude difference is real

**Interpreting results**:

```typescript
import { tTest } from '../reporting/statistics.js'

const test = tTest(grepScores, searchScores)

if (test.p < 0.01) {
  console.log('✓✓ Very strong evidence of difference')
  console.log(`   p = ${test.p.toFixed(4)}`)
} else if (test.p < 0.05) {
  console.log('✓ Strong evidence of difference')
  console.log(`   p = ${test.p.toFixed(4)}`)
} else if (test.p < 0.10) {
  console.log('~ Weak evidence (marginally significant)')
  console.log(`   p = ${test.p.toFixed(4)}`)
} else {
  console.log('✗ No statistical evidence of difference')
  console.log(`   p = ${test.p.toFixed(4)}`)
  console.log('   Need more samples or difference is too small')
}
```

### Understanding Effect Sizes

**Cohen's d** measures the magnitude of difference:

| Effect Size (d) | Interpretation |
|----------------|----------------|
| d < 0.2 | Negligible |
| 0.2 ≤ d < 0.5 | Small |
| 0.5 ≤ d < 0.8 | Medium |
| d ≥ 0.8 | Large |

```typescript
const test = tTest(grepScores, searchScores)

console.log(`Effect size (Cohen's d): ${test.effectSize.toFixed(2)}`)

if (test.effectSize >= 0.8) {
  console.log('  → Large effect (clear practical difference)')
} else if (test.effectSize >= 0.5) {
  console.log('  → Medium effect (noticeable difference)')
} else if (test.effectSize >= 0.2) {
  console.log('  → Small effect (subtle difference)')
} else {
  console.log('  → Negligible effect (not meaningful)')
}
```

**What we want**:
- p < 0.05 (statistical significance)
- d ≥ 0.5 (medium to large effect size)
- Both conditions = strong, meaningful difference

## Troubleshooting Guide by Failure Type

### Failure Type 1: Task Too Easy for Grep

**Symptom**: Grep success rate >60% (for Tier 1) or >80% (for Tier 2)

**Root Causes**:
- Task description contains obvious keywords
- Task requires only direct file finding
- Simple string matching solves it

**Diagnostic Questions**:
1. Can you solve this with one grep command?
2. Does the description contain the function/class name?
3. Is this asking for direct matches rather than relationships?

**Fixes**:
1. **Apply anti-keyword pattern** - Remove obvious keywords
2. **Add indirection** - Require transitive relationships
3. **Increase conceptual complexity** - Ask about concepts, not names
4. **Consider tier change** - Maybe this is Tier 2 or 3, not Tier 1

**Example fix flow**:
```typescript
// Original (grep succeeds 75%)
"Find the createWorktree function in WorktreeManager"

// Attempt 1: Remove direct names (grep succeeds 50%)
"Find the function that creates new Git worktrees"

// Attempt 2: Add indirection (grep succeeds 25%)
"Find code that would break if we change the worktree creation API"

// Success: Grep <30%, search >70%
```

### Failure Type 2: Task Too Hard for Everyone

**Symptom**: Both grep AND search success <40%

**Root Causes**:
- Requires knowledge outside codebase
- Success criteria too strict
- Task scope too broad
- Ambiguous definition of success

**Diagnostic Questions**:
1. Can a human solve this without external knowledge?
2. Are success criteria realistic?
3. Is the task scope appropriate?
4. Is "success" well-defined?

**Fixes**:
1. **Narrow scope** - Focus on specific subsystem
2. **Add context** - Provide necessary background
3. **Relax criteria** - Accept partial success
4. **Split task** - Break into smaller subtasks
5. **Clarify success** - Make expectations concrete

**Example fix flow**:
```typescript
// Original (too hard, both fail)
"Find all security vulnerabilities in the authentication system"

// Attempt 1: Narrow scope (still too vague)
"Find security issues in JWT validation"

// Attempt 2: Specific pattern (achievable)
"Find authentication checks that don't validate token expiration"

// Success: Clear, specific, achievable
```

### Failure Type 3: Insufficient Search Advantage

**Symptom**: Search only 10-20% better than grep (need 30%+ for Tier 1)

**Root Causes**:
- Task doesn't leverage semantic search strengths
- Grep is actually "good enough"
- Search overhead negates benefits

**Diagnostic Questions**:
1. Does this task require semantic understanding?
2. Would relationships/concepts help here?
3. Is grep struggling with this?

**Fixes**:
1. **Add transitive relationships** - Grep can't traverse code graphs
2. **Require pattern recognition** - Same concept, different implementations
3. **Add semantic ambiguity** - Context-dependent meaning
4. **Move to appropriate tier** - Maybe this is Tier 2 or 3

**Example fix flow**:
```typescript
// Original (search only 15% better)
"Find files that import WorktreeManager"
// Grep: 60% success, Search: 75% success (only 15pp improvement)

// Attempt 1: Add relationships (search 30% better)
"Find code that uses WorktreeManager, directly or indirectly"
// Grep: 40% success, Search: 70% success (30pp improvement)

// Attempt 2: Require understanding (search 50% better)
"Find code that would break if WorktreeManager API changes"
// Grep: 25% success, Search: 75% success (50pp improvement)

// Success: Clear search advantage demonstrated
```

### Failure Type 4: Unreliable Results (High Variance)

**Symptom**: CV >10%, results inconsistent across runs

**Root Causes**:
- Subjective success criteria
- Vague validation requirements
- Multiple interpretations possible

**Diagnostic Questions**:
1. Is success criteria binary or subjective?
2. Can a machine check success automatically?
3. Are there multiple ways to "succeed"?

**Fixes**:
1. **Make criteria objective** - Binary checks, not judgments
2. **Use code_change validator** - Most objective type
3. **Specify requirements** - Explicit file/pattern mentions
4. **Remove ambiguity** - One clear path to success

**Example fix flow**:
```typescript
// Original (high variance, CV = 18%)
validator: {
  type: 'explanation',
  criteria: "Agent provides good explanation of retry logic"
}

// Attempt 1: Add some structure (CV = 12%)
validator: {
  type: 'explanation',
  criteria: "Explains retry logic and mentions relevant files"
}

// Attempt 2: Specific requirements (CV = 7%)
validator: {
  type: 'explanation',
  mentionsFiles: ['message-bus.ts'],
  mentionsPattern: /(retry|attempt|backoff).*mechanism/i,
  minimumLength: 100
}

// Success: Low variance, reproducible results
```

### Failure Type 5: Ecologically Invalid

**Symptom**: Developers say "I would never do this task"

**Root Causes**:
- Synthetic task, not based on real scenarios
- Artificially constructed to be hard
- Rare edge case, not common developer need
- No clear practical value

**Diagnostic Questions**:
1. Have you actually done this task?
2. When would a developer need to do this?
3. How often does this come up?
4. What's the practical benefit?

**Fixes**:
1. **Base on real scenario** - Link to PR/issue/discussion
2. **Add context** - Explain when developers do this
3. **Find common variant** - Generalize to more frequent case
4. **Interview developers** - Validate with actual users

**Example fix flow**:
```typescript
// Original (developers: "Why?")
{
  name: "Find all functions with exactly 3 parameters that use async/await",
  basedOnRealScenario: false
}

// Attempt 1: Still artificial
{
  name: "Find all async functions without error handling",
  basedOnRealScenario: false
}

// Attempt 2: Real-world grounding
{
  name: "Find async operations that don't have error handling",
  basedOnRealScenario: true,
  linkedScenario: "Issue #234 - Unhandled promise rejections causing crashes",
  context: "During security audit, need to find error handling gaps"
}

// Success: Developers recognize this as real work
```

## Validation Workflow Example

Complete validation workflow for a new task:

```typescript
import { validateTask, formatValidationReport } from '../validation/task-validator.js'
import { TASK_NEW } from '../tasks/my-category/new-task.js'

async function validateNewTask() {
  console.log('Validating new task...\n')

  // Step 1: Quick validation (mock mode)
  const mockResult = await validateTask({
    task: TASK_NEW,
    tier: 'tier1-impossible',
    iterations: 5,
    useMockData: true  // Fast, no API calls
  })

  console.log(formatValidationReport(mockResult))

  if (!mockResult.passed) {
    console.log('\n❌ Task failed mock validation')
    console.log('Fix these issues before running expensive real validation:\n')
    mockResult.recommendations.forEach((rec, i) => {
      console.log(`${i + 1}. ${rec}`)
    })
    return
  }

  console.log('\n✅ Task passed mock validation')
  console.log('Proceeding to real validation (this will cost API credits)...\n')

  // Step 2: Real validation with actual LLM execution
  // (Only run after mock validation passes)
  const realResult = await validateTask({
    task: TASK_NEW,
    tier: 'tier1-impossible',
    iterations: 5,
    useMockData: false  // Expensive, runs actual agents
  })

  console.log(formatValidationReport(realResult))

  if (realResult.passed) {
    console.log('\n✅✅ Task passed all validation!')
    console.log('Ready to add to benchmark suite')
  } else {
    console.log('\n❌ Task failed real validation')
    console.log('Recommendations:\n')
    realResult.recommendations.forEach((rec, i) => {
      console.log(`${i + 1}. ${rec}`)
    })
  }
}

validateNewTask()
```

## Next Steps

- **To design tasks**: See [Task Design Guide](./task-design-guide.md)
- **To run benchmarks**: See [Benchmark Usage Guide](./benchmark-usage.md)
- **For quality strategy details**: See [.agents/projects/TESTDES_grep-impossible-task-design/planning/quality-strategy.md](../../.agents/projects/TESTDES_grep-impossible-task-design/planning/quality-strategy.md)

## Validation Checklist

Before declaring a task validated:

- [ ] Construct validity: Grep baseline appropriate for tier
- [ ] Discriminant validity: Search advantage meets threshold
- [ ] Ecological validity: Based on real scenario, realistic frequency
- [ ] Reliability: Objective criteria, low expected variance
- [ ] Statistical power: Adequate sample size (n ≥ 5)
- [ ] Mock validation passed
- [ ] Real validation passed (for production tasks)
- [ ] All recommendations addressed
- [ ] Task added to appropriate suite
- [ ] Documentation updated

**Remember**: Validation is iterative. Use failures to improve tasks, not to discard them. The goal is high-quality tasks that prove semantic search value.

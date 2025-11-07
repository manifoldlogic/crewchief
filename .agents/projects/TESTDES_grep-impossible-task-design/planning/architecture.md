# Architecture: Grep-Impossible Task Design Framework

## Design Philosophy

Build a systematic framework for creating, categorizing, and validating search tasks that prove semantic code search provides measurable value without coercing agents to use it. The architecture must support:

1. **Natural Tool Selection**: Agents choose tools based on task characteristics, not prompts
2. **Objective Measurement**: Clear success criteria, no subjective judgment
3. **Ecological Validity**: Tasks reflect real developer workflows
4. **Systematic Improvement**: Learn from failures, iterate task design

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Task Design Framework                     │
└─────────────────────────────────────────────────────────────┘
                              │
              ┌───────────────┴───────────────┐
              │                               │
     ┌────────▼────────┐            ┌────────▼────────┐
     │  Task Taxonomy   │            │  Task Generator  │
     │                  │            │                  │
     │  - Categories    │            │  - Templates     │
     │  - Patterns      │            │  - Variations    │
     │  - Difficulty    │            │  - Validation    │
     └────────┬────────┘            └────────┬────────┘
              │                               │
              └───────────────┬───────────────┘
                              │
                    ┌─────────▼─────────┐
                    │  Task Evaluation   │
                    │                    │
                    │  - Execution       │
                    │  - Measurement     │
                    │  - Analysis        │
                    └─────────┬─────────┘
                              │
              ┌───────────────┴───────────────┐
              │                               │
     ┌────────▼────────┐            ┌────────▼────────┐
     │ Baseline (Grep)  │            │  Semantic Search │
     │                  │            │                  │
     │  - Control       │            │  - Treatment     │
     │  - Reference     │            │  - Comparison    │
     └─────────────────┘            └─────────────────┘
```

## Core Components

### 1. Task Taxonomy

**Purpose**: Categorize tasks by characteristics that predict tool performance

**Structure**:
```typescript
interface TaskCategory {
  name: string
  description: string
  grepDifficulty: 'impossible' | 'hard' | 'possible' | 'easy'
  searchAdvantage: 'critical' | 'significant' | 'moderate' | 'none'
  realWorldFrequency: 'common' | 'occasional' | 'rare'
  exampleScenarios: string[]
}
```

**Categories**:

#### Category 1: Relationship Discovery
**Grep Difficulty**: Impossible
**Search Advantage**: Critical
**Examples**:
- "What code depends on X without importing it directly?"
- "Find all callers of this function through indirection"
- "What would break if we change this API?"

**Why Grep Fails**: Requires code graph traversal, not string matching

#### Category 2: Conceptual Similarity
**Grep Difficulty**: Hard
**Search Advantage**: Significant
**Examples**:
- "Find all retry implementations across the codebase"
- "Locate error handling patterns similar to this one"
- "Where else do we do rate limiting?"

**Why Grep Struggles**: Different implementations use different terminology

#### Category 3: Ambiguity Resolution
**Grep Difficulty**: Hard
**Search Advantage**: Significant
**Examples**:
- "Where are database transactions managed?" (ORM vs manual vs decorators)
- "Find authentication logic" (middleware vs decorators vs manual checks)
- "How do we handle caching?" (Redis vs in-memory vs CDN)

**Why Grep Struggles**: Multiple implementation patterns, can't disambiguate

#### Category 4: Negative Space
**Grep Difficulty**: Impossible
**Search Advantage**: Critical
**Examples**:
- "Find code that modifies state without persistence"
- "Where do we call APIs without retry logic?"
- "What event handlers don't have error boundaries?"

**Why Grep Fails**: Searching for absence requires understanding presence first

#### Category 5: Cross-Cutting Concerns
**Grep Difficulty**: Hard
**Search Advantage**: Moderate
**Examples**:
- "Find all error handling in async operations"
- "Where do we log security events?"
- "How is authentication checked across different endpoints?"

**Why Grep Struggles**: Scattered across codebase, context-dependent

#### Category 6: Architectural Understanding
**Grep Difficulty**: Impossible
**Search Advantage**: Critical
**Examples**:
- "How does data flow from API to database?"
- "What's the initialization sequence for the application?"
- "How are background jobs scheduled and executed?"

**Why Grep Fails**: Requires understanding system-level interactions

### 2. Task Generator

**Purpose**: Systematically create task variations based on templates

**Template Structure**:
```typescript
interface TaskTemplate {
  category: TaskCategory
  pattern: string  // e.g., "Find {concept} implementations in {scope}"
  requiredFeatures: string[]  // e.g., ["code graph", "semantic similarity"]
  avoidKeywords: boolean  // Don't use obvious searchable terms

  variants: {
    easy: TaskVariant    // Some keywords present
    medium: TaskVariant  // Conceptual description only
    hard: TaskVariant    // Requires deep understanding
  }
}
```

**Generation Strategy**:

1. **Start with Real Scenarios**
   - Mine from actual code reviews
   - Extract from debugging sessions
   - Based on refactoring needs

2. **Apply Anti-Keyword Pattern**
   - Replace direct terms with concepts
   - Example: "Find retry logic" → "Find code that re-attempts failed operations"

3. **Create Variations**
   - Easy: Some keyword hints
   - Medium: Pure conceptual
   - Hard: Requires inference

4. **Validate Grep-Impossibility**
   - Run grep baseline
   - Measure success rate
   - If grep succeeds >30%, task is too easy

**Example Task Generation**:

```typescript
// Template: Find circular dependency detection
const template = {
  category: "relationship-discovery",
  pattern: "Find {mechanism} that detects {problem}",

  variants: {
    easy: {
      description: "Find code that checks for circular dependencies in the module graph",
      keywords: ["circular", "dependency", "cycle"],
      expectedGrepSuccess: 60%
    },

    medium: {
      description: "Find the mechanism that prevents infinite loops in module loading",
      keywords: ["loop", "module"],  // Less direct
      expectedGrepSuccess: 30%
    },

    hard: {
      description: "How does the system ensure modules don't create reference cycles?",
      keywords: [],  // Pure conceptual
      expectedGrepSuccess: 10%
    }
  }
}
```

### 3. Task Evaluation Framework

**Purpose**: Execute tasks, measure performance, analyze failures

**Execution Flow**:
```
1. Setup
   ├─ Create fresh agent session
   ├─ Configure tool availability (Grep-only vs Full)
   └─ Set task context

2. Execute
   ├─ Present task to agent
   ├─ Log all tool calls
   ├─ Capture reasoning
   └─ Measure time

3. Evaluate
   ├─ Check success criteria
   ├─ Calculate metrics
   ├─ Compare to baseline
   └─ Categorize failures

4. Analyze
   ├─ Why did agent choose each tool?
   ├─ What led to success/failure?
   ├─ How can we improve task design?
   └─ Update taxonomy
```

**Measurement Framework**:

```typescript
interface TaskMetrics {
  // Success Metrics
  taskCompleted: boolean
  correctFileFound: boolean
  correctFunctionIdentified: boolean
  explanationAccurate: boolean

  // Efficiency Metrics
  timeToSuccess: number  // seconds
  toolCallCount: number
  searchQueryCount: number
  grepQueryCount: number

  // Quality Metrics
  precision: number  // relevant results / total results
  recall: number     // relevant results found / all relevant
  firstResultRank: number  // position of first correct result

  // Behavioral Metrics
  toolSequence: string[]  // e.g., ["grep", "glob", "read", "search"]
  queryRefinements: number
  deadEnds: number  // searches that led nowhere
}
```

**Comparison Framework**:

```typescript
interface ComparisonResult {
  task: SearchTask

  grepOnly: {
    metrics: TaskMetrics
    outcome: 'success' | 'failure' | 'partial'
    transcript: string
  }

  searchAvailable: {
    metrics: TaskMetrics
    outcome: 'success' | 'failure' | 'partial'
    usedSearch: boolean
    transcript: string
  }

  advantage: {
    timeSaved: number  // seconds
    qualityImprovement: number  // 0-1 scale
    toolSelectionCorrect: boolean
    significantDifference: boolean  // p < 0.05
  }
}
```

### 4. Validation Pipeline

**Purpose**: Ensure tasks are well-designed and ecologically valid

**Validation Steps**:

#### Step 1: Grep Baseline
```typescript
// Run task with ONLY Grep/Glob/Read available
const grepBaseline = await runTask(task, {
  availableTools: ['grep', 'glob', 'read', 'bash']
})

// Task difficulty calibration
if (grepBaseline.success > 0.7) {
  status = 'too-easy'  // Grep works fine, not testing anything
} else if (grepBaseline.success < 0.1) {
  status = 'too-hard'  // Even semantic search might struggle
} else {
  status = 'appropriate'  // Sweet spot: grep struggles
}
```

#### Step 2: Search Performance
```typescript
// Run task WITH semantic search available
const searchAvailable = await runTask(task, {
  availableTools: ['grep', 'glob', 'read', 'bash', 'mcp__maproom__search']
})

// Calculate advantage
const advantage = {
  successDelta: searchAvailable.success - grepBaseline.success,
  timeDelta: grepBaseline.time - searchAvailable.time,
  toolUsed: searchAvailable.usedSearch
}

// Require significant improvement
if (advantage.successDelta < 0.3 && advantage.timeDelta < 30) {
  status = 'insufficient-advantage'  // Search doesn't help enough
}
```

#### Step 3: Ecological Validation
```typescript
interface EcologicalChecks {
  // Realism
  basedOnRealScenario: boolean
  developersWouldDoThis: boolean
  frequencyInRealWork: 'daily' | 'weekly' | 'monthly' | 'rare'

  // Clarity
  objectiveSuccessCriteria: boolean
  noSubjectiveJudgment: boolean
  deterministicOutcome: boolean

  // Fairness
  noCoercion: boolean  // Task doesn't force tool choice
  multipleValidApproaches: boolean
  clearerWithoutToolHint: boolean
}
```

#### Step 4: Cross-Project Validation
```typescript
// Test on multiple codebases
const codebases = ['crewchief', 'another-ts-project', 'python-project']

for (const codebase of codebases) {
  const result = await runTask(adaptTaskForCodebase(task, codebase))
  results.push(result)
}

// Require generalization
const successRate = results.filter(r => r.success).length / results.length
if (successRate < 0.6) {
  status = 'too-specific'  // Only works on one codebase
}
```

## Task Design Patterns

### Pattern 1: Transitive Relationship Query

**Template**: "Find X that affects Y indirectly"

**Implementation**:
```typescript
{
  description: "Find code that could break if we change the worktree creation API",

  grepApproach: "Search for 'createWorktree', find direct callers, manually check each",
  grepDifficulty: "high",

  searchApproach: "Query: 'worktree creation callers dependencies'",
  searchAdvantage: "Finds transitive dependencies via code graph",

  successCriteria: {
    foundDirectCallers: true,
    foundIndirectDependents: true,
    identifiedBreakageRisk: true
  }
}
```

### Pattern 2: Conceptual Pattern Match

**Template**: "Find all implementations of {concept} across codebase"

**Implementation**:
```typescript
{
  description: "Find all places where we retry failed operations",

  grepApproach: "Search for 'retry', might miss exponential backoff, circuit breakers",
  grepDifficulty: "medium",

  searchApproach: "Query: 'retry failed operation implementation'",
  searchAdvantage: "Finds conceptually similar patterns with different naming",

  successCriteria: {
    foundRetryDecorators: true,
    foundExponentialBackoff: true,
    foundCircuitBreakers: true,
    foundManualLoops: true
  }
}
```

### Pattern 3: Architectural Flow Trace

**Template**: "How does {data/control} flow through the system?"

**Implementation**:
```typescript
{
  description: "How does a git worktree request flow from CLI to creation?",

  grepApproach: "Search for entry point, manually follow calls, easy to miss steps",
  grepDifficulty: "hard",

  searchApproach: "Query: 'worktree creation workflow', use context bundles",
  searchAdvantage: "Assembles complete call chain with context",

  successCriteria: {
    identifiedEntryPoint: true,
    tracedCommandParsing: true,
    foundValidation: true,
    locatedGitExecution: true,
    completedEndToEnd: true
  }
}
```

### Pattern 4: Negative Constraint

**Template**: "Find X that lacks Y (where Y is expected)"

**Implementation**:
```typescript
{
  description: "Find API endpoints that don't have rate limiting",

  grepApproach: "Find all endpoints, find rate limit logic, manually diff",
  grepDifficulty: "impossible",

  searchApproach: "Query both, use code graph to find unprotected endpoints",
  searchAdvantage: "Can reason about absence via relationship graph",

  successCriteria: {
    foundAllEndpoints: true,
    identifiedProtected: true,
    identifiedUnprotected: true,
    noFalsePositives: true
  }
}
```

### Pattern 5: Multi-Pattern Aggregation

**Template**: "Find all implementations of {concept} where implementations vary widely"

**Implementation**:
```typescript
{
  description: "Where do we perform authentication checks?",

  grepApproach: "Search 'auth', 'authenticate', 'isLoggedIn', etc - miss many",
  grepDifficulty: "hard",

  searchApproach: "Query: 'authentication verification', finds all patterns",
  searchAdvantage: "Understands concept across different implementations",

  successCriteria: {
    foundMiddleware: true,
    foundDecorators: true,
    foundManualChecks: true,
    foundJWTValidation: true,
    foundSessionChecks: true
  }
}
```

## Data Model

### Task Definition
```typescript
interface SearchTask {
  // Identity
  id: string
  name: string
  category: TaskCategory
  difficulty: 'easy' | 'medium' | 'hard'

  // Description
  description: string  // What agent receives
  internalNotes: string  // Why this task, what it tests

  // Search target
  searchTarget: SearchTarget

  // Follow-up work
  followUpTask?: {
    type: 'explanation' | 'code_change' | 'file_creation'
    prompt: string
    validator: TaskValidator
  }

  // Constraints
  maxSearchAttempts: number
  maxTimeSeconds: number

  // Validation
  successValidator: (result: AgentOutput) => TaskScore

  // Metadata
  basedOnRealScenario?: string  // Link to actual PR/issue/question
  expectedGrepSuccess: number  // 0-1
  expectedSearchSuccess: number  // 0-1
  validatedOn?: string[]  // Codebases tested on
}
```

### Evaluation Result
```typescript
interface EvaluationResult {
  task: SearchTask
  agent: string
  timestamp: Date

  // Execution
  toolsAvailable: string[]
  transcript: string
  duration: number

  // Metrics
  metrics: TaskMetrics
  score: TaskScore

  // Analysis
  toolSelection: {
    firstTool: string
    toolSequence: string[]
    searchUsed: boolean
    grepUsed: boolean
    reasoning: string
  }

  failure: {
    failed: boolean
    reason?: string
    category?: FailureCategory
  }
}
```

### Benchmark Suite
```typescript
interface BenchmarkSuite {
  name: string
  version: string
  tasks: SearchTask[]

  // Organization
  byCategory: Map<string, SearchTask[]>
  byDifficulty: Map<string, SearchTask[]>

  // Validation status
  validated: {
    grepBaselineRun: boolean
    searchPerformanceRun: boolean
    ecologicalReview: boolean
    crossProjectTested: boolean
  }

  // Results
  results?: BenchmarkResults
}
```

## Integration Points

### With Existing System

```typescript
// Extend current competition framework
interface CompetitionConfig {
  task: SearchTask  // Use new task format
  variants: Variant[]  // Tool description variants

  // NEW: Baseline comparison
  runGrepBaseline: boolean
  requireSearchAdvantage: number  // Minimum improvement required

  // NEW: Ecological validation
  validateRealism: boolean
  crossProjectTest?: string[]  // Additional codebases
}
```

### With Genetic Iterator

```typescript
// Evolution now optimizes for correct tool selection
interface GenerationEvaluation {
  // OLD: Just score
  score: number

  // NEW: Behavioral metrics
  toolSelectionCorrect: boolean
  searchUsedWhenAppropriate: boolean
  grepUsedWhenAppropriate: boolean

  // NEW: Multi-tier scoring
  tier1GrepImpossible: number  // Must use search
  tier2GrepHard: number  // Search faster
  tier3RealWorld: number  // Natural selection
}
```

## File Structure

```
packages/cli/src/search-optimization/
├── tasks/
│   ├── relationship-discovery/
│   │   ├── transitive-dependencies.ts
│   │   ├── call-chain-analysis.ts
│   │   └── impact-analysis.ts
│   ├── conceptual-similarity/
│   │   ├── pattern-matching.ts
│   │   ├── similar-implementations.ts
│   │   └── cross-cutting-concerns.ts
│   ├── architectural-understanding/
│   │   ├── data-flow.ts
│   │   ├── initialization-sequence.ts
│   │   └── system-interactions.ts
│   └── index.ts  // Export task registry
│
├── taxonomy/
│   ├── categories.ts  // Task category definitions
│   ├── patterns.ts  // Task design patterns
│   └── difficulty.ts  // Difficulty calibration
│
├── evaluation/
│   ├── baseline-runner.ts  // Grep-only execution
│   ├── comparison.ts  // Side-by-side comparison
│   ├── metrics.ts  // Metric calculation
│   └── analysis.ts  // Failure analysis
│
├── validation/
│   ├── grep-baseline.ts  // Ensure grep struggles
│   ├── search-performance.ts  // Ensure search helps
│   ├── ecological.ts  // Realism checks
│   └── cross-project.ts  // Generalization
│
└── benchmarks/
    ├── tier1-impossible.ts  // Grep can't solve
    ├── tier2-hard.ts  // Grep inefficient
    └── tier3-realworld.ts  // Natural scenarios
```

## Design Decisions

### Decision 1: No Task Coercion
**Problem**: Could hint "use semantic search" in task description
**Choice**: Tasks must be tool-agnostic, let agent choose
**Rationale**: Proves organic utility, not forced adoption

### Decision 2: Objective Success Criteria
**Problem**: "Good explanation" is subjective
**Choice**: Binary checks (found correct file, mentioned key function, etc)
**Rationale**: Reproducible, automatable, clear

### Decision 3: Three-Tier Framework
**Problem**: Single metric (score) hides complexity
**Choice**: Tier 1 (capability), Tier 2 (efficiency), Tier 3 (utility)
**Rationale**: Proves value at different levels

### Decision 4: Grep Baseline Required
**Problem**: Can't attribute success to search vs agent capability
**Choice**: Every task has grep-only control run
**Rationale**: Scientific validity, clear attribution

### Decision 5: Real-World Grounding
**Problem**: Synthetic tasks might not generalize
**Choice**: Base tasks on actual code review, debugging, refactoring
**Rationale**: Ecological validity, practical relevance

## Success Metrics

Framework is successful when:

1. **Task Quality**
   - 80%+ of Tier 1 tasks have grep success < 30%
   - 70%+ of Tier 2 tasks show >30% time savings with search
   - 60%+ of Tier 3 tasks result in voluntary search adoption

2. **Measurement Validity**
   - Success criteria are objective (no human judgment needed)
   - Results are reproducible (same task → same outcome)
   - Cross-project generalization >60%

3. **Ecological Validity**
   - Developer survey: 70%+ say "I would actually do this task"
   - Tasks based on real scenarios (not synthetic)
   - Tool selection is natural (no coercion)

4. **System Integration**
   - Works with existing competition framework
   - Supports genetic optimization
   - Enables systematic improvement

This architecture enables rigorous, scientific validation of semantic search value while maintaining practical relevance and avoiding artificial constraints on agent behavior.

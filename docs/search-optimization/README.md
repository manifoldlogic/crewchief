# Search Optimization Framework Documentation

This directory contains comprehensive documentation for the grep-impossible task design framework—a rigorous, scientific approach to evaluating semantic code search through benchmarks.

## Quick Links

- **[Competition Framework Guide](./competition-framework.md)** - How to run agent competitions to optimize tool descriptions
- **[Task Design Guide](./task-design-guide.md)** - How to create high-quality grep-impossible tasks
- **[Validation Guide](./validation-guide.md)** - How to validate task quality across 5 dimensions
- **[Benchmark Usage Guide](./benchmark-usage.md)** - How to run benchmarks and interpret results

## What is the Grep-Impossible Framework?

The grep-impossible task framework is a systematic approach to:

1. **Creating Search Tasks**: Design tasks that prove semantic search provides measurable value
2. **Validating Quality**: Ensure tasks measure what they claim across 5 quality dimensions
3. **Running Benchmarks**: Execute three-tier benchmark suites with statistical rigor
4. **Optimizing Tools**: Integrate with genetic optimization to evolve tool descriptions

### The Three-Tier Framework

**Tier 1: Grep-Impossible Tasks** (Capability)
- Tasks that fundamentally defeat grep
- Require: Code graph traversal, architectural understanding, negative space detection
- Examples: Transitive dependencies, data flow tracing, finding missing patterns

**Tier 2: Grep-Hard Tasks** (Efficiency)
- Tasks where grep might succeed but is inefficient
- Require: Conceptual similarity, ambiguity resolution, cross-cutting concerns
- Examples: Retry patterns, authentication checks, scattered functionality

**Tier 3: Real-World Tasks** (Utility)
- Natural developer scenarios without artificial constraints
- Focus: Realistic workflows, voluntary tool adoption
- Examples: Code review, debugging, refactoring tasks

## Documentation Structure

### For Task Creators

Start here if you want to create new search tasks:

1. **[Task Design Guide](./task-design-guide.md)**
   - Six task categories explained
   - Anti-keyword pattern tutorial
   - Five task design patterns
   - Creating objective success criteria
   - Common pitfalls and fixes
   - Complete code examples

**Topics Covered**:
- Relationship Discovery tasks
- Conceptual Similarity tasks
- Ambiguity Resolution tasks
- Negative Space tasks
- Cross-Cutting Concerns tasks
- Architectural Understanding tasks
- Validation checklist
- Task creation workflow

### For Quality Assurance

Start here if you want to validate task quality:

2. **[Validation Guide](./validation-guide.md)**
   - Five quality dimensions explained
   - Running baseline comparisons
   - Interpreting statistical results
   - Troubleshooting by failure type
   - Mock vs real validation modes
   - Ecological validity surveys

**Topics Covered**:
- Construct Validity (grep baseline)
- Discriminant Validity (search advantage)
- Ecological Validity (realism)
- Test-Retest Reliability (consistency)
- Statistical Power (sample size)
- Validation workflow examples
- Fixing common validation failures

### For Competition Runners

Start here if you want to run agent competitions:

3. **[Competition Framework Guide](./competition-framework.md)**
   - Setting up the environment
   - Running agent competitions
   - Understanding competition results
   - Genetic iteration for optimization
   - Cost management and budgeting
   - Troubleshooting common issues

**Topics Covered**:
- Environment variable setup (ANTHROPIC_API_KEY, MAPROOM_DATABASE_URL)
- Competition configuration and execution
- Variant creation and testing
- Multi-generation genetic optimization
- Statistical validation
- Cost estimation and savings
- Best practices

### For Benchmark Users

Start here if you want to run benchmarks:

4. **[Benchmark Usage Guide](./benchmark-usage.md)**
   - Running individual tasks
   - Running full validation suites
   - Reading and interpreting reports
   - Integration with genetic optimizer
   - Cost considerations
   - Cross-project validation

**Topics Covered**:
- Three-tier suite execution
- Grep vs search comparisons
- Statistical analysis
- Multi-tier scoring for optimization
- Tool selection tracking
- Cost estimation and savings
- Best practices
- Command-line interface

## Key Concepts

### Anti-Keyword Pattern

A technique for making tasks grep-resistant without being artificially obscure:

**Bad** (keyword-heavy): "Find the retry logic with exponential backoff"
**Good** (conceptual): "Find code that re-attempts failed operations with increasing delays"

The anti-keyword pattern ensures tasks require semantic understanding, not just string matching.

### Five Quality Dimensions

Every task is validated across five dimensions:

1. **Construct Validity**: Does grep fail as expected?
2. **Discriminant Validity**: Does search provide significant advantage?
3. **Ecological Validity**: Is this a realistic developer task?
4. **Test-Retest Reliability**: Are results consistent?
5. **Statistical Power**: Is sample size adequate?

### Objective Success Criteria

Success must be measurable without human judgment:

**Bad** (subjective): "Agent provides good explanation"
**Good** (objective): "Mentions files X, Y and pattern /retry.*mechanism/i"

Prefer code changes > explanations, binary checks > scalar judgments.

## Getting Started

### Quick Start: Create a Task

```typescript
import type { SearchTask } from '../types.js'
import { createTaskValidator } from '../validators.js'

export const MY_TASK: SearchTask = {
  id: 'tier1-my-task',
  name: 'My Grep-Impossible Task',
  category: 'relationship-discovery',
  difficulty: 'hard',

  description:
    'Find code that would break if we change the API...',

  searchTarget: {
    type: 'pattern',
    pattern: /dependency|import|usage/i,
  },

  followUpTask: {
    type: 'explanation',
    prompt: 'Explain what code depends on this API...',
    validator: {
      type: 'explanation',
      mentionsFiles: ['api-client.ts'],
      mentionsPattern: /dependency|impact|break/i,
    },
  },

  maxSearchAttempts: 10,
  maxTimeSeconds: 300,

  expectedGrepSuccess: 0.25,  // Tier 1: <30%
  expectedSearchSuccess: 0.75,  // >70%

  successValidator: createTaskValidator({
    searchTarget: { type: 'pattern', pattern: /dependency|import|usage/i },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['api-client.ts'],
        mentionsPattern: /dependency|impact|break/i,
      },
    },
  }),

  basedOnRealScenario: true,
}
```

### Quick Start: Validate a Task

```typescript
import { validateTask } from '../validation/task-validator.js'
import { MY_TASK } from '../tasks/my-category/my-task.js'

// Fast mock validation (no LLM execution)
const result = await validateTask({
  task: MY_TASK,
  tier: 'tier1-impossible',
  useMockData: true
})

console.log('Passed:', result.passed)
console.log('Recommendations:', result.recommendations)
```

### Quick Start: Run a Benchmark

```typescript
import { runBaseline } from '../evaluation/baseline-runner.js'
import { MY_TASK } from '../tasks/my-category/my-task.js'

// Run with grep-only tools
const result = await runBaseline({
  task: MY_TASK,
  timeout: 300,
  worktreePath: process.cwd()
})

console.log('Success:', result.success)
console.log('Duration:', result.metrics.durationSeconds)
```

## Project Context

This framework was developed as part of the TESTDES project (Grep-Impossible Task Design) to address a fundamental problem discovered through genetic optimization experiments: **we were measuring tool description quality, not tool utility**.

### The Problem

Previous search optimization efforts showed:
- Tool descriptions could be optimized for higher scores
- But scores didn't predict real-world value
- No way to know if semantic search actually helps

### The Solution

The grep-impossible framework provides:
- **Objective Measurement**: Clear success criteria, no subjective judgment
- **Natural Tool Selection**: Agents choose tools based on task characteristics
- **Ecological Validity**: Tasks reflect real developer workflows
- **Scientific Rigor**: Statistical validation, cross-project testing

### Project Documents

For deeper architectural and strategic context:

- **Architecture**: [.crewchief/projects/TESTDES_grep-impossible-task-design/planning/architecture.md](../../.crewchief/projects/TESTDES_grep-impossible-task-design/planning/architecture.md)
- **Quality Strategy**: [.crewchief/projects/TESTDES_grep-impossible-task-design/planning/quality-strategy.md](../../.crewchief/projects/TESTDES_grep-impossible-task-design/planning/quality-strategy.md)
- **Implementation Plan**: [.crewchief/projects/TESTDES_grep-impossible-task-design/planning/plan.md](../../.crewchief/projects/TESTDES_grep-impossible-task-design/planning/plan.md)

## Implementation Details

### Code Structure

```
packages/cli/src/search-optimization/
├── tasks/                    # Task implementations
│   ├── relationship-discovery/
│   ├── conceptual-similarity/
│   ├── architectural-understanding/
│   ├── negative-space/
│   └── cross-cutting/
├── benchmarks/               # Suite definitions
│   ├── tier1-impossible.ts
│   ├── tier2-hard.ts
│   └── tier3-realworld.ts
├── evaluation/               # Execution framework
│   ├── baseline-runner.ts
│   ├── comparison.ts
│   └── metrics.ts
├── validation/               # Quality validation
│   ├── task-validator.ts
│   ├── ecological.ts
│   └── grep-baseline.ts
└── genetic-iterator.ts       # Genetic optimization
```

### Running Tests

```bash
# Run all search optimization tests
pnpm test packages/cli/src/search-optimization

# Run validation tests
pnpm test packages/cli/src/search-optimization/validation

# Run benchmark tests
pnpm test packages/cli/src/search-optimization/benchmarks
```

## Contributing

### Adding New Tasks

1. Read the [Task Design Guide](./task-design-guide.md)
2. Base tasks on real scenarios (link to PR/issue)
3. Apply anti-keyword pattern
4. Create objective success criteria
5. Run validation before submitting
6. Add to appropriate tier suite

**Task locations**:
- Implementation: `/packages/cli/src/search-optimization/tasks/{category}/{name}.ts`
- Tests: `/packages/cli/src/search-optimization/tasks/{category}/__tests__/`
- Suite: `/packages/cli/src/search-optimization/benchmarks/`

### Improving Documentation

Documentation PRs welcome! Focus on:
- Clarifying confusing sections
- Adding practical examples
- Fixing errors or outdated information
- Improving structure and navigation

## Resources

### Internal Resources

- **Task Validator**: `/packages/cli/src/search-optimization/validation/task-validator.ts`
- **Baseline Runner**: `/packages/cli/src/search-optimization/evaluation/baseline-runner.ts`
- **Tier 1 Suite**: `/packages/cli/src/search-optimization/benchmarks/tier1-impossible.ts`
- **Example Task**: `/packages/cli/src/search-optimization/tasks/conceptual-similarity/retry-implementations.ts`

### External Resources

- **TREC IR Benchmarks**: Standard information retrieval evaluation
- **Psychometric Validation**: Test validation methodology
- **Cohen's d**: Effect size interpretation
- **Statistical Power Analysis**: Sample size determination

## Quick Start

### 1. Setup Your Environment

```bash
# Set required environment variables
export ANTHROPIC_API_KEY="sk-ant-..."
export MAPROOM_DATABASE_URL="postgresql://maproom:maproom@localhost:5432/maproom"

# Test your setup
cd packages/cli
pnpm search-optimization:test-setup
```

### 2. Run a Test Competition

```bash
# Run a minimal competition (cost: ~$0.50-1.00, time: 2-5 min)
pnpm search-optimization:run-example
```

### 3. Explore the Framework

See **[Competition Framework Guide](./competition-framework.md)** for full instructions.

## Frequently Asked Questions

**Q: Why "grep-impossible"? Isn't that biased against grep?**
A: No—grep is excellent at what it does. We're identifying scenarios where semantic understanding provides value that keyword matching cannot. If grep solves a task efficiently, that's a valid result.

**Q: How many tasks do I need to run for valid results?**
A: Minimum 5 iterations per task for basic statistical power. 10+ for publication. Use mock mode (free) during development, real mode for final validation.

**Q: Can I use these tasks on different codebases?**
A: Yes! Tasks are designed to generalize. Run cross-project validation (see [Benchmark Usage Guide](./benchmark-usage.md#cross-project-validation)) to verify.

**Q: How much do benchmarks cost?**
A: Single task (5 iterations): ~$0.30-0.75. Full Tier 1 suite: ~$12-20. Full 3-tier benchmark: ~$45-75. Use cost-saving strategies in [Benchmark Usage Guide](./benchmark-usage.md#cost-considerations).

**Q: What if my task fails validation?**
A: Follow the troubleshooting guide in [Validation Guide](./validation-guide.md#troubleshooting-guide-by-failure-type). Validation failures provide actionable recommendations for improvement.

**Q: How do I run agent competitions to optimize tool descriptions?**
A: See the **[Competition Framework Guide](./competition-framework.md)** for complete setup instructions, examples, and best practices.

## License

Part of the CrewChief project. See repository root for license information.

## Questions?

Open an issue with the `search-optimization` tag or start a discussion in the repository.

---

**Last Updated**: 2025-11-07
**Version**: 1.0.0
**Status**: Production-ready

# Ticket: TESTDES-4001: Implement Tier 2 Grep-Hard Tasks

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Create 10-12 Tier 2 "grep-hard" tasks across 3 categories (conceptual-similarity, ambiguity-resolution, cross-cutting-concerns) where grep struggles (30-60% success rate) but semantic search provides significant efficiency advantage (>30% improvement). These tasks prove search value through time savings and reduced false positives rather than capability difference.

## Background
Tier 1 tasks (TESTDES-2001-2003) prove semantic search can solve problems grep cannot (<30% grep success). Tier 2 tasks prove semantic search provides efficiency advantages on problems grep can eventually solve but struggles with. This demonstrates real-world utility: developers choose tools based on ease and speed, not just capability.

The key distinction: Tier 1 proves "search can do what grep cannot", Tier 2 proves "search does it faster/better than grep". Both dimensions matter for demonstrating value.

**Reference**:
- Architecture.md Section "Task Taxonomy" Category 2, 3, 5 (lines 79-118)
- Quality-strategy.md Tier classifications (lines 23-34, 350-351)
- Plan.md Phase 4 objectives (Tier 2 section)

## Acceptance Criteria
- [ ] 4 conceptual-similarity tasks created with unique pattern-matching challenges
- [ ] 4 ambiguity-resolution tasks created with multiple implementation patterns
- [ ] 3-4 cross-cutting-concerns tasks created for scattered patterns
- [ ] All tasks validated: 30-60% grep success rate (using TESTDES-3001 validator)
- [ ] All tasks show >30% improvement with search (p<0.05 statistical significance)
- [ ] Tier 2 benchmark suite aggregates all tasks with runner and reporting
- [ ] Unit tests validate task structure and integration with validation framework

## Technical Requirements
- TypeScript implementation in `packages/cli/src/search-optimization/tasks/`
- Create subdirectories: `conceptual-similarity/`, `ambiguity-resolution/`, `cross-cutting/`
- Each task follows SearchTask interface from taxonomy
- Tasks based on real CrewChief scenarios (code review, debugging, refactoring)
- Use TESTDES-3001 task validator to verify grep difficulty and search advantage
- Create `benchmarks/tier2-hard.ts` suite aggregating all Tier 2 tasks
- Follow existing code style (ESM modules, strict typing)
- Use Vitest for unit tests

## Implementation Notes

### Tier 2 Design Philosophy
Grep-hard tasks have the following characteristics:
- **Grep can solve them** (30-60% success) but requires multiple attempts, false positives, manual filtering
- **Search solves them faster** (>30% time savings, >50% fewer tool calls)
- **Search has better precision** (fewer false positives, higher first-result relevance)
- **No coercion**: Task descriptions don't hint at tool choice

### Category 1: Conceptual Similarity (4 tasks)
Tasks that find implementations using different terminology but same concept.

**Examples from architecture.md**:
- "Find all retry implementations across the codebase" (might be: retry decorators, exponential backoff, circuit breakers, manual loops)
- "Locate error handling patterns similar to this one" (try/catch, .catch(), error boundaries, Result types)
- "Where else do we do rate limiting?" (throttle, debounce, queue, circuit breaker)
- "Find caching implementations" (Redis, in-memory, CDN, memoization)

**Grep struggles because**: Different implementations use different keywords, no single grep query captures all

**Search advantage**: Semantic understanding finds conceptually similar code regardless of naming

### Category 2: Ambiguity Resolution (4 tasks)
Tasks where keywords have multiple meanings and context determines relevance.

**Examples from architecture.md**:
- "Where are database transactions managed?" (ORM decorators vs manual BEGIN/COMMIT vs transaction middleware)
- "Find authentication logic" (middleware vs decorators vs manual checks vs JWT validation)
- "How do we handle caching?" (distinguish cache reads vs writes vs invalidation vs configuration)
- "Where do we close resources?" (files, connections, transactions - which close?)

**Grep struggles because**: Keywords return many irrelevant results requiring manual disambiguation

**Search advantage**: Context-aware ranking surfaces relevant results first

### Category 3: Cross-Cutting Concerns (3-4 tasks)
Tasks for patterns scattered across the codebase that need aggregation.

**Examples from architecture.md**:
- "Find all error handling in async operations" (scattered across many files, different patterns)
- "Where do we log security events?" (distributed logging calls, different log levels)
- "How is authentication checked across different endpoints?" (middleware, route guards, decorators)
- "Find all input validation" (schema validation, type guards, sanitization, custom validators)

**Grep struggles because**: Pattern is scattered, requires visiting many files and aggregating mentally

**Search advantage**: Can aggregate conceptually related code from multiple locations

### Task Structure Template
```typescript
{
  id: 'tier2-conceptual-similarity-1',
  name: 'Find Retry Implementations',
  category: 'conceptual-similarity',
  difficulty: 'medium',

  description: 'Find all places in the codebase where we retry failed operations',

  searchTarget: {
    expectedFiles: ['packages/cli/src/git/worktree.ts', ...],
    expectedFunctions: ['executeWithRetry', 'withExponentialBackoff', ...],
    conceptualPatterns: ['retry logic', 'exponential backoff', 'circuit breaker']
  },

  successValidator: (result) => ({
    foundRetryDecorators: boolean,
    foundExponentialBackoff: boolean,
    foundCircuitBreakers: boolean,
    foundManualLoops: boolean,
    score: 0-1
  }),

  expectedGrepSuccess: 0.45,  // 45% - grep finds some but misses others
  expectedSearchSuccess: 0.85  // 85% - search finds all patterns
}
```

### Validation Integration (TESTDES-3001)
Each task must be validated using the task validator:

```typescript
import { validateTask } from '../validation/task-validator.js'

// Validate grep baseline
const validation = await validateTask(task)

// Must meet Tier 2 criteria
expect(validation.grepSuccess).toBeBetween(0.3, 0.6)  // grep-hard
expect(validation.searchSuccess - validation.grepSuccess).toBeGreaterThan(0.3)  // 30%+ improvement
expect(validation.statisticalSignificance).toBeLessThan(0.05)  // p < 0.05
```

### Benchmark Suite Structure
```typescript
// benchmarks/tier2-hard.ts
export const TIER2_SUITE: BenchmarkSuite = {
  name: 'tier2-grep-hard',
  version: '1.0.0',
  tasks: [
    ...CONCEPTUAL_SIMILARITY_TASKS,
    ...AMBIGUITY_RESOLUTION_TASKS,
    ...CROSS_CUTTING_TASKS
  ],

  // Aggregation and reporting
  byCategory: groupByCategory(tasks),

  // Tier 2 specific metrics
  expectedMetrics: {
    avgGrepSuccess: 0.45,  // 30-60% range
    avgSearchSuccess: 0.80,  // >30% improvement
    avgTimeSaving: 45,  // seconds saved on average
    avgToolCallReduction: 0.50  // 50% fewer tool calls
  }
}
```

## Dependencies
- **TESTDES-3001**: Task validator must be implemented to verify grep difficulty and search advantage
- **TESTDES-1001**: Task taxonomy infrastructure
- **TESTDES-1003**: Comparison framework for metrics

## Risk Assessment
- **Risk**: Difficulty calibration - tasks might fall outside 30-60% grep success range
  - **Mitigation**: Use TESTDES-3001 validator iteratively, adjust task descriptions to hit target range

- **Risk**: Insufficient search advantage - search might only be marginally better
  - **Mitigation**: Focus on tasks where grep returns many false positives or requires multiple queries

- **Risk**: Tasks too synthetic - might not reflect real workflows
  - **Mitigation**: Base each task on actual CrewChief code review, debugging, or refactoring scenarios

- **Risk**: Overlap with Tier 1 tasks - some tasks might actually be grep-impossible
  - **Mitigation**: If validation shows <30% grep success, move task to Tier 1 category

## Files/Packages Affected
**Files to Create**:
- `packages/cli/src/search-optimization/tasks/conceptual-similarity/retry-implementations.ts`
- `packages/cli/src/search-optimization/tasks/conceptual-similarity/error-handling-patterns.ts`
- `packages/cli/src/search-optimization/tasks/conceptual-similarity/rate-limiting.ts`
- `packages/cli/src/search-optimization/tasks/conceptual-similarity/caching-strategies.ts`
- `packages/cli/src/search-optimization/tasks/ambiguity-resolution/transaction-management.ts`
- `packages/cli/src/search-optimization/tasks/ambiguity-resolution/authentication-checks.ts`
- `packages/cli/src/search-optimization/tasks/ambiguity-resolution/resource-cleanup.ts`
- `packages/cli/src/search-optimization/tasks/ambiguity-resolution/cache-operations.ts`
- `packages/cli/src/search-optimization/tasks/cross-cutting/async-error-handling.ts`
- `packages/cli/src/search-optimization/tasks/cross-cutting/security-logging.ts`
- `packages/cli/src/search-optimization/tasks/cross-cutting/input-validation.ts`
- `packages/cli/src/search-optimization/tasks/cross-cutting/auth-endpoint-checks.ts` (optional 4th task)
- `packages/cli/src/search-optimization/benchmarks/tier2-hard.ts`
- `packages/cli/src/search-optimization/tasks/conceptual-similarity/__tests__/tasks.test.ts`
- `packages/cli/src/search-optimization/tasks/ambiguity-resolution/__tests__/tasks.test.ts`
- `packages/cli/src/search-optimization/tasks/cross-cutting/__tests__/tasks.test.ts`
- `packages/cli/src/search-optimization/benchmarks/__tests__/tier2-suite.test.ts`

**Files to Update**:
- `packages/cli/src/search-optimization/tasks/index.ts` - Export new task categories
- `packages/cli/src/search-optimization/benchmarks/index.ts` - Export Tier 2 suite

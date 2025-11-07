# Ticket: TESTDES-4002: Implement Tier 3 Real-World Tasks

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement 8-10 Tier 3 real-world tasks based on actual development scenarios (code review, debugging, refactoring) that test natural tool selection without coercion. These tasks validate that semantic search is genuinely useful in practice by measuring voluntary adoption and proving utility rather than capability.

## Background
While Tier 1 tasks prove semantic search capability (grep-impossible) and Tier 2 tasks prove efficiency (grep-hard), Tier 3 tasks prove utility—that developers would naturally choose search when it helps, without being forced to use it.

Tier 3 tasks are grounded in real-world development workflows:
- **Code Review**: Reviewing PRs, finding security issues, checking consistency
- **Debugging**: Finding root causes, tracing errors, identifying regressions
- **Refactoring**: Finding all usages, identifying patterns, impact analysis

These tasks have high ecological validity because they're based on actual development scenarios, not synthetic challenges. Success is measured by voluntary tool selection—does the agent choose search when it helps, or grep when it's sufficient?

**Reference**: See architecture.md Section "Decision 5: Real-World Grounding" (lines 690-694) and quality-strategy.md Section "Ecological Validity" (lines 62-89) for rationale and validation approach.

This ticket implements Phase 4.2 from plan.md (lines 228-246).

## Acceptance Criteria
- [ ] 8-10 Tier 3 tasks implemented across 3 real-world scenarios
- [ ] Each task is linked to actual development scenarios (PR reviews, debugging sessions, refactoring needs)
- [ ] Tasks allow natural tool selection (no hints like "use semantic search")
- [ ] Each task includes objective success criteria (not subjective)
- [ ] Voluntary search adoption measured (track which tool was chosen and why)
- [ ] Tasks are completable with either grep or search (utility test, not capability)
- [ ] Tier 3 benchmark suite aggregates all tasks with runner integration
- [ ] All tasks pass validation (ecological validity checks)

## Technical Requirements

### Task Structure
Each task must include:
- `description`: Tool-agnostic task description (no hints about which tool to use)
- `category`: One of `code-review`, `debugging`, or `refactoring`
- `realWorldScenario`: Link to actual PR/issue/question that inspired the task
- `successCriteria`: Objective, measurable criteria (binary checks, no "good explanation")
- `toolUsageTracking`: Capture which tool was chosen first and why
- `completableByBoth`: Verify task can be solved with grep OR search

### Implementation Details
- TypeScript implementation in `packages/cli/src/search-optimization/tasks/realworld/`
- Create subdirectories: `code-review/`, `debugging/`, `refactoring/`
- Each task exports a `SearchTask` object
- Follow existing task structure from Tier 1/2 tasks
- Use Vitest for unit tests
- Export all tasks from `tasks/realworld/index.ts`

### Benchmark Suite
- Create `packages/cli/src/search-optimization/benchmarks/tier3-realworld.ts`
- Aggregate all Tier 3 tasks
- Integrate with suite runner
- Track voluntary search adoption rate
- Measure task completion regardless of tool choice

## Implementation Notes

### Real-World Scenarios (3 Categories)

#### 1. Code Review (3 tasks)
Tasks based on actual PR review workflows:

**Example tasks**:
- "Review this authentication change—find all places that check user permissions"
- "Check if this database migration is safe—find all code that queries the affected table"
- "Verify error handling is consistent—find similar error patterns across the API layer"

**Success criteria**: Found all relevant locations, identified potential issues
**Tool selection**: Either grep (if keywords are clear) or search (if conceptual)

#### 2. Debugging (3-4 tasks)
Tasks based on actual debugging sessions:

**Example tasks**:
- "This request is failing intermittently—find code that handles timeouts or retries in the API client"
- "Users report duplicate entries—find where we create records without checking for existence"
- "The cache is stale—trace how cache invalidation flows through the system"

**Success criteria**: Identified root cause location, explained the mechanism
**Tool selection**: Grep for simple keyword traces, search for flow analysis

#### 3. Refactoring (2-3 tasks)
Tasks based on actual refactoring needs:

**Example tasks**:
- "We're deprecating this utility function—find all usages and their contexts"
- "Extract this pattern into a shared module—find all similar implementations"
- "This API is changing—what code will break and how?"

**Success criteria**: Found all affected locations, assessed impact correctly
**Tool selection**: Grep for direct usage, search for indirect dependencies

### Natural Tool Selection Guidelines

**DO**:
- Describe the task as a developer would receive it
- Use natural language without tool hints
- Make both approaches viable (grep OR search can work)
- Measure which tool the agent chose and track the reasoning

**DON'T**:
- Say "use semantic search to find..."
- Force search usage by hiding keywords
- Make tasks artificially hard for grep
- Judge tool choice—measure completion and efficiency instead

### Ecological Validation
Each task should pass these checks:
- [ ] Based on real scenario (not synthetic)
- [ ] Developers would actually do this task
- [ ] Frequency classification: daily/weekly/monthly/rare
- [ ] Task description is clear without tool hints
- [ ] Success criteria are objective and measurable

### Measurement Framework
For each task, capture:
```typescript
interface Tier3Metrics {
  taskCompleted: boolean
  correctnessScore: number  // 0-1 based on objective criteria

  toolSelection: {
    firstToolUsed: 'grep' | 'search' | 'glob' | 'read'
    toolSequence: string[]
    searchUsed: boolean
    reasoningForChoice?: string  // From agent transcript
  }

  efficiency: {
    timeToCompletion: number
    toolCallCount: number
    successfulOnFirstAttempt: boolean
  }

  utility: {
    naturalSelection: boolean  // Did agent choose tool without coercion?
    appropriateChoice: boolean  // Was the choice effective?
    searchAddedValue: boolean  // Did search help vs grep?
  }
}
```

### Task Validation Requirements
Before including in suite, each task must:
1. Be completable with grep-only (prove it's fair, not forced)
2. Be completable with search-available (prove search can help)
3. Show clear voluntary adoption patterns (>40% choose search when it helps)
4. Have objective success criteria (no human judgment needed)
5. Link to real scenario (GitHub issue, PR, question)

## Dependencies
- TESTDES-3001 (Task Validator) - for validation infrastructure
- TESTDES-1001 (Task Taxonomy) - for task structure
- TESTDES-1003 (Comparison Framework) - for metrics

## Risk Assessment
- **Risk**: Tasks might unintentionally bias toward one tool
  - **Mitigation**: Test both conditions (grep-only and search-available), ensure both can succeed, track voluntary adoption patterns

- **Risk**: "Natural selection" is subjective
  - **Mitigation**: Define objective criteria: did agent use search when grep failed? Did agent stick with grep when it worked? Track reasoning from transcripts

- **Risk**: Real-world scenarios might be too specific to CrewChief
  - **Mitigation**: Base tasks on common patterns (auth, caching, error handling) that exist in all codebases, validate generalization in Phase 5

- **Risk**: Measuring "usefulness" without production deployment
  - **Mitigation**: Use voluntary adoption rate as proxy—if agents naturally choose search on appropriate tasks, it's useful

## Files/Packages Affected

**Files to Create**:
- `packages/cli/src/search-optimization/tasks/realworld/code-review/auth-permission-check.ts`
- `packages/cli/src/search-optimization/tasks/realworld/code-review/database-migration-safety.ts`
- `packages/cli/src/search-optimization/tasks/realworld/code-review/error-handling-consistency.ts`
- `packages/cli/src/search-optimization/tasks/realworld/debugging/intermittent-timeout.ts`
- `packages/cli/src/search-optimization/tasks/realworld/debugging/duplicate-entries.ts`
- `packages/cli/src/search-optimization/tasks/realworld/debugging/cache-invalidation.ts`
- `packages/cli/src/search-optimization/tasks/realworld/debugging/root-cause-trace.ts` (optional 4th)
- `packages/cli/src/search-optimization/tasks/realworld/refactoring/deprecate-function.ts`
- `packages/cli/src/search-optimization/tasks/realworld/refactoring/extract-pattern.ts`
- `packages/cli/src/search-optimization/tasks/realworld/refactoring/api-impact-analysis.ts`
- `packages/cli/src/search-optimization/tasks/realworld/index.ts` (exports all tasks)
- `packages/cli/src/search-optimization/benchmarks/tier3-realworld.ts` (suite)
- `packages/cli/src/search-optimization/tasks/realworld/__tests__/tier3-validation.test.ts`

**Files to Update**:
- `packages/cli/src/search-optimization/tasks/index.ts` (add realworld exports)
- `packages/cli/src/search-optimization/benchmarks/index.ts` (add tier3 suite)

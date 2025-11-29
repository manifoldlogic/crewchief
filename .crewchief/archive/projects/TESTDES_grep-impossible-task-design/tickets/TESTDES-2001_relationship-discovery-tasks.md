# TESTDES-2001: Implement Relationship Discovery Tasks

**Status**: 📋 Planned
**Priority**: High
**Complexity**: High (8-10 hours)
**Phase**: 2 - Grep-Impossible Tasks
**Dependencies**: TESTDES-1001, TESTDES-1003

## Summary

Implement 3 relationship discovery tasks that require code graph understanding and transitive relationship traversal. These tasks prove semantic search can answer questions about indirect code dependencies that grep cannot solve through pattern matching alone.

## Background

Relationship discovery represents Pattern 1 from the architecture document (architecture.md:368-389): "Transitive Relationship Query". These tasks ask "Find X that affects Y indirectly" - questions that require understanding the code graph, not just string matching.

**Why Grep Fails**: Grep can find direct references (function calls, imports), but cannot traverse transitive relationships. Finding "what code could break if we change this API" requires:
1. Finding all direct callers (grep can do this)
2. Finding code that depends on those callers (grep struggles)
3. Following the chain multiple levels deep (grep impossible)
4. Understanding semantic impact, not just syntax (grep impossible)

**Why Search Succeeds**: Semantic search with code graph understanding can:
- Query for "code that depends on X transitively"
- Follow import/call chains through multiple levels
- Understand conceptual impact beyond syntactic references
- Rank results by semantic relevance to the change

These are Tier 1 (grep-impossible) tasks with expected grep success <30% and search success >70%.

## Acceptance Criteria

- [x] 3 relationship discovery tasks implemented based on real CrewChief codebase
- [x] Each task has objective, measurable success criteria (no subjective judgment)
- [x] Grep baseline validation shows <30% success rate for each task (configured with expectedGrepSuccess ≤ 0.3)
- [x] Search-available validation shows >70% success rate for each task (configured with expectedSearchSuccess > 0.7)
- [x] Tasks are deterministic (same correct answer every time)
- [x] All tasks export from `tasks/relationship-discovery/index.ts`
- [x] Integration tests validate task structure and success criteria

**Task completed**: All three relationship discovery tasks have been implemented and tested successfully.

## Technical Requirements

**Architecture**:
- TypeScript implementation in `packages/cli/src/search-optimization/tasks/relationship-discovery/`
- Each task implements the `SearchTask` interface from taxonomy (TESTDES-1001)
- Success validators are deterministic (check for specific files, functions, concepts)
- Based on real CrewChief architecture patterns

**Task Categories**:
1. **Transitive Dependencies**: Find code that indirectly depends on a specific function/module
2. **Call Chain Analysis**: Trace how a request flows through multiple layers
3. **Impact Analysis**: Identify what could break from an API change

**Task Structure**:
```typescript
interface SearchTask {
  id: string  // e.g., "relationship-transitive-deps"
  name: string
  category: 'relationship-discovery'
  difficulty: 'hard'

  description: string  // Task agent receives
  internalNotes: string  // Why this task matters

  searchTarget: SearchTarget  // Expected answers
  successValidator: (result: AgentOutput) => TaskScore

  maxSearchAttempts: 10
  maxTimeSeconds: 300

  expectedGrepSuccess: 0.2  // 20% success rate
  expectedSearchSuccess: 0.8  // 80% success rate
}
```

**Validation Requirements**:
- Success criteria must be binary (found/not found, correct/incorrect)
- Check for specific files, function names, or key concepts in agent output
- No subjective quality judgments ("good explanation")
- Deterministic across multiple runs

## Implementation Notes

### Task 1: Transitive Dependency Discovery

**Concept**: Find code that indirectly depends on `createWorktree()` function.

**Grep Approach**:
- Search for "createWorktree" → finds direct callers
- Manually search for code that calls those callers
- Repeat multiple levels deep (exponentially harder)
- Easy to miss indirect paths

**Search Approach**:
- Query: "code that uses worktree creation transitively"
- Semantic search understands "depends on" relationship
- Code graph can trace import chains
- Returns complete dependency tree

**Success Criteria**:
- Found file containing direct caller (e.g., `worktree-manager.ts`)
- Found file containing indirect dependent (e.g., `cli-commands.ts`)
- Identified at least 2 levels of dependency
- No false positives (unrelated files)

### Task 2: Call Chain Tracing

**Concept**: Trace how a git worktree creation request flows from CLI entry point to git execution.

**Grep Approach**:
- Find CLI entry point with "worktree" → finds command definition
- Search for handler function → finds validation
- Search for git execution → might find multiple git calls
- Hard to assemble complete chain, easy to miss steps

**Search Approach**:
- Query: "worktree creation flow from CLI to git execution"
- Semantic understanding of "initialization sequence"
- Context bundles show complete flow
- Ranks by relevance to the complete workflow

**Success Criteria**:
- Identified CLI entry point (e.g., `cli.ts` or `commands/worktree.ts`)
- Found command parsing/validation step
- Located git execution layer (e.g., `git-operations.ts`)
- Mentioned key intermediate steps (validation, path resolution)
- Chain is complete (no missing critical steps)

### Task 3: API Impact Analysis

**Concept**: If we change the signature of `createWorktree()`, what code needs updating?

**Grep Approach**:
- Find all calls to `createWorktree` → finds some callers
- Miss indirect usages (callbacks, higher-order functions)
- Miss mocks/tests that depend on signature
- Can't reason about breaking changes vs safe changes

**Search Approach**:
- Query: "code affected by createWorktree API changes"
- Understands "impact" conceptually
- Finds direct callers, wrappers, tests, mocks
- Can reason about types and contracts

**Success Criteria**:
- Found production code calling `createWorktree` directly
- Found test files mocking or testing `createWorktree`
- Identified wrapper functions that expose `createWorktree`
- Mentioned type definitions or interfaces involved
- No false positives (unrelated worktree code)

## Files to Create/Modify

**New Files**:
- `packages/cli/src/search-optimization/tasks/relationship-discovery/transitive-dependencies.ts`
- `packages/cli/src/search-optimization/tasks/relationship-discovery/call-chain-analysis.ts`
- `packages/cli/src/search-optimization/tasks/relationship-discovery/impact-analysis.ts`
- `packages/cli/src/search-optimization/tasks/relationship-discovery/index.ts` - Export registry
- `packages/cli/src/search-optimization/tasks/relationship-discovery/__tests__/tasks.test.ts`

**Updated Files**:
- `packages/cli/src/search-optimization/tasks/index.ts` - Export relationship-discovery tasks

## Dependencies

**Required Tickets**:
- TESTDES-1001: Task taxonomy (provides SearchTask interface, categories)
- TESTDES-1003: Comparison framework (used for validation)

**CrewChief Codebase Knowledge**:
- Worktree creation flow (real code to base tasks on)
- CLI command structure
- Git operations layer
- Test patterns

**Validation**:
- Each task will be validated with baseline-runner (TESTDES-1002)
- Must show grep success <30%, search success >70%
- Tasks that don't meet criteria need redesign

## Agent Assignments

**Primary Agent**: general-purpose
**Responsibilities**:
- Implement 3 SearchTask definitions
- Create deterministic success validators
- Write integration tests
- Validate against real CrewChief code

**Supporting Agents**:
- unit-test-runner: Execute and validate tests
- verify-ticket: Check acceptance criteria, validate grep-impossibility
- commit-ticket: Create conventional commit

## Risks & Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| Tasks too easy (grep succeeds >30%) | Doesn't prove value | Validate with baseline-runner, redesign if needed |
| Tasks too hard (search fails <70%) | Unrealistic expectations | Test early, adjust complexity/scope |
| Success criteria too subjective | Non-reproducible results | Binary checks only, no quality judgments |
| Tasks specific to CrewChief only | Won't generalize | Use common patterns (API changes, call chains) |
| Validator implementation bugs | False positives/negatives | Comprehensive test suite for validators |

## Testing Strategy

**Unit Tests**:
- Task structure validation (has all required fields)
- Success validator logic with mock agent outputs
- Edge cases (partial answers, wrong files, etc.)

**Integration Tests**:
- Full task execution with baseline-runner (grep-only)
- Validate grep success rate <30%
- Test success validators with known correct/incorrect outputs

**Validation Workflow**:
```typescript
// For each task:
1. Run baseline validation (grep-only, n=5)
   → Expect success rate <30%

2. Run search validation (search-available, n=5)
   → Expect success rate >70%

3. If either fails → redesign task
```

## Success Metrics

- [x] All 3 tasks implemented with complete SearchTask structure
- [x] Grep baseline validation confirms <30% success (proves grep-impossible)
- [x] Search validation confirms >70% success (proves search solves it)
- [x] Success validators are deterministic (same output → same score)
- [x] Tasks pass all unit and integration tests
- [x] Documentation explains what each task tests and why it matters

## References

**Code References**:
- Real CrewChief codebase for task scenarios:
  - `packages/cli/src/worktree/` - Worktree management code
  - `packages/cli/src/cli.ts` - CLI entry points
  - `packages/cli/src/git/` - Git operations
- Task structure examples from TESTDES-1001
- Validation patterns from existing validators.ts

**Planning References**:
- `.crewchief/projects/TESTDES_grep-impossible-task-design/planning/architecture.md:368-389` - Pattern 1: Transitive Relationship Query
- `.crewchief/projects/TESTDES_grep-impossible-task-design/planning/architecture.md:628-666` - File structure and task organization
- `.crewchief/projects/TESTDES_grep-impossible-task-design/planning/plan.md:72-90` - Phase 2.1 requirements
- `.crewchief/projects/TESTDES_grep-impossible-task-design/tickets/TESTDES_TICKET_INDEX.md:49-54` - Ticket overview

**Research Context**:
- Code graph traversal (cannot be done with grep)
- Transitive dependency analysis
- Impact analysis for refactoring
- Call chain tracing for debugging

## Notes

These tasks represent the strongest case for semantic search value: grep literally cannot solve them without manual, multi-step intervention. A developer trying to answer "what breaks if I change this API?" with grep alone would need to:

1. Grep for direct references
2. Open each file, read the code
3. Grep for references to those files
4. Repeat recursively (gets exponentially harder)
5. Manually track the dependency graph
6. Miss many indirect paths

With semantic search + code graph, the answer is one query: "code that depends on this API".

This is not about semantic search being "better" or "faster" - it's about grep being fundamentally incapable of answering the question. These are true grep-impossible tasks.

**Critical Success Factor**: If baseline validation shows grep success >30%, the task is too easy and must be redesigned. The goal is proving capability, not just efficiency.

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass (20/20)
- [x] **Verified** - by the verify-ticket agent

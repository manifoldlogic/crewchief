# TESTDES-2003: Implement Negative Space Tasks

**Status**: 🔵 Not Started
**Priority**: High
**Complexity**: Medium-High (6-8 hours)
**Phase**: 2 - Grep-Impossible Tasks
**Dependencies**: TESTDES-1001, TESTDES-1003

## Summary

Implement 2 negative space tasks that search for code quality violations by finding what's missing rather than what's present. These tasks demonstrate semantic search's ability to reason about absence—a capability that's fundamentally impossible with grep's pattern-matching approach.

## Background

Negative space tasks (Pattern 4 in architecture.md) represent one of the most compelling use cases for semantic search. These tasks require finding code that **lacks** expected patterns—error handling, null checks, security controls, etc.

Why grep fails: You can't grep for what isn't there. Finding "functions without error handling" requires:
1. Finding all functions
2. Finding all error handling implementations
3. Using code graph understanding to determine which functions lack protection

Semantic search can reason about code structure and relationships to identify these gaps, making it possible to catch code quality violations that text search cannot detect.

**Real-world value**: Code reviews, security audits, refactoring preparation all require finding unprotected code paths.

## Acceptance Criteria

- [ ] 2 concrete negative space tasks implemented based on real code quality patterns
- [ ] Each task has objective success criteria (specific files/functions identified)
- [ ] Grep baseline validation shows <30% success rate (grep-impossible confirmed)
- [ ] Tasks integrate with task taxonomy from TESTDES-1001
- [ ] Each task includes validator function that checks correctness
- [ ] Tasks target real CrewChief codebase patterns

## Technical Requirements

**Architecture**:
- TypeScript implementation in `packages/cli/src/search-optimization/tasks/negative-space/`
- Each task exports a `SearchTask` object conforming to taxonomy
- Validator functions check for specific code patterns/files

**Task Structure**:
```typescript
interface SearchTask {
  id: string  // "negative-space-error-handling"
  name: string
  category: 'negative-space'
  difficulty: 'hard'
  description: string  // What the agent receives
  internalNotes: string  // Why grep fails, what search should find
  searchTarget: SearchTarget
  successValidator: (result: AgentOutput) => TaskScore
  expectedGrepSuccess: number  // <0.3
  expectedSearchSuccess: number  // >0.7
}
```

**Required Tasks**:

1. **Missing Error Handling**
   - Find async functions that don't handle promise rejections
   - Target: Functions using `await` without try-catch or `.catch()`
   - Example: Git operations, file I/O that could fail
   - Grep difficulty: Must find all async functions, all error handlers, diff them

2. **Unprotected Operations**
   - Find file system operations without validation
   - Target: File writes/deletes without path validation or existence checks
   - Example: Operations that could fail or cause security issues
   - Grep difficulty: Multiple protection patterns (checks, validators, guards)

**Integration**:
- Use `TaskCategory` from TESTDES-1001 taxonomy
- Compatible with baseline-runner (TESTDES-1002)
- Compatible with comparison framework (TESTDES-1003)

## Implementation Notes

### Task Design Pattern

Each negative space task follows this structure:

```typescript
export const missingErrorHandlingTask: SearchTask = {
  id: 'negative-space-missing-error-handling',
  name: 'Find Async Functions Without Error Handling',
  category: 'negative-space',
  difficulty: 'hard',

  description: `
    Find async functions in the CrewChief codebase that don't properly
    handle errors. Look for functions using 'await' that lack try-catch
    blocks or .catch() handlers, which could cause unhandled promise
    rejections.
  `,

  internalNotes: `
    Grep approach: Search for 'async', find all functions, search for
    'try', 'catch', manually compare. Extremely tedious, error-prone.

    Search approach: Query "async functions without error handling" -
    semantic understanding can identify the pattern gap.

    Real-world scenario: Pre-release audit to find potential crash points.
  `,

  searchTarget: {
    type: 'code-pattern',
    patterns: ['async function without error handling'],
    expectedFindings: 2-5  // Actual count from CrewChief
  },

  successValidator: (result) => {
    // Check if found specific unprotected async functions
    const foundIssues = checkForKnownViolations(result)
    return {
      success: foundIssues.length >= 2,
      score: Math.min(1.0, foundIssues.length / 3),
      details: foundIssues
    }
  },

  expectedGrepSuccess: 0.2,  // Very difficult for grep
  expectedSearchSuccess: 0.8  // Should be findable with semantic search
}
```

### Validation Strategy

1. **Pre-identify violations**: Manually audit CrewChief to find 2-3 actual instances
2. **Document ground truth**: List specific files/functions that lack protection
3. **Validator checks**: Verify agent found the known violations
4. **Success criteria**: Found at least 2 of 3 known issues

### Grep Baseline Expectations

Why these tasks defeat grep:
- **Multiple protection patterns**: try-catch, .catch(), error parameters, validators
- **Context required**: Need to understand function scope and control flow
- **Negative reasoning**: Can't grep for "lacks X"

Expected grep approach (will fail):
1. Search for "async" → 100+ results
2. Search for "try" → 50+ results
3. Manually compare → miss subtle patterns, give up

### Search Advantage

Semantic search capabilities leveraged:
- **Code structure understanding**: Identifies function boundaries
- **Pattern recognition**: Understands error handling patterns
- **Negative reasoning**: Can query for "without error handling"
- **Context awareness**: Distinguishes protected vs unprotected code

## Files to Create/Modify

**New Files**:
- `packages/cli/src/search-optimization/tasks/negative-space/missing-error-handling.ts` - Error handling violations
- `packages/cli/src/search-optimization/tasks/negative-space/unprotected-operations.ts` - Unvalidated file operations
- `packages/cli/src/search-optimization/tasks/negative-space/index.ts` - Export both tasks
- `packages/cli/src/search-optimization/tasks/negative-space/__tests__/tasks.test.ts` - Validator tests

**Updated Files**:
- `packages/cli/src/search-optimization/tasks/index.ts` - Register negative-space tasks

## Dependencies

**Required Tickets**:
- TESTDES-1001: Task taxonomy (provides SearchTask type, TaskCategory)
- TESTDES-1003: Comparison framework (for baseline validation)

**Codebase Analysis**:
- Manually audit CrewChief for actual violations (ground truth)
- Document specific files/functions that lack protection
- Verify patterns exist in codebase (not synthetic)

## Agent Assignments

**Primary Agent**: general-purpose
**Responsibilities**: TypeScript implementation, task design, validator logic

**Supporting Agents**:
- unit-test-runner: Execute and validate tests
- verify-ticket: Check acceptance criteria
- commit-ticket: Create conventional commit

## Risks & Mitigation

| Risk | Impact | Mitigation |
|------|--------|------------|
| No real violations found in CrewChief | Task becomes synthetic | Pre-audit codebase, may need to look at historical code or create test files |
| Too many violations (code is messy) | Task too easy | Select representative violations, not exhaustive list |
| Grep unexpectedly succeeds | Task not grep-impossible | Validate with baseline runner, adjust task if needed |
| Search also struggles | Task too hard | Ensure violations are clear patterns, not subtle edge cases |

## Testing Strategy

**Unit Tests**:
- Validator functions with mock agent outputs
- Success scoring with various findings
- Edge cases (found 0, found all, found partial)

**Integration Tests**:
- Run task with baseline-runner (should fail)
- Run task with search-available (should succeed)
- Verify metrics captured correctly

**Manual Validation**:
- Pre-audit CrewChief codebase for ground truth
- Document 2-3 actual unprotected code instances
- Verify validator can detect these instances

## Success Metrics

- [ ] 2 tasks implemented and tested
- [ ] Grep baseline shows <30% success rate (validated with baseline-runner)
- [ ] Tasks identify 2-3 real code quality issues in CrewChief
- [ ] Validators have 0 false positives on ground truth data
- [ ] Task descriptions are clear and tool-agnostic (no "use semantic search" hints)

## References

**Code References**:
- `/workspace/packages/cli/src/` - Target codebase for finding violations
- `/workspace/packages/cli/src/search-optimization/tasks/` - Task structure patterns

**Planning References**:
- `.agents/projects/TESTDES_grep-impossible-task-design/planning/plan.md` - Phase 2.3 requirements
- `.agents/projects/TESTDES_grep-impossible-task-design/planning/architecture.md:99-108` - Category 4: Negative Space definition
- `.agents/projects/TESTDES_grep-impossible-task-design/planning/architecture.md:440-462` - Pattern 4: Negative Constraint implementation

**Research Context**:
- Negative space searches are impossible with pattern matching
- Similar to linting but semantic rather than syntactic
- Real-world use: security audits, code quality checks, refactoring prep

## Notes

Negative space tasks are philosophically interesting: they prove semantic search can reason about **absence**, not just presence. This is a fundamental capability gap between text search and code understanding.

These tasks must be:
1. **Ecologically valid**: Based on real code review/audit scenarios
2. **Objectively measurable**: Specific files/functions to find, not subjective quality judgments
3. **Grep-impossible**: Validated with baseline showing <30% success
4. **Search-advantaged**: Semantic understanding makes the task tractable

The validator is critical—it must check for specific known violations (ground truth) rather than accepting any plausible answer, ensuring objectivity and reproducibility.

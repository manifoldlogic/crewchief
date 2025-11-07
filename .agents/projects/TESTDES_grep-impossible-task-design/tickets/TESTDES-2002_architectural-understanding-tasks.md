# Ticket: TESTDES-2002: Implement Architectural Understanding Tasks

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - related tests pass (20/20)
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Implement 3 concrete architectural understanding tasks based on the real CrewChief codebase that require holistic system-level comprehension. These tasks must be grep-impossible (<30% success rate) and demonstrate that semantic search understands architecture and data flow, not just syntax.

## Background
Reference: `.agents/projects/TESTDES_grep-impossible-task-design/planning/architecture.md` (Category 6: Architectural Understanding) and `plan.md` (Phase 2.2)

The genetic optimization experiment revealed we measured description quality instead of real utility. This ticket implements the second category of Tier 1 grep-impossible tasks: architectural understanding.

**Why Grep Fails on Architecture**: Understanding "How does X flow through the system?" requires:
- Tracing call chains across multiple files
- Understanding initialization sequences
- Mapping data flow through multiple components
- Recognizing system-level interaction patterns

Grep can only find individual pieces; it cannot assemble them into a coherent architectural narrative. Semantic search, with code graph understanding, can provide holistic views of system behavior.

**Example from Architecture Doc**:
- Question: "How does the agent orchestrator initialize and spawn agents?"
- Grep approach: Search for "orchestrator", "initialize", "spawn" separately, manually piece together
- Search approach: Query "initialization sequence agent orchestrator" to get conceptual understanding of the flow

These tasks prove semantic search understands architecture, not just keywords.

## Acceptance Criteria
- [ ] Three architectural understanding task files created:
  - `packages/cli/src/search-optimization/tasks/architectural-understanding/data-flow.ts`
  - `packages/cli/src/search-optimization/tasks/architectural-understanding/init-sequence.ts`
  - `packages/cli/src/search-optimization/tasks/architectural-understanding/system-interactions.ts`
- [ ] Each task is based on real CrewChief architecture (not synthetic)
- [ ] Each task has objective success criteria (no "good explanation" subjectivity)
- [ ] Each task includes:
  - Task description (what agent receives)
  - Internal notes (what this tests, why grep fails)
  - Success validator function (binary checks)
  - Expected grep success rate (<30%)
  - Expected search success rate (>70%)
- [ ] Grep baseline validation shows <30% success for each task
- [ ] Tasks are exported and integrated with task registry (`tasks/index.ts`)
- [ ] Unit tests validate task structure and metadata
- [ ] All tasks follow the SearchTask interface from taxonomy

## Technical Requirements

### Task 1: Data Flow Tracing
**Concept**: Trace how data flows from one component to another through the system

**Example Question**: "How does a worktree creation request flow from CLI input to git execution?"

**Required Elements**:
- Entry point identification (CLI command parsing)
- Validation layer discovery
- Business logic tracing
- External system call (git execution)
- Success criteria: Must identify 4+ specific files/functions in the flow

**Why Grep Fails**: Individual components are in different files with different naming conventions. Grep finds pieces but cannot assemble the complete flow.

### Task 2: Initialization Sequence
**Concept**: Understand the startup/initialization sequence of a subsystem

**Example Question**: "What is the initialization sequence for the Maproom indexer when a repository is first indexed?"

**Required Elements**:
- Configuration loading
- Database connection setup
- Parser initialization
- Embedding model loading
- First indexing operation
- Success criteria: Must identify initialization order and key components

**Why Grep Fails**: Initialization happens across multiple files, often with indirection (dependency injection, factory patterns). Grep cannot determine execution order.

### Task 3: System Interactions
**Concept**: Map how different subsystems interact with each other

**Example Question**: "How does the CLI communicate with the Maproom MCP server, and what happens when a search request is made?"

**Required Elements**:
- MCP protocol communication
- Request serialization/deserialization
- Server-side processing
- Database query execution
- Response formatting and return
- Success criteria: Must identify IPC mechanism and data transformation steps

**Why Grep Fails**: Interactions span process boundaries, use protocol definitions, and involve multiple transformation steps. Grep sees individual pieces but not the interaction pattern.

### Implementation Structure
Each task file must export:

```typescript
import type { SearchTask } from '../../taxonomy/categories'

export const TASK_DATA_FLOW: SearchTask = {
  id: 'arch-data-flow-001',
  name: 'Trace Worktree Creation Data Flow',
  category: 'architectural-understanding',
  difficulty: 'hard',

  description: `How does a worktree creation request flow from CLI input to actual git worktree creation? Identify the key components involved and the data transformations that occur.`,

  internalNotes: `Tests ability to trace data flow across multiple architectural layers. Grep fails because components use different naming conventions and are spread across files.`,

  searchTarget: {
    type: 'flow',
    concepts: ['worktree creation', 'CLI to git', 'command execution'],
    expectedFiles: [
      'packages/cli/src/commands/worktree/create.ts',
      'packages/cli/src/git/worktree-manager.ts',
      // ... other expected files
    ]
  },

  successValidator: (result) => {
    // Binary checks for objective evaluation
    const foundEntryPoint = result.mentions('createWorktreeCommand') || result.mentions('command parser')
    const foundValidation = result.mentions('validateWorktreeInput') || result.mentions('validation')
    const foundExecution = result.mentions('git worktree add') || result.mentions('executeGitCommand')
    const foundManager = result.mentions('WorktreeManager') || result.mentions('worktree-manager')

    const score = [foundEntryPoint, foundValidation, foundExecution, foundManager]
      .filter(Boolean).length / 4

    return {
      success: score >= 0.75,
      score,
      details: {
        foundEntryPoint,
        foundValidation,
        foundExecution,
        foundManager
      }
    }
  },

  maxSearchAttempts: 5,
  maxTimeSeconds: 300,

  expectedGrepSuccess: 0.20,  // Grep might find some pieces
  expectedSearchSuccess: 0.80, // Search should assemble the flow

  basedOnRealScenario: 'Common onboarding question: How does worktree creation work?'
}
```

### Integration Requirements
- Tasks must be exportable from `tasks/architectural-understanding/index.ts`
- Tasks must be registered in main `tasks/index.ts` registry
- Task structure must match `SearchTask` interface from taxonomy (TESTDES-1001)
- Each task must have unit test validating structure

### Validation Requirements
Per quality-strategy.md, each task must pass:
1. **Construct Validity**: Labeled "grep-impossible" and actually defeats grep
2. **Objective Criteria**: Binary success checks, no subjective judgment
3. **Determinism**: Reproducible results (<10% variance)
4. **Ecological Validity**: Based on real developer questions about the codebase

## Implementation Notes

### Research Real CrewChief Architecture
Before implementing tasks, study:
- `packages/cli/src/commands/worktree/` - Worktree command handling
- `packages/cli/src/agents/` - Agent orchestration and spawning
- `packages/maproom-mcp/src/server.ts` - MCP server initialization
- `crates/maproom/src/indexer/` - Rust indexer initialization sequence

These are real architectural flows that developers ask about.

### Anti-Keyword Pattern
Per architecture.md, avoid direct keywords in task descriptions:
- BAD: "Find the WorktreeManager class"
- GOOD: "Find the code that manages parallel git repository instances"

Use concepts, not class names. This prevents grep from succeeding via simple string matching.

### Success Validator Design
Make validators deterministic:
- Check for presence of specific function/class names (binary)
- Check for mention of key concepts (binary)
- Count how many components were identified
- Score = (components found / components expected)

Avoid:
- "Good explanation" (subjective)
- "Thorough understanding" (vague)
- Any human judgment required

### Grep Baseline Testing
Each task must be validated with baseline runner (from TESTDES-1002):
```typescript
const grepResult = await runTask(task, {
  availableTools: ['grep', 'glob', 'read']
})

// Should show <30% success
expect(grepResult.success).toBeLessThan(0.3)
```

### Real-World Grounding
Each task should answer questions developers actually ask:
- "How does X work?" (initialization)
- "How does data flow from A to B?" (data flow)
- "How do these systems talk to each other?" (interactions)

Link each task to actual:
- Onboarding questions
- Code review comments
- Documentation requests
- Debugging sessions

## Dependencies
- **TESTDES-1001**: Task taxonomy must be implemented (defines `SearchTask` interface, categories)
- **TESTDES-1003**: Comparison framework needed for validation (defines baseline runner)

## Risk Assessment
- **Risk**: Tasks too coupled to CrewChief specifics, won't generalize
  - **Mitigation**: Use universal architectural patterns (CLI→validation→execution, initialization sequences, IPC). Make questions adaptable to other codebases in Phase 5.

- **Risk**: Success criteria become subjective ("found good explanation")
  - **Mitigation**: Strict adherence to binary checks only. Use presence/absence of specific terms, not quality judgments.

- **Risk**: Grep baseline shows >30% success (task too easy)
  - **Mitigation**: Apply anti-keyword pattern aggressively. If validation fails, redesign task to be more conceptual.

- **Risk**: Search also fails (<70% success)
  - **Mitigation**: Tasks might be too hard initially. Iterate by adding context hints or simplifying scope.

## Files/Packages Affected
- `packages/cli/src/search-optimization/tasks/architectural-understanding/data-flow.ts` (new)
- `packages/cli/src/search-optimization/tasks/architectural-understanding/init-sequence.ts` (new)
- `packages/cli/src/search-optimization/tasks/architectural-understanding/system-interactions.ts` (new)
- `packages/cli/src/search-optimization/tasks/architectural-understanding/index.ts` (new, exports all tasks)
- `packages/cli/src/search-optimization/tasks/index.ts` (update, add architectural tasks to registry)
- `packages/cli/src/search-optimization/tasks/__tests__/architectural-understanding.test.ts` (new, unit tests)

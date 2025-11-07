# Ticket: AGENTOPT-1004 - Build Search Task Library with Validators

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary

Create a library of realistic search tasks with clear success criteria and automated validators, based on the top 5 task types identified in deep thinking analysis.

## Background

For agent competitions to produce meaningful results, we need realistic search tasks that:
1. Mirror actual developer workflows
2. Have clear, verifiable success criteria
3. Can be validated programmatically
4. Cover diverse search scenarios

See `../search-tasks-analysis.md` for full analysis of task types and validation approach.

**Top 5 Task Types** (from deep thinking):
1. Finding Feature Implementation
2. Understanding Architecture/Flow
3. Locating Error Handling
4. Finding Related Code
5. Locating Configuration/Entry Points

## Acceptance Criteria

- [ ] Implement `SearchTask` interface and types
- [ ] Create 10-15 specific search tasks across the 5 types
- [ ] Implement automated validators for each task
- [ ] Test validators against known correct/incorrect outcomes
- [ ] Document task creation guidelines

## Technical Requirements

**Core Types**:
```typescript
// packages/cli/src/search-optimization/types.ts

export interface SearchTarget {
  type: 'file' | 'function' | 'class' | 'pattern'
  path?: string              // For file type
  name?: string              // For function/class type
  pattern?: RegExp           // For pattern type
  alternatives?: string[]    // Accept any of these
}

export interface TaskValidator {
  type: 'code_change' | 'explanation' | 'file_creation'

  // For code_change tasks
  fileChanged?: string
  containsPattern?: RegExp

  // For explanation tasks
  mentionsFiles?: string[]
  mentionsPattern?: RegExp

  // For file_creation tasks
  fileCreated?: string
  contentPattern?: RegExp
}

export interface SearchTask {
  id: string
  name: string
  description: string         // Task for agent to complete

  // What agent should find
  searchTarget: SearchTarget

  // What agent should do with it
  followUpTask: {
    type: 'code_change' | 'explanation' | 'file_creation'
    prompt: string
    validator: TaskValidator
  }

  // Context and constraints
  context?: string
  maxSearchAttempts?: number
  maxTimeSeconds?: number

  // Metadata
  difficulty: 'easy' | 'medium' | 'hard'
  category: string            // Which of the 5 types

  // Success validation
  successValidator: (result: AgentOutput) => TaskScore
}

export interface TaskScore {
  searchQuality: number       // 0-1: Did agent find target?
  taskCompletion: number      // 0-1: Did agent complete task?
  efficiency: number          // 0-1: How efficiently?
  total: number               // Composite: 0-1
  details: string             // Explanation
}

export interface AgentOutput {
  searchResults: SearchResult[]  // All search results from agent
  workResult: WorkResult         // Files changed, explanations written, etc.
  searchCount: number            // Number of searches performed
  toolCallCount: number          // Total tool calls made
  durationSeconds: number        // Time taken to complete task
}

export interface SearchResult {
  query: string
  results: any[]  // Maproom search results
  rank?: number   // Where target was found (if applicable)
}

export interface WorkResult {
  filesChanged?: string[]
  filesCreated?: string[]
  explanationText?: string
  success: boolean
}
```

**Example Tasks** (minimum 10):

```typescript
// Type 1: Finding Feature Implementation
export const TASK_FIND_WORKTREE_CREATION: SearchTask = {
  id: 'impl-worktree-001',
  name: 'Find Worktree Creation Implementation',
  description: 'Find the code that creates git worktrees in the crewchief CLI',
  searchTarget: {
    type: 'file',
    path: 'packages/cli/src/git/worktree.ts',
    alternatives: ['packages/cli/src/git/index.ts']
  },
  followUpTask: {
    type: 'explanation',
    prompt: 'Explain how worktree creation works in this codebase',
    validator: {
      type: 'explanation',
      mentionsFiles: ['packages/cli/src/git/worktree.ts'],
      mentionsPattern: /worktree|git worktree add|branch/i
    }
  },
  difficulty: 'easy',
  category: 'finding-implementation',
  maxSearchAttempts: 5,
  maxTimeSeconds: 180,
  successValidator: validateTask
}

// Type 2: Understanding Architecture
export const TASK_UNDERSTAND_COMPETITION: SearchTask = {
  id: 'arch-competition-001',
  name: 'Understand Competition Flow',
  description: 'Understand how agent competitions work end-to-end',
  searchTarget: {
    type: 'pattern',
    pattern: /CompetitionManager|competition\.ts|runCompetition/,
  },
  followUpTask: {
    type: 'explanation',
    prompt: 'Explain the competition workflow from start to winner selection',
    validator: {
      type: 'explanation',
      mentionsFiles: [
        'packages/cli/src/orchestrator/competition.ts',
        'packages/cli/src/evaluation/checks.ts'
      ],
      mentionsPattern: /spawn.*agent|evaluate|score|winner/i
    }
  },
  difficulty: 'medium',
  category: 'understanding-architecture',
  maxSearchAttempts: 8,
  maxTimeSeconds: 300,
  successValidator: validateTask
}

// Type 3: Locating Error Handling
export const TASK_FIND_ERROR_HANDLING: SearchTask = {
  id: 'error-db-001',
  name: 'Find Database Error Handling',
  description: 'Find how database connection errors are handled',
  searchTarget: {
    type: 'pattern',
    pattern: /catch.*database|PostgresError|connection.*error/i
  },
  followUpTask: {
    type: 'code_change',
    prompt: 'Add a console.log to the database error handler',
    validator: {
      type: 'code_change',
      fileChanged: 'packages/maproom-mcp/src/database.ts',
      containsPattern: /console\.log.*error/i
    }
  },
  difficulty: 'medium',
  category: 'locating-errors',
  maxSearchAttempts: 6,
  maxTimeSeconds: 240,
  successValidator: validateTask
}

// Create 7 more tasks covering all 5 types...
```

**Validation Functions**:
```typescript
// packages/cli/src/search-optimization/validators.ts

export function validateSearchQuality(
  searchResults: SearchResult[],
  target: SearchTarget
): number {
  // Check if target found in top 3, 10, 20
  // Return 1.0, 0.7, 0.4, or 0.0
}

export function validateTaskCompletion(
  agentOutput: AgentOutput,
  validator: TaskValidator
): number {
  // Check if task completed correctly
  // Return 1.0, 0.5, or 0.0
}

export function calculateEfficiency(
  searchCount: number,
  toolCallCount: number,
  durationSeconds: number
): number {
  // Calculate efficiency score
  // Return 0-1
}

export function validateTask(result: AgentOutput): TaskScore {
  const searchQuality = validateSearchQuality(
    result.searchResults,
    this.searchTarget
  )
  const taskCompletion = validateTaskCompletion(
    result.workResult,
    this.followUpTask.validator
  )
  const efficiency = calculateEfficiency(
    result.searchCount,
    result.toolCallCount,
    result.durationSeconds
  )

  const total = (
    searchQuality * 0.40 +
    taskCompletion * 0.40 +
    efficiency * 0.20
  )

  return { searchQuality, taskCompletion, efficiency, total, details: '...' }
}
```

## Implementation Notes

**Task Library Structure**:
```
packages/cli/src/search-optimization/
├── types.ts               # Core types
├── validators.ts          # Validation functions
├── tasks/
│   ├── index.ts           # Export all tasks
│   ├── implementation.ts  # Type 1 tasks
│   ├── architecture.ts    # Type 2 tasks
│   ├── errors.ts          # Type 3 tasks
│   ├── related.ts         # Type 4 tasks
│   └── config.ts          # Type 5 tasks
└── README.md              # Task creation guidelines
```

**Task Creation Guidelines** (in README.md):
- Keep tasks realistic and practical
- Clear success criteria only
- Avoid tasks that are too easy or too hard
- Test validators with known correct/incorrect outputs
- Document expected difficulty

**Phase 1 Tasks** (MVP - 10 tasks):
- 3 Finding Implementation (easy)
- 3 Locating Errors (medium)
- 2 Locating Config (easy)
- 2 Understanding Architecture (medium)

**Phase 2 Tasks** (Expansion - 5 more):
- 3 Understanding Architecture (hard)
- 2 Finding Related Code (medium)

## Dependencies

None - foundational ticket

## Risk Assessment

**Risk**: Tasks too easy - all variants succeed
**Mitigation**: Include medium and hard tasks, test difficulty calibration

**Risk**: Tasks too hard - all variants fail
**Mitigation**: Start with easy tasks, add harder ones incrementally

**Risk**: Validators have false positives/negatives
**Mitigation**: Test validators against known outcomes first

## Files/Packages Affected

- packages/cli/src/search-optimization/ (new directory)
- packages/cli/tests/search-optimization/ (new tests)

## Planning References

- Deep Thinking: `../search-tasks-analysis.md`
- Replan Analysis: `../replan-analysis.md`

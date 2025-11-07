# Search Task Library

A library of realistic search tasks with validators for agent evaluation.

## Overview

This library provides:

- **SearchTask interface** - Structured task definitions
- **Validators** - Automated success criteria checking
- **12 pre-built tasks** - Covering 5 common search scenarios
- **Task utilities** - Query, filter, and select tasks

## Task Structure

Each `SearchTask` includes:

```typescript
interface SearchTask {
  id: string // Unique identifier
  name: string // Human-readable name
  description: string // Task prompt for agent

  searchTarget: SearchTarget // What to find
  followUpTask: {
    // What to do with findings
    type: 'code_change' | 'explanation' | 'file_creation'
    prompt: string
    validator: TaskValidator
  }

  difficulty: 'easy' | 'medium' | 'hard'
  category: string // Task type category
  successValidator: Function // Automated scoring
}
```

## Task Categories

### 1. Finding Implementation (3 tasks)

Find specific code features or functions.

**Examples:**

- Find worktree creation implementation
- Find agent spawning code
- Find variant injection logic

**Difficulty:** Easy to Medium

### 2. Understanding Architecture (2 tasks)

Understand system workflows and relationships.

**Examples:**

- Understand competition flow
- Understand SDK integration

**Difficulty:** Medium

### 3. Locating Error Handling (3 tasks)

Find how errors are caught and handled.

**Examples:**

- Find CLI error handling
- Find worktree error handling
- Find SDK error handling

**Difficulty:** Medium

### 4. Finding Related Code (2 tasks)

Find code related to a specific feature.

**Examples:**

- Find related test files
- Find related type definitions

**Difficulty:** Easy to Medium

### 5. Locating Configuration (2 tasks)

Find configuration and entry points.

**Examples:**

- Find CLI entry point
- Find SDK configuration

**Difficulty:** Easy

## Usage

### Import Tasks

```typescript
import {
  ALL_TASKS,
  getTasksByCategory,
  getTasksByDifficulty,
  getRandomTasks,
  TASK_FIND_WORKTREE_CREATION,
} from './search-optimization/tasks/index.js'
```

### Get Tasks by Category

```typescript
const implTasks = getTasksByCategory('finding-implementation')
// Returns all "finding implementation" tasks
```

### Get Tasks by Difficulty

```typescript
const easyTasks = getTasksByDifficulty('easy')
const mediumTasks = getTasksByDifficulty('medium')
```

### Get Random Tasks

```typescript
const randomTask = getRandomTask()
const fiveTasks = getRandomTasks(5)
```

### Validate Agent Results

```typescript
const task = TASK_FIND_WORKTREE_CREATION

const agentOutput: AgentOutput = {
  searchResults: [...],
  workResult: {
    explanationText: 'Worktree creation uses...',
    success: true
  },
  searchCount: 2,
  toolCallCount: 10,
  durationSeconds: 60
}

const score = task.successValidator(agentOutput)
// Returns: {
//   searchQuality: 1.0,      // Found target in top 3
//   taskCompletion: 0.8,     // Good explanation
//   efficiency: 0.85,        // Efficient execution
//   total: 0.85,             // Weighted average
//   details: '...'           // Human-readable summary
// }
```

## Scoring

Each task is scored on three dimensions:

### 1. Search Quality (40% weight)

- **1.0**: Target found in top 3 results
- **0.7**: Target found in top 10 results
- **0.4**: Target found in top 20 results
- **0.0**: Target not found

### 2. Task Completion (40% weight)

- **1.0**: Task fully completed with all criteria met
- **0.5-0.8**: Partially completed
- **0.0**: Task failed

### 3. Efficiency (20% weight)

- Fewer searches (1-10 optimal)
- Fewer tool calls (5-30 optimal)
- Faster execution (30-300s optimal)

**Total Score** = 0.4 × searchQuality + 0.4 × taskCompletion + 0.2 × efficiency

## Creating New Tasks

### Task Creation Guidelines

1. **Keep tasks realistic** - Base on actual developer workflows
2. **Clear success criteria** - Validators should be objective
3. **Appropriate difficulty** - Balance between too easy and impossible
4. **Test validators first** - Verify with known correct/incorrect outputs
5. **Document expected behavior** - Add comments for complex validators

### Task Template

```typescript
export const TASK_YOUR_TASK: SearchTask = {
  id: 'category-feature-001',
  name: 'Descriptive Name',
  description: 'Task prompt for agent to execute',

  searchTarget: {
    type: 'file' | 'function' | 'class' | 'pattern',
    path: 'expected/path.ts', // for file type
    name: 'functionName', // for function/class type
    pattern: /regex pattern/, // for pattern type
    alternatives: ['alt1', 'alt2'], // optional alternatives
  },

  followUpTask: {
    type: 'explanation' | 'code_change' | 'file_creation',
    prompt: 'What agent should do after finding target',
    validator: {
      type: 'explanation',
      mentionsFiles: ['required', 'files'],
      mentionsPattern: /required.*keywords/i,
    },
  },

  difficulty: 'easy' | 'medium' | 'hard',
  category: 'task-category',
  maxSearchAttempts: 5,
  maxTimeSeconds: 180,

  successValidator: createTaskValidator({
    searchTarget: {
      /* same as above */
    },
    followUpTask: {
      validator: {
        /* same as above */
      },
    },
  }),
}
```

### Validator Types

#### Explanation Validator

```typescript
validator: {
  type: 'explanation',
  mentionsFiles: ['file1.ts', 'file2.ts'],      // Optional
  mentionsPattern: /required.*keywords/i         // Optional
}
```

#### Code Change Validator

```typescript
validator: {
  type: 'code_change',
  fileChanged: 'path/to/file.ts',
  containsPattern: /added.*code/i               // Optional
}
```

#### File Creation Validator

```typescript
validator: {
  type: 'file_creation',
  fileCreated: 'path/to/new/file.ts',
  contentPattern: /expected.*content/i          // Optional
}
```

## Testing

Run tests:

```bash
pnpm test tests/search-optimization
```

Test coverage:

- Validator functions (search quality, task completion, efficiency)
- Task library structure
- Task filtering and selection
- Score calculation

## Examples

### Easy Task - Find CLI Entry Point

```typescript
const task = TASK_FIND_CLI_ENTRY
// Agent should:
// 1. Search for "CLI entry point" or "main command"
// 2. Find packages/cli/src/cli/index.ts
// 3. Explain the command structure
```

### Medium Task - Understand Competition Flow

```typescript
const task = TASK_UNDERSTAND_COMPETITION
// Agent should:
// 1. Search for competition-related code
// 2. Find CompetitionManager and search-competition.ts
// 3. Explain the workflow from creation to winner selection
```

### Medium Task - Find Error Handling

```typescript
const task = TASK_FIND_CLI_ERROR_HANDLING
// Agent should:
// 1. Search for error handling patterns
// 2. Find try/catch blocks and error handlers
// 3. Explain the error handling strategy
```

## File Organization

```
search-optimization/
├── types.ts                    # Core type definitions
├── validators.ts               # Validation functions
├── tasks/
│   ├── index.ts               # All tasks exported
│   ├── implementation.ts      # Type 1: Finding Implementation
│   ├── architecture.ts        # Type 2: Understanding Architecture
│   ├── errors.ts              # Type 3: Locating Errors
│   ├── config.ts              # Type 5: Locating Config
│   └── related.ts             # Type 4: Finding Related
└── README.md                  # This file
```

## Future Enhancements

- Add hard difficulty tasks
- Expand to 15-20 tasks
- Add multi-step tasks
- Include negative test cases
- Add performance benchmarks
- Create task generation utilities

# Deep Thinking: Search Tasks & Success Validation

## Question 1: Top 5 Actual Search Tasks

### Analysis

For a semantic code search tool used by AI agents in Claude Code, I analyzed:
- What developers actually need when navigating codebases
- What Claude Code agents struggle with currently
- What provides the most value in real-world scenarios
- What we can validate programmatically

### The Top 5

#### 1. Finding Feature Implementation (CRITICAL)
**Why**: Most common developer task - "Where is X implemented?"

**Examples**:
- "Find authentication implementation"
- "Locate user registration code"
- "Find worktree creation logic"
- "Locate database connection setup"

**Success Criteria**: Agent finds the primary file and main function

**Why This Matters**:
- Represents 40-50% of all code navigation
- Clear success definition (found it or didn't)
- Directly maps to real user needs

---

#### 2. Understanding Architecture/Flow (HIGH VALUE)
**Why**: Critical for complex tasks - "How does X work end-to-end?"

**Examples**:
- "Understand how agents are spawned in crewchief"
- "Trace the message bus flow from send to receive"
- "Map out the MCP server request lifecycle"
- "Understand worktree creation from CLI to git"

**Success Criteria**: Agent finds 3+ related files in correct sequence

**Why This Matters**:
- Requires understanding relationships, not just keywords
- Tests semantic capabilities (not just keyword matching)
- Essential for architectural decisions and refactoring

---

#### 3. Locating Error Handling (DEBUGGING)
**Why**: Debugging is a primary use case - "Where are errors caught/handled?"

**Examples**:
- "Find database connection error handling"
- "Locate API error response logic"
- "Find where git command failures are handled"
- "Locate validation error creation"

**Success Criteria**: Agent finds error handling code (try/catch, error classes)

**Why This Matters**:
- Error handling is often scattered
- Keywords like "error" are too generic
- Semantic understanding of flow is crucial
- High-value for debugging scenarios

---

#### 4. Finding Related Code (REFACTORING)
**Why**: Dependencies and callers - "What calls/uses this?"

**Examples**:
- "Find all callers of createWorktree"
- "Locate all database query executions"
- "Find all places that spawn agents"
- "Find all MCP tool handlers"

**Success Criteria**: Agent finds 80%+ of actual call sites

**Why This Matters**:
- Critical for refactoring safety
- Tests understanding of code relationships
- Maproom has graph capabilities for this
- Clear correctness validation

---

#### 5. Locating Configuration/Entry Points (SETUP)
**Why**: Starting point for modifications - "Where is X configured/initialized?"

**Examples**:
- "Find MCP server initialization code"
- "Locate CLI command definitions"
- "Find PostgreSQL connection configuration"
- "Locate environment variable usage"

**Success Criteria**: Agent finds config files and initialization code

**Why This Matters**:
- Common when adding features
- Configuration is often non-obvious
- Tests ability to find both config and usage
- Real bottleneck for new developers

---

### Task Prioritization

**Phase 1** (MVP): Tasks 1, 3, 5
- Most concrete success criteria
- Clear right/wrong answers
- Cover 70% of use cases

**Phase 2** (Expansion): Tasks 2, 4
- More complex validation
- Require graph/relationship understanding
- Higher ceiling for optimization

---

## Question 2: Success Validation Criteria

### The Challenge

How do we programmatically validate that an agent "succeeded" at a search task without manual inspection?

### Three-Level Validation Model

#### Level 1: Search Quality (40% of score)
**Metric**: Did the agent find the target code?

**Validation**:
```typescript
interface SearchTarget {
  type: 'file' | 'function' | 'class' | 'pattern'
  path?: string          // For file type
  name?: string          // For function/class type
  pattern?: RegExp       // For pattern type
  alternatives?: string[] // Accept any of these
}

function validateSearchQuality(
  agentSearchResults: SearchResult[],
  target: SearchTarget
): boolean {
  // Check if any search result matches the target
  return agentSearchResults.some(result =>
    matchesTarget(result, target)
  )
}
```

**Scoring**:
- Found target in top 3 results: 1.0
- Found target in top 10 results: 0.7
- Found target in top 20 results: 0.4
- Didn't find target: 0.0

#### Level 2: Task Completion (40% of score)
**Metric**: Did the agent complete the follow-up task correctly?

**Validation**:
```typescript
interface TaskValidator {
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

function validateTaskCompletion(
  agentOutput: AgentWorkResult,
  validator: TaskValidator
): boolean {
  // Check if agent's work meets criteria
  switch (validator.type) {
    case 'code_change':
      return checkCodeChange(agentOutput, validator)
    case 'explanation':
      return checkExplanation(agentOutput, validator)
    case 'file_creation':
      return checkFileCreation(agentOutput, validator)
  }
}
```

**Scoring**:
- Task completed correctly: 1.0
- Task partially completed: 0.5
- Task failed: 0.0

#### Level 3: Efficiency (20% of score)
**Metric**: How efficiently did the agent search?

**Calculation**:
```typescript
function calculateEfficiency(
  searchAttempts: number,
  toolCalls: number,
  timeSeconds: number
): number {
  // Fewer searches = better
  const searchFactor = 1.0 / (1.0 + (searchAttempts - 1) * 0.3)

  // Fewer tool calls = more focused
  const focusFactor = Math.max(0, 1.0 - (toolCalls - 5) * 0.05)

  // Reasonable time (not too fast = didn't try, not too slow = struggling)
  const timeFactor = timeSeconds > 10 && timeSeconds < 300 ? 1.0 : 0.7

  return (searchFactor * 0.5) + (focusFactor * 0.3) + (timeFactor * 0.2)
}
```

**Rationale**:
- Penalize excessive searching (guessing)
- Penalize excessive tool use (flailing)
- Reasonable time bounds (too fast = gave up, too slow = struggling)

---

### Composite Score Formula

```typescript
function calculateAgentScore(
  searchQuality: number,    // 0-1
  taskCompletion: number,   // 0-1
  efficiency: number        // 0-1
): number {
  return (
    searchQuality * 0.40 +
    taskCompletion * 0.40 +
    efficiency * 0.20
  )
}
```

**Why These Weights?**
- **40% Search Quality**: Core function of tool - did it help agent find code?
- **40% Task Completion**: Ultimate goal - did agent succeed at task?
- **20% Efficiency**: Tiebreaker - prefer cleaner, faster solutions

---

### Example Task Definition

```typescript
const TASK_FIND_AUTH: SearchTask = {
  id: 'auth-001',
  description: 'Find the authentication implementation in the crewchief CLI',

  // What the agent should find
  searchTarget: {
    type: 'file',
    path: 'packages/cli/src/auth/provider.ts',
    alternatives: [
      'packages/cli/src/auth/index.ts',
      'packages/cli/src/config/auth.ts'
    ]
  },

  // What the agent should do with it
  followUpTask: {
    type: 'explanation',
    prompt: 'Explain how authentication works in this codebase',
    validator: {
      type: 'explanation',
      mentionsFiles: [
        'packages/cli/src/auth/provider.ts'
      ],
      mentionsPattern: /authentication|auth|credentials|token/i
    }
  },

  // Context for the agent
  context: 'You are working on the crewchief CLI. Find and explain the authentication system.',

  // Success criteria
  maxSearchAttempts: 5,
  maxTimeSeconds: 180,

  // How to validate
  successValidator: (result: AgentOutput) => {
    const searchQuality = validateSearchQuality(
      result.searchResults,
      TASK_FIND_AUTH.searchTarget
    )
    const taskCompletion = validateTaskCompletion(
      result.workResult,
      TASK_FIND_AUTH.followUpTask.validator
    )
    const efficiency = calculateEfficiency(
      result.searchCount,
      result.toolCallCount,
      result.durationSeconds
    )

    return calculateAgentScore(searchQuality, taskCompletion, efficiency)
  }
}
```

---

### Why This Approach Works

1. **Objective**: No manual review needed - all programmatically validated
2. **Realistic**: Mirrors actual developer workflows (find → understand → use)
3. **Granular**: Can debug why variants succeed/fail at each level
4. **Balanced**: No single metric dominates (search quality AND task completion)
5. **Extensible**: Easy to add new task types and validators

---

### Validation Confidence Levels

**High Confidence** (Use for optimization):
- Task Completion = 1.0 AND Search Quality ≥ 0.7
- Clear winner for this task

**Medium Confidence** (Needs more tasks):
- Multiple variants score similarly (within 0.1)
- Need more diverse tasks to differentiate

**Low Confidence** (Something's wrong):
- All variants score < 0.3
- Tasks might be too hard or poorly defined

---

## Implementation Strategy

### Phase 1: Simple Tasks (Week 1)
- 3 tasks with clear binary success
- File-finding only (Type: 'file')
- Validate by checking if file opened/read

### Phase 2: Complex Tasks (Week 2)
- 5 tasks with code changes required
- Validate changes with pattern matching
- Multi-file searches

### Phase 3: Architecture Tasks (Week 3)
- 3 tasks requiring understanding relationships
- Validate explanations contain key concepts
- Test graph-based search capabilities

### Metrics to Track

```typescript
interface CompetitionMetrics {
  // Per variant
  variantScores: Map<string, number>

  // Per task
  taskDifficulty: Map<string, number>  // Avg success rate

  // Overall
  averageSearchAttempts: number
  averageTimeToSuccess: number

  // Insights
  hardestTask: string
  bestVariant: string
  consistencyScore: number  // How consistent across tasks
}
```

---

## Conclusion

**Top 5 Tasks**: Implementation, Architecture, Errors, Related Code, Config
**Success Validation**: 3-level (Search Quality 40% + Task Completion 40% + Efficiency 20%)

This gives us:
✅ Objective, automated validation
✅ Realistic, developer-relevant scenarios
✅ Granular debugging of variant performance
✅ Clear path to iteration and improvement

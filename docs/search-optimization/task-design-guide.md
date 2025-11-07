# Task Design Guide: Creating Grep-Impossible Search Tasks

## Overview

This guide explains how to create high-quality search tasks that prove semantic code search provides measurable value over traditional keyword-based tools like grep. These tasks are designed to be **grep-impossible** or **grep-hard**, demonstrating clear scenarios where semantic understanding is critical.

### Purpose

The grep-impossible task framework serves three key goals:

1. **Objective Evaluation**: Measure semantic search performance through rigorous, scientific benchmarks
2. **Natural Tool Selection**: Let agents choose tools based on task characteristics, not prompt coercion
3. **Ecological Validity**: Ensure tasks reflect real-world developer workflows

### Philosophy

**Key Principle**: We're not trying to make grep look bad—we're identifying scenarios where semantic understanding provides genuine value. If a task can be solved efficiently with grep, it's not a good candidate for this framework.

**Why This Matters**: The genetic optimization experiments revealed we were measuring tool description quality, not tool utility. This framework ensures we measure what actually matters: real-world value of semantic search.

## The Six Task Categories

Each category represents a fundamental type of query where semantic understanding outperforms keyword matching.

### Category 1: Relationship Discovery

**What It Is**: Finding code relationships that require traversing the code graph, not just matching text.

**Why Grep Fails**: Grep can only find direct mentions. It cannot traverse transitive relationships, indirect dependencies, or follow call chains through multiple levels.

**Characteristics**:
- Grep difficulty: Impossible
- Search advantage: Critical
- Real-world frequency: Common

**Examples**:
```typescript
// Good: Requires transitive relationship traversal
{
  description: "Find all code that would break if we change the WorktreeManager API",
  // Requires: Finding direct callers + their dependents + potential side effects
}

// Good: Indirect call chain
{
  description: "What code paths lead from CLI command to database write?",
  // Requires: Tracing execution flow through multiple layers
}

// Bad: Direct relationship
{
  description: "Find all files that import WorktreeManager",
  // Too easy: grep 'import.*WorktreeManager' works fine
}
```

**Task Design Pattern**: Transitive Relationship Query
- Start with a concrete API or function
- Ask about indirect dependencies or impacts
- Require multi-hop traversal through code graph
- Success criteria: Found both direct and transitive relationships

**When to Use**: Code reviews (impact analysis), refactoring (dependency checking), debugging (call chain tracing)

### Category 2: Conceptual Similarity

**What It Is**: Finding implementations of the same concept that use different terminology or patterns.

**Why Grep Struggles**: Different developers implement the same concept with different keywords. Grep requires knowing all possible variations upfront.

**Characteristics**:
- Grep difficulty: Hard
- Search advantage: Significant
- Real-world frequency: Common

**Examples**:
```typescript
// Good: Multiple implementation patterns
{
  description: "Find all retry logic implementations across the codebase",
  // Includes: exponential backoff, circuit breakers, manual loops, promise retries
  // Keywords vary: "retry", "attempt", "backoff", "circuit breaker", timeout loops
}

// Good: Semantic pattern
{
  description: "Where do we implement rate limiting or throttling?",
  // Implementations: token bucket, leaky bucket, fixed window, sliding window
  // Each uses different terminology and structure
}

// Bad: Single keyword
{
  description: "Find all uses of the retry decorator",
  // Too easy: grep '@retry' finds it directly
}
```

**Task Design Pattern**: Conceptual Pattern Match
- Choose a concept with multiple implementation approaches
- Avoid obvious keywords in the description
- Require finding diverse implementations
- Success criteria: Found N different patterns (decorators, manual, library-based)

**When to Use**: Pattern discovery, consistency checks, refactoring to standardized approaches

### Category 3: Ambiguity Resolution

**What It Is**: Queries where the same words mean different things in different contexts, requiring semantic understanding to disambiguate.

**Why Grep Struggles**: Grep returns all matches without understanding context. Cannot distinguish between similar-looking but functionally different code.

**Characteristics**:
- Grep difficulty: Hard
- Search advantage: Significant
- Real-world frequency: Occasional

**Examples**:
```typescript
// Good: Context-dependent meaning
{
  description: "Find where we manage database transactions (not Git transactions)",
  // Ambiguity: Both use words like "commit", "rollback", "transaction"
  // Semantic search understands database vs version control context
}

// Good: Multiple implementation styles
{
  description: "Where is authentication enforced?",
  // Patterns: middleware, decorators, manual checks, guards
  // Grep finds all "auth" but can't tell which are actual enforcement points
}

// Bad: Unambiguous term
{
  description: "Find the PostgreSQL connection pool",
  // Too specific: grep 'PostgreSQL.*pool' works fine
}
```

**Task Design Pattern**: Multi-Pattern Aggregation
- Identify a concept with multiple implementation patterns
- Require distinguishing real implementations from false positives
- Success criteria: Found all N patterns, explained differences

**When to Use**: Understanding unfamiliar codebases, finding specific implementation types, avoiding false positives

### Category 4: Negative Space

**What It Is**: Finding code that **lacks** expected patterns—detecting absence rather than presence.

**Why Grep Fails**: Grep cannot search for what's not there. Requires understanding what **should** exist, then finding violations.

**Characteristics**:
- Grep difficulty: Impossible
- Search advantage: Critical
- Real-world frequency: Occasional

**Examples**:
```typescript
// Good: Finding missing patterns
{
  description: "Find async functions that don't have try-catch error handling",
  // Requires: Finding all async functions + finding error handling patterns + identifying gaps
}

// Good: Unprotected operations
{
  description: "Find file operations that don't check for existence before writing",
  // Requires: Finding file writes + finding existence checks + spotting missing checks
}

// Bad: Finding presence
{
  description: "Find functions without error handling",
  // Too vague: nearly impossible even for semantic search
  // Better: Be specific about operation type and expected pattern
}
```

**Task Design Pattern**: Negative Constraint
- Identify an operation type (file I/O, API calls, database queries)
- Define expected protective pattern (error handling, validation, rate limiting)
- Require finding operations lacking the pattern
- Success criteria: Found N operations without protection, no false positives

**When to Use**: Security audits, code quality checks, identifying technical debt

### Category 5: Cross-Cutting Concerns

**What It Is**: Finding functionality scattered across the codebase that serves a common purpose.

**Why Grep Struggles**: Cross-cutting concerns are distributed and context-dependent. Grep finds matches but cannot group by semantic purpose.

**Characteristics**:
- Grep difficulty: Hard
- Search advantage: Moderate
- Real-world frequency: Common

**Examples**:
```typescript
// Good: Scattered functionality
{
  description: "Find all security-relevant logging across the application",
  // Scattered: login attempts, permission checks, data access, admin actions
  // Context matters: not all logging is security-relevant
}

// Good: Distributed pattern
{
  description: "Where do we validate user input before database queries?",
  // Appears in: route handlers, GraphQL resolvers, service methods
  // Validation styles: Zod schemas, manual checks, ORM validation
}

// Bad: Centralized concern
{
  description: "Find the logging configuration",
  // Too easy: usually in one file, grep finds it directly
}
```

**Task Design Pattern**: Cross-Cutting Aggregation
- Choose a concern that appears in multiple unrelated files
- Require finding all instances across different subsystems
- Success criteria: Found N locations, categorized by subsystem

**When to Use**: Architectural understanding, consistency enforcement, feature removal

### Category 6: Architectural Understanding

**What It Is**: Understanding how data or control flows through the system at an architectural level.

**Why Grep Fails**: Grep shows individual pieces but cannot assemble the complete picture of system interactions.

**Characteristics**:
- Grep difficulty: Impossible
- Search advantage: Critical
- Real-world frequency: Common

**Examples**:
```typescript
// Good: End-to-end flow
{
  description: "How does a worktree creation request flow from CLI to completion?",
  // Requires: CLI parsing → validation → manager → git operations → file I/O → confirmation
}

// Good: Initialization sequence
{
  description: "What is the startup sequence for the orchestrator service?",
  // Requires: Understanding order of initialization, dependencies, side effects
}

// Bad: Single component
{
  description: "How does WorktreeManager create directories?",
  // Too narrow: grep can find the createDirectory method
  // Better: Ask about the complete workflow involving multiple components
}
```

**Task Design Pattern**: Architectural Flow Trace
- Choose an end-to-end workflow
- Require identifying all components in the path
- Include side effects and error paths
- Success criteria: Complete flow diagram, mentioned all key components

**When to Use**: Onboarding, debugging complex issues, planning refactoring, architectural documentation

## The Anti-Keyword Pattern

The anti-keyword pattern is a technique for making tasks grep-resistant without being artificially obscure.

### The Problem

If a task description contains obvious keywords, grep can solve it:

```typescript
// BAD: Contains obvious keywords
"Find the WorktreeManager class and its createWorktree method"
// Grep solution: grep -r "createWorktree" → finds it immediately
```

### The Solution

Replace direct terms with conceptual descriptions:

```typescript
// GOOD: Conceptual description
"Find the code responsible for managing parallel Git repository checkouts"
// Grep struggles: no obvious keyword to search for
// Semantic search: understands concept → finds WorktreeManager
```

### Anti-Keyword Tutorial

**Step 1: Identify the obvious keywords**
- What would you grep for? Write them down.
- Example: "retry", "backoff", "exponential"

**Step 2: Find the concept behind the keywords**
- What does this code actually **do**?
- Example: "automatically re-attempts failed operations with increasing delays"

**Step 3: Describe the concept without using the keywords**
- Use purpose/behavior/outcome language
- Example: "Find code that automatically tries operations again when they fail, waiting longer between each attempt"

**Step 4: Test your description**
- Can you solve this with a single grep command?
- If yes, make it more conceptual
- If no, but the task is clear, you've got it right

### Examples of Anti-Keyword Pattern

| Bad (Keyword-Heavy) | Good (Conceptual) |
|---------------------|-------------------|
| "Find retry logic with exponential backoff" | "Find code that re-attempts failed operations with increasing delays between tries" |
| "Find the authentication middleware" | "Find where we verify user identity before processing requests" |
| "Find circuit breaker implementations" | "Find code that stops calling failing services after repeated failures" |
| "Find rate limiting code" | "Find mechanisms that restrict how often operations can be performed" |
| "Find the database transaction manager" | "Find code that ensures groups of database operations succeed or fail together" |

### When NOT to Use Anti-Keyword Pattern

Don't make tasks artificially obscure:

```typescript
// BAD: Too vague
"Find the thing that does the stuff with the other thing"
// Even semantic search can't help here

// BAD: Unnatural language
"Locate the mechanisms of user identity establishment and verification"
// Sounds like a legal document, not a developer task

// GOOD: Natural but conceptual
"Find where we check if users are logged in before letting them access protected features"
// Clear, natural, but doesn't use obvious keywords like "auth middleware"
```

## Task Design Patterns

Design patterns are reusable templates for creating high-quality tasks.

### Pattern 1: Transitive Relationship Query

**Template**: "Find X that affects Y indirectly"

**Structure**:
```typescript
{
  description: "Find code that [would break / depends on / is affected by] if we change X",

  // What grep does
  grepApproach: "Search for direct mentions of X, manually check each",
  grepDifficulty: "high",

  // What semantic search does
  searchApproach: "Traverse code graph to find transitive dependencies",
  searchAdvantage: "Finds multi-hop relationships grep cannot see",

  successCriteria: {
    foundDirectCallers: true,
    foundIndirectDependents: true,
    identifiedRiskAreas: true
  }
}
```

**Examples**:
- "What would break if we change the WorktreeManager API?"
- "Find all code paths that eventually call createWorktree"
- "What features depend on the message bus, directly or indirectly?"

**When to Use**: Impact analysis, refactoring safety, dependency management

### Pattern 2: Conceptual Pattern Match

**Template**: "Find all implementations of {concept} across codebase"

**Structure**:
```typescript
{
  description: "Find all [retry / caching / validation] implementations",

  grepApproach: "Search for known keywords, miss alternate implementations",
  grepDifficulty: "medium",

  searchApproach: "Recognize the concept across different naming/patterns",
  searchAdvantage: "Finds conceptually similar code with different keywords",

  successCriteria: {
    foundPattern1: true,  // e.g., decorator-based
    foundPattern2: true,  // e.g., manual loops
    foundPattern3: true,  // e.g., library-based
    identifiedVariations: true
  }
}
```

**Examples**:
- "Find all retry logic (decorators, manual loops, promise wrappers)"
- "Where do we implement caching (Redis, in-memory, CDN)?"
- "Find authentication checks (middleware, decorators, manual)"

**When to Use**: Pattern discovery, consistency enforcement, standardization efforts

### Pattern 3: Architectural Flow Trace

**Template**: "How does {data/control} flow through the system?"

**Structure**:
```typescript
{
  description: "How does X flow from A to B?",

  grepApproach: "Find entry point, manually follow calls, easy to miss steps",
  grepDifficulty: "hard",

  searchApproach: "Assemble complete call chain with context",
  searchAdvantage: "Understands system-level interactions",

  successCriteria: {
    identifiedEntryPoint: true,
    tracedMiddleLayers: true,
    foundPersistence: true,
    coveredErrorPaths: true,
    endToEndComplete: true
  }
}
```

**Examples**:
- "How does a CLI command flow to git execution?"
- "What is the initialization sequence for the orchestrator?"
- "How does data flow from API request to database write?"

**When to Use**: Debugging, onboarding, architectural documentation, performance optimization

### Pattern 4: Negative Constraint

**Template**: "Find X that lacks Y (where Y is expected)"

**Structure**:
```typescript
{
  description: "Find [operations] that don't have [protective pattern]",

  grepApproach: "Find all operations, find all protections, manually diff",
  grepDifficulty: "impossible",

  searchApproach: "Use code graph to identify unprotected operations",
  searchAdvantage: "Can reason about absence via relationships",

  successCriteria: {
    foundAllOperations: true,
    identifiedProtectedOnes: true,
    identifiedUnprotectedOnes: true,
    noFalsePositives: true
  }
}
```

**Examples**:
- "Find async functions without try-catch error handling"
- "Find file operations without existence checks"
- "Find API endpoints without rate limiting"

**When to Use**: Security audits, quality checks, finding tech debt

### Pattern 5: Multi-Pattern Aggregation

**Template**: "Find all implementations of {concept} where implementations vary widely"

**Structure**:
```typescript
{
  description: "Where do we perform [authentication / logging / validation]?",

  grepApproach: "Search multiple keywords, miss many implementations",
  grepDifficulty: "hard",

  searchApproach: "Understand concept across different implementations",
  searchAdvantage: "Recognizes semantic similarity despite syntax differences",

  successCriteria: {
    foundImplementationStyle1: true,
    foundImplementationStyle2: true,
    foundImplementationStyle3: true,
    categorizedByStyle: true
  }
}
```

**Examples**:
- "Where do we check authentication? (middleware, decorators, manual)"
- "Find all input validation (Zod, manual, ORM-level)"
- "Where do we log errors? (logger calls, console, telemetry)"

**When to Use**: Consistency checks, refactoring preparation, architectural understanding

## Creating Objective Success Criteria

Success criteria must be objective, measurable, and deterministic.

### The Problem with Subjective Criteria

```typescript
// BAD: Subjective
validator: {
  type: 'explanation',
  criteria: "Agent provides a good explanation"
}
// Who decides what's "good"? Not reproducible.

// BAD: Vague
validator: {
  type: 'explanation',
  criteria: "Agent understands the architecture"
}
// How do we measure "understands"?
```

### The Solution: Binary Checks

```typescript
// GOOD: Objective
validator: {
  type: 'explanation',
  mentionsFiles: ['worktree-manager.ts', 'git-operations.ts'],
  mentionsPattern: /(createWorktree|initialize|clone)/i
}
// Clear: Either mentions files or doesn't. Either matches pattern or doesn't.

// GOOD: Code-based validation
validator: {
  type: 'code_change',
  modifiedFiles: ['target-file.ts'],
  addedPattern: /export function newFeature/
}
// Even more objective: Did the code change happen?
```

### Guidelines for Success Criteria

**1. Use binary checks, not scalar judgments**
- ✓ Good: "Mentions at least 3 of these 5 files"
- ✗ Bad: "Provides comprehensive coverage"

**2. Prefer code changes over explanations**
- ✓ Best: "Created file with correct structure"
- ✓ Good: "Mentions specific function names"
- ✗ Bad: "Explains the concept well"

**3. Make criteria testable by machine**
- ✓ Good: "Explanation matches regex /retry.*\d+.*attempts/i"
- ✗ Bad: "Explanation demonstrates understanding"

**4. Specify required elements explicitly**
- ✓ Good: "Must mention: WorktreeManager, createWorktree, and git.clone"
- ✗ Bad: "Must discuss the worktree creation process"

**5. Allow multiple paths to success**
```typescript
// GOOD: Either file A or B, both are correct
mentionsFiles: ['implementation-a.ts', 'implementation-b.ts'],
requireAny: true  // At least one

// GOOD: Alternative patterns
mentionsPattern: [
  /decorator.*@retry/i,
  /function.*retry.*loop/i,
  /Promise.*retry/i
]
requireAny: true
```

### Examples of Good Success Criteria

```typescript
// Example 1: File-based validation
{
  successValidator: {
    searchTarget: {
      type: 'file',
      files: ['worktree-manager.ts', 'git-operations.ts']
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['worktree-manager.ts'],
        mentionsPattern: /(create|initialize|setup).*worktree/i,
        minimumLength: 100  // Prevents trivial responses
      }
    }
  }
}

// Example 2: Pattern-based validation
{
  successValidator: {
    searchTarget: {
      type: 'pattern',
      pattern: /retry|backoff|attempt/i
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsPattern: /(exponential.*backoff|retry.*logic|attempt.*counter)/i,
        mentionsAnyOf: [
          'exponential',
          'backoff',
          'circuit',
          'timeout'
        ]
      }
    }
  }
}

// Example 3: Code change validation (most objective)
{
  successValidator: {
    searchTarget: {
      type: 'file',
      files: ['target.ts']
    },
    followUpTask: {
      validator: {
        type: 'code_change',
        modifiedFiles: ['target.ts'],
        addedPattern: /export.*function.*feature/,
        requiresTest: true
      }
    }
  }
}
```

## Validation Checklist

Before submitting a new task, verify it meets these criteria:

### 1. Grep Baseline Check
- [ ] Task defeats grep (estimated <30% success for Tier 1)
- [ ] No obvious keywords in description
- [ ] Requires semantic understanding, not just string matching
- [ ] Anti-keyword pattern applied

### 2. Success Criteria Check
- [ ] Criteria are objective (binary, measurable)
- [ ] No subjective judgments ("good", "thorough", "comprehensive")
- [ ] Machine-testable (regex, file mentions, code changes)
- [ ] Clear pass/fail determination

### 3. Ecological Validity Check
- [ ] Based on real developer scenarios
- [ ] You would actually do this task in real work
- [ ] Frequency: Daily/Weekly/Monthly (not rare edge case)
- [ ] Clear practical value

### 4. Task Description Check
- [ ] Clear and unambiguous
- [ ] No tool hints ("use semantic search")
- [ ] Natural language (not artificially obscure)
- [ ] Appropriate scope (not too broad, not too narrow)

### 5. Category Fit Check
- [ ] Task clearly fits one of the six categories
- [ ] Leverages category's characteristic strength
- [ ] Demonstrates expected grep difficulty
- [ ] Shows expected search advantage

### 6. Metadata Complete
- [ ] `expectedGrepSuccess` set (based on category)
- [ ] `expectedSearchSuccess` set
- [ ] `basedOnRealScenario` marked (if applicable)
- [ ] `category` correctly assigned
- [ ] `difficulty` calibrated (easy/medium/hard)

## Common Pitfalls and Mitigations

### Pitfall 1: Task Too Easy (Grep Succeeds)

**Symptom**: Grep success rate >60%

**Root Causes**:
- Description contains obvious keywords
- Task requires only direct file/function finding
- Single grep command solves it

**Mitigations**:
1. Apply anti-keyword pattern more aggressively
2. Require transitive relationships, not direct mentions
3. Add ambiguity that requires semantic disambiguation
4. Increase conceptual complexity

**Example Fix**:
```typescript
// Before (Too Easy)
"Find the createWorktree function in WorktreeManager"
// grep "createWorktree" → success

// After (Appropriate)
"Find the code responsible for setting up parallel Git repository checkouts"
// Requires understanding concept → finding WorktreeManager
```

### Pitfall 2: Task Too Hard (Both Tools Fail)

**Symptom**: Both grep AND search success <40%

**Root Causes**:
- Task requires knowledge outside codebase
- Success criteria too strict or ambiguous
- Task scope too broad
- Unclear what "success" means

**Mitigations**:
1. Narrow task scope to specific subsystem
2. Add context or clarifying information
3. Relax success criteria (require partial completion)
4. Split into multiple smaller tasks

**Example Fix**:
```typescript
// Before (Too Hard)
"Find all security vulnerabilities in the authentication system"
// Too vague, subjective, impossible

// After (Appropriate)
"Find authentication checks that don't validate token expiration"
// Specific, objective, achievable
```

### Pitfall 3: Insufficient Search Advantage

**Symptom**: Search only marginally better than grep (<20% improvement)

**Root Causes**:
- Task doesn't leverage semantic search strengths
- Grep is "good enough" for this task
- Search overhead outweighs benefits

**Mitigations**:
1. Redesign to emphasize relationships/concepts
2. Add indirection (transitive dependencies)
3. Require pattern recognition across variations
4. Consider moving to Tier 2 or 3 (not Tier 1)

**Example Fix**:
```typescript
// Before (Small Advantage)
"Find files that import WorktreeManager"
// Grep works pretty well, small search advantage

// After (Clear Advantage)
"Find code that would break if we change WorktreeManager's createWorktree API"
// Requires transitive dependency analysis → clear search advantage
```

### Pitfall 4: Subjective Success Criteria

**Symptom**: High variance across runs (>20% CV)

**Root Causes**:
- Criteria like "good explanation" or "thorough analysis"
- Human judgment required
- Multiple interpretations possible

**Mitigations**:
1. Convert to binary checks (mentions X? matches pattern Y?)
2. Use code changes instead of explanations
3. Specify required elements explicitly
4. Make criteria machine-testable

**Example Fix**:
```typescript
// Before (Subjective)
validator: {
  type: 'explanation',
  criteria: "Agent provides good explanation of retry logic"
}

// After (Objective)
validator: {
  type: 'explanation',
  mentionsFiles: ['message-bus.ts'],
  mentionsPattern: /(retry|attempt|backoff|timeout).*mechanism/i,
  minimumLength: 150
}
```

### Pitfall 5: Ecological Invalidity

**Symptom**: Developers say "I would never do this"

**Root Causes**:
- Synthetic task not based on real scenarios
- Artificially constructed to be hard
- Frequency: rare edge case
- No clear practical value

**Mitigations**:
1. Base tasks on actual code reviews, PRs, or debugging sessions
2. Link to real-world scenario in `basedOnRealScenario` field
3. Ensure task frequency is at least monthly
4. Verify task has clear practical benefit

**Example Fix**:
```typescript
// Before (Synthetic)
{
  description: "Find all functions with exactly 3 parameters that use async/await",
  basedOnRealScenario: false
}
// Developers: "Why would I ever need this?"

// After (Real-World)
{
  description: "Find async operations that don't have error handling",
  basedOnRealScenario: "Issue #234 - Unhandled promise rejections causing crashes",
  linkedScenario: "https://github.com/org/repo/issues/234"
}
// Developers: "Yes, I do this during security audits"
```

### Pitfall 6: Tool Coercion

**Symptom**: Task description hints at tool choice

**Root Causes**:
- Description says "use semantic search"
- Task obviously requires specific tool
- Agent is guided rather than choosing

**Mitigations**:
1. Remove all tool mentions from description
2. Make task solvable multiple ways (in theory)
3. Let difficulty emerge from task nature, not hints
4. Test: Could this be presented without tool context?

**Example Fix**:
```typescript
// Before (Coercive)
"Use semantic code search to find retry logic implementations across the codebase"
// Forces tool choice in description

// After (Neutral)
"Find all retry logic implementations across the codebase"
// Agent chooses tool based on task characteristics
```

## Task Creation Workflow

### Step 1: Identify Real Scenario

Start with an actual developer need:
- Code review question you've asked
- Debugging session where you searched for something
- Refactoring where you needed to find all instances
- Architecture documentation you had to piece together

**Document**:
- What was the actual scenario?
- What did you need to find?
- Why was it hard with grep?
- Link to PR/issue/discussion if available

### Step 2: Choose Category

Map your scenario to one of the six categories:
- Relationship Discovery → transitive dependencies, call chains
- Conceptual Similarity → same pattern, different words
- Ambiguity Resolution → context-dependent meaning
- Negative Space → finding what's missing
- Cross-Cutting Concerns → scattered functionality
- Architectural Understanding → system-level flows

**Verify**:
- Category is clear fit
- Leverages category's characteristic strength
- Demonstrates expected grep weakness

### Step 3: Write Task Description

Apply the anti-keyword pattern:
1. List obvious keywords
2. Identify the underlying concept
3. Describe concept without keywords
4. Verify grep cannot easily solve it

**Guidelines**:
- Natural language (not artificially obscure)
- Clear and specific
- No tool hints
- Appropriate scope

### Step 4: Define Success Criteria

Create objective, measurable criteria:
1. What files should be found?
2. What patterns should be identified?
3. What explanations should be given?
4. How do we measure success?

**Prefer**:
- Code changes > explanations
- Binary checks > scalar judgments
- Regex patterns > subjective assessment
- File mentions > vague descriptions

### Step 5: Set Expected Metrics

Estimate performance based on category:

**Tier 1 (Grep-Impossible)**:
- expectedGrepSuccess: 0.10 - 0.30 (10-30%)
- expectedSearchSuccess: 0.70 - 0.85 (70-85%)

**Tier 2 (Grep-Hard)**:
- expectedGrepSuccess: 0.30 - 0.60 (30-60%)
- expectedSearchSuccess: 0.70 - 0.85 (70-85%)

**Tier 3 (Real-World)**:
- expectedGrepSuccess: 0.40 - 0.80 (40-80%)
- expectedSearchSuccess: 0.60 - 0.90 (60-90%)

### Step 6: Validate Task

Run through the validation checklist:
- Grep baseline check
- Success criteria check
- Ecological validity check
- Task description check
- Category fit check
- Metadata complete

**Use the validator**:
```typescript
import { validateTask } from '../validation/task-validator.js'

const result = await validateTask({
  task: myNewTask,
  tier: 'tier1-impossible',
  useMockData: true  // Fast validation without LLM execution
})

console.log(result.passed)  // true/false
console.log(result.recommendations)  // What to improve
```

### Step 7: Iterate Based on Feedback

Review validation results and recommendations:
- Construct validity failed? → Make task harder for grep
- Discriminant validity failed? → Increase search advantage
- Ecological validity failed? → Ground in real scenario
- Reliability failed? → Make criteria more objective

**Iterate until**:
- All validation dimensions pass
- Recommendations confirm task quality
- Ready to add to benchmark suite

## Code Examples

### Complete Task Definition

```typescript
import type { SearchTask } from '../types.js'
import { createTaskValidator } from '../validators.js'

export const TASK_FIND_RETRY_LOGIC: SearchTask = {
  // Identity
  id: 'tier2-conceptual-retry',
  name: 'Find All Retry Logic Implementations',
  category: 'conceptual-similarity',
  difficulty: 'medium',

  // Task description (what agent sees)
  description:
    'Find all code that automatically re-attempts failed operations. ' +
    'This includes any mechanism that retries operations when they fail, ' +
    'such as loops with delays, exponential backoff, circuit breakers, ' +
    'or promise retry wrappers. List the files containing retry logic ' +
    'and explain the different retry patterns used.',

  // Internal notes (for documentation)
  internalNotes:
    'Grep struggles because retry logic has many forms: ' +
    'manual loops, exponential backoff calculations, promise chains, ' +
    'timeout handling. Semantic search recognizes the retry pattern ' +
    'concept regardless of syntax.',

  // What to search for
  searchTarget: {
    type: 'pattern',
    pattern: /retry|backoff|attempt|circuit.*break|timeout.*retry/i,
  },

  // Follow-up task after search
  followUpTask: {
    type: 'explanation',
    prompt:
      'Describe all retry logic patterns found. For each pattern, ' +
      'identify: 1) Type of retry mechanism, 2) Where implemented, ' +
      '3) What operations it retries.',
    validator: {
      type: 'explanation',
      mentionsFiles: ['message-bus.ts'],
      mentionsPattern: /(retry|attempt|backoff|circuit).*mechanism/i,
    },
  },

  // Constraints
  maxSearchAttempts: 10,
  maxTimeSeconds: 300,

  // Expected performance (for validation)
  expectedGrepSuccess: 0.45,  // 45% - finds some with "retry" keyword
  expectedSearchSuccess: 0.80,  // 80% - understands retry concepts

  // Validation function
  successValidator: createTaskValidator({
    searchTarget: {
      type: 'pattern',
      pattern: /retry|backoff|attempt|circuit.*break|timeout.*retry/i,
    },
    followUpTask: {
      validator: {
        type: 'explanation',
        mentionsFiles: ['message-bus.ts'],
        mentionsPattern: /(retry|attempt|backoff|circuit).*mechanism/i,
      },
    },
  }),

  // Ecological validity
  basedOnRealScenario: true,
  linkedScenario: 'Code review: Standardize retry patterns across services',
}
```

### Using the Task in Validation

```typescript
import { validateTask } from '../validation/task-validator.js'
import { TASK_FIND_RETRY_LOGIC } from '../tasks/conceptual-similarity/retry-implementations.js'

// Validate task quality (mock mode, no LLM execution)
const result = await validateTask({
  task: TASK_FIND_RETRY_LOGIC,
  tier: 'tier2-hard',
  iterations: 5,
  useMockData: true,  // Fast validation
})

if (result.passed) {
  console.log('✓ Task passed all validation criteria')
  console.log('Ready to add to benchmark suite')
} else {
  console.log('✗ Task failed validation')
  console.log('Failed dimensions:',
    Object.entries(result.dimensions)
      .filter(([_, d]) => !d.passed)
      .map(([name, _]) => name)
  )
  console.log('Recommendations:', result.recommendations)
}
```

## Next Steps

- **To validate tasks**: See [Validation Guide](./validation-guide.md)
- **To run benchmarks**: See [Benchmark Usage Guide](./benchmark-usage.md)
- **For architecture details**: See [.agents/projects/TESTDES_grep-impossible-task-design/planning/architecture.md](../../.agents/projects/TESTDES_grep-impossible-task-design/planning/architecture.md)
- **For quality strategy**: See [.agents/projects/TESTDES_grep-impossible-task-design/planning/quality-strategy.md](../../.agents/projects/TESTDES_grep-impossible-task-design/planning/quality-strategy.md)

## Contributing

When contributing new tasks:

1. Read this guide thoroughly
2. Base tasks on real scenarios
3. Apply anti-keyword pattern
4. Create objective success criteria
5. Run validation before submitting
6. Document your reasoning

**Task locations**:
- `/packages/cli/src/search-optimization/tasks/{category}/{task-name}.ts`
- Export from `/packages/cli/src/search-optimization/tasks/{category}/index.ts`
- Add to appropriate benchmark suite in `/packages/cli/src/search-optimization/benchmarks/`

**Questions?** Open an issue or discussion with the tag `search-optimization`.

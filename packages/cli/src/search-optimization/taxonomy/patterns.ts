/**
 * Pattern templates for task design.
 *
 * Each pattern shows the structure for creating tasks in a category,
 * including grep approach vs search approach and success criteria.
 */

/**
 * A pattern template for creating tasks.
 */
export interface PatternTemplate {
  /** The category this pattern belongs to */
  category: string

  /** Template string with placeholders (e.g., "Find {concept} in {scope}") */
  pattern: string

  /** Description of the pattern */
  description: string

  /** Example task using this pattern */
  example: {
    /** The task description given to the agent */
    taskDescription: string

    /** How grep would approach this task */
    grepApproach: string

    /** Expected difficulty for grep */
    grepDifficulty: 'impossible' | 'hard' | 'possible'

    /** How semantic search would approach this task */
    searchApproach: string

    /** Advantage semantic search provides */
    searchAdvantage: 'critical' | 'significant' | 'moderate'

    /** Success criteria for the task */
    successCriteria: Record<string, boolean>
  }
}

/**
 * Pattern 1: Transitive Relationship Query
 *
 * Template: "Find X that affects Y indirectly"
 */
export const TRANSITIVE_RELATIONSHIP_PATTERN: PatternTemplate = {
  category: 'relationship-discovery',
  pattern: 'Find {X} that affects {Y} indirectly',
  description: 'Find code dependencies through transitive relationships',
  example: {
    taskDescription: 'Find code that could break if we change the worktree creation API',
    grepApproach: "Search for 'createWorktree', find direct callers, manually check each for dependencies",
    grepDifficulty: 'impossible',
    searchApproach: "Query: 'worktree creation callers dependencies', uses code graph to find transitive dependencies",
    searchAdvantage: 'critical',
    successCriteria: {
      foundDirectCallers: true,
      foundIndirectDependents: true,
      identifiedBreakageRisk: true,
    },
  },
}

/**
 * Pattern 2: Conceptual Pattern Match
 *
 * Template: "Find all implementations of {concept} across codebase"
 */
export const CONCEPTUAL_PATTERN_MATCH: PatternTemplate = {
  category: 'conceptual-similarity',
  pattern: 'Find all implementations of {concept} across codebase',
  description: 'Find conceptually similar code with different naming',
  example: {
    taskDescription: 'Find all places where we retry failed operations',
    grepApproach: "Search for 'retry', but will miss exponential backoff, circuit breakers, manual loops",
    grepDifficulty: 'hard',
    searchApproach: "Query: 'retry failed operation implementation', finds conceptually similar patterns",
    searchAdvantage: 'significant',
    successCriteria: {
      foundRetryDecorators: true,
      foundExponentialBackoff: true,
      foundCircuitBreakers: true,
      foundManualLoops: true,
    },
  },
}

/**
 * Pattern 3: Architectural Flow Trace
 *
 * Template: "How does {data/control} flow through the system?"
 */
export const ARCHITECTURAL_FLOW_PATTERN: PatternTemplate = {
  category: 'architectural-understanding',
  pattern: 'How does {data/control} flow through the system?',
  description: 'Trace data or control flow through multiple components',
  example: {
    taskDescription: 'How does a git worktree request flow from CLI to creation?',
    grepApproach: 'Search for entry point, manually follow calls, easy to miss steps or get lost',
    grepDifficulty: 'impossible',
    searchApproach: "Query: 'worktree creation workflow', use context bundles to assemble complete call chain",
    searchAdvantage: 'critical',
    successCriteria: {
      identifiedEntryPoint: true,
      tracedCommandParsing: true,
      foundValidation: true,
      locatedGitExecution: true,
      completedEndToEnd: true,
    },
  },
}

/**
 * Pattern 4: Negative Constraint
 *
 * Template: "Find X that lacks Y (where Y is expected)"
 */
export const NEGATIVE_CONSTRAINT_PATTERN: PatternTemplate = {
  category: 'negative-space',
  pattern: 'Find {X} that lacks {Y} where {Y} is expected',
  description: 'Find code missing expected patterns or protections',
  example: {
    taskDescription: "Find API endpoints that don't have rate limiting",
    grepApproach: 'Find all endpoints, find all rate limit logic, manually diff to find unprotected ones',
    grepDifficulty: 'impossible',
    searchApproach: 'Query both, use code graph to identify which endpoints lack rate limiting relationships',
    searchAdvantage: 'critical',
    successCriteria: {
      foundAllEndpoints: true,
      identifiedProtected: true,
      identifiedUnprotected: true,
      noFalsePositives: true,
    },
  },
}

/**
 * Pattern 5: Multi-Pattern Aggregation
 *
 * Template: "Find all implementations of {concept} where implementations vary widely"
 */
export const MULTI_PATTERN_AGGREGATION: PatternTemplate = {
  category: 'ambiguity-resolution',
  pattern: 'Find all implementations of {concept} where implementations vary widely',
  description: 'Aggregate multiple implementation patterns of the same concept',
  example: {
    taskDescription: 'Where do we perform authentication checks?',
    grepApproach: "Search 'auth', 'authenticate', 'isLoggedIn', etc - will miss many patterns",
    grepDifficulty: 'hard',
    searchApproach: "Query: 'authentication verification', understands concept across different implementations",
    searchAdvantage: 'significant',
    successCriteria: {
      foundMiddleware: true,
      foundDecorators: true,
      foundManualChecks: true,
      foundJWTValidation: true,
      foundSessionChecks: true,
    },
  },
}

/**
 * Pattern 6: Cross-Cutting Concern Search
 *
 * Template: "Find {concern} scattered across {scope}"
 */
export const CROSS_CUTTING_CONCERN_PATTERN: PatternTemplate = {
  category: 'cross-cutting-concerns',
  pattern: 'Find {concern} scattered across {scope}',
  description: 'Find related patterns distributed across the codebase',
  example: {
    taskDescription: 'Find all error handling in async operations',
    grepApproach: "Search 'try', 'catch', 'async' - gets many false positives, hard to identify patterns",
    grepDifficulty: 'hard',
    searchApproach: "Query: 'async error handling', groups related implementations across files",
    searchAdvantage: 'moderate',
    successCriteria: {
      foundTryCatchBlocks: true,
      foundErrorCallbacks: true,
      foundPromiseRejections: true,
      identifiedPatterns: true,
    },
  },
}

/**
 * All pattern templates in the taxonomy.
 */
export const ALL_PATTERNS: PatternTemplate[] = [
  TRANSITIVE_RELATIONSHIP_PATTERN,
  CONCEPTUAL_PATTERN_MATCH,
  ARCHITECTURAL_FLOW_PATTERN,
  NEGATIVE_CONSTRAINT_PATTERN,
  MULTI_PATTERN_AGGREGATION,
  CROSS_CUTTING_CONCERN_PATTERN,
]

/**
 * Get all patterns for a specific category.
 *
 * @param categoryName - The category name to filter by
 * @returns Array of patterns for this category
 */
export function getPatternsByCategory(categoryName: string): PatternTemplate[] {
  return ALL_PATTERNS.filter((pattern) => pattern.category === categoryName)
}

/**
 * Get a pattern by its template string.
 *
 * @param pattern - The pattern template to look up
 * @returns The matching pattern template, or undefined
 */
export function getPatternByTemplate(pattern: string): PatternTemplate | undefined {
  return ALL_PATTERNS.find((p) => p.pattern === pattern)
}

/**
 * Get all patterns with a specific grep difficulty.
 *
 * @param difficulty - The grep difficulty to filter by
 * @returns Array of patterns matching the difficulty
 */
export function getPatternsByGrepDifficulty(difficulty: 'impossible' | 'hard' | 'possible'): PatternTemplate[] {
  return ALL_PATTERNS.filter((pattern) => pattern.example.grepDifficulty === difficulty)
}

/**
 * Get all patterns with a specific search advantage.
 *
 * @param advantage - The search advantage level to filter by
 * @returns Array of patterns matching the advantage level
 */
export function getPatternsBySearchAdvantage(advantage: 'critical' | 'significant' | 'moderate'): PatternTemplate[] {
  return ALL_PATTERNS.filter((pattern) => pattern.example.searchAdvantage === advantage)
}

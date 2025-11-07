/**
 * Task category definitions for search optimization taxonomy.
 *
 * Categories are based on task characteristics that predict tool performance,
 * enabling systematic creation of tasks that demonstrate semantic search value.
 */

export interface TaskCategory {
  /** Unique identifier for this category */
  name: string

  /** Clear explanation of what this category tests */
  description: string

  /** How difficult is this task for grep-based approaches */
  grepDifficulty: 'impossible' | 'hard' | 'possible' | 'easy'

  /** How much advantage does semantic search provide */
  searchAdvantage: 'critical' | 'significant' | 'moderate' | 'none'

  /** How often do developers encounter tasks in this category */
  realWorldFrequency: 'common' | 'occasional' | 'rare'

  /** Concrete examples of tasks in this category */
  exampleScenarios: string[]
}

/**
 * Category 1: Relationship Discovery
 *
 * Tasks requiring code graph traversal to find transitive dependencies,
 * call chains, or impact analysis. Grep fundamentally cannot solve these
 * as they require understanding code relationships, not just string matching.
 */
export const RELATIONSHIP_DISCOVERY: TaskCategory = {
  name: 'relationship-discovery',
  description:
    'Finding transitive dependencies, call chains, and impact analysis through code graphs. Requires understanding code relationships beyond direct string matches.',
  grepDifficulty: 'impossible',
  searchAdvantage: 'critical',
  realWorldFrequency: 'common',
  exampleScenarios: [
    'What code depends on X without importing it directly?',
    'Find all callers of this function through indirection',
    'What would break if we change this API?',
  ],
}

/**
 * Category 2: Conceptual Similarity
 *
 * Pattern matching across different naming conventions. Tasks where
 * implementations of the same concept use different terminology,
 * making keyword-based search ineffective.
 */
export const CONCEPTUAL_SIMILARITY: TaskCategory = {
  name: 'conceptual-similarity',
  description:
    'Finding conceptually similar code that uses different naming conventions or implementations. Requires understanding patterns beyond exact keyword matches.',
  grepDifficulty: 'hard',
  searchAdvantage: 'significant',
  realWorldFrequency: 'common',
  exampleScenarios: [
    'Find all retry implementations across the codebase',
    'Locate error handling patterns similar to this one',
    'Where else do we do rate limiting?',
  ],
}

/**
 * Category 3: Ambiguity Resolution
 *
 * Disambiguating multiple implementations of the same concept.
 * Grep returns too many false positives when a term has multiple
 * meanings or implementation patterns.
 */
export const AMBIGUITY_RESOLUTION: TaskCategory = {
  name: 'ambiguity-resolution',
  description:
    'Disambiguating multiple implementations or meanings of the same concept. Requires context to separate different patterns that share keywords.',
  grepDifficulty: 'hard',
  searchAdvantage: 'significant',
  realWorldFrequency: 'occasional',
  exampleScenarios: [
    'Where are database transactions managed? (ORM vs manual vs decorators)',
    'Find authentication logic (middleware vs decorators vs manual checks)',
    'How do we handle caching? (Redis vs in-memory vs CDN)',
  ],
}

/**
 * Category 4: Negative Space
 *
 * Finding absence of expected patterns. Fundamentally impossible for grep
 * as it requires first understanding what should be present, then
 * identifying where it's missing.
 */
export const NEGATIVE_SPACE: TaskCategory = {
  name: 'negative-space',
  description:
    'Finding code that lacks expected patterns or protections. Requires understanding what should be present, then identifying its absence.',
  grepDifficulty: 'impossible',
  searchAdvantage: 'critical',
  realWorldFrequency: 'occasional',
  exampleScenarios: [
    'Find code that modifies state without persistence',
    'Where do we call APIs without retry logic?',
    "What event handlers don't have error boundaries?",
  ],
}

/**
 * Category 5: Cross-Cutting Concerns
 *
 * Finding patterns scattered across the codebase in different contexts.
 * Grep can find individual instances but struggles to aggregate
 * related patterns across different files and contexts.
 */
export const CROSS_CUTTING_CONCERNS: TaskCategory = {
  name: 'cross-cutting-concerns',
  description:
    'Finding patterns scattered across the codebase in different contexts. Requires aggregating related implementations across files and modules.',
  grepDifficulty: 'hard',
  searchAdvantage: 'moderate',
  realWorldFrequency: 'common',
  exampleScenarios: [
    'Find all error handling in async operations',
    'Where do we log security events?',
    'How is authentication checked across different endpoints?',
  ],
}

/**
 * Category 6: Architectural Understanding
 *
 * Understanding system-level flows and interactions. Requires piecing
 * together multiple components and understanding their relationships,
 * which is fundamentally impossible with string matching alone.
 */
export const ARCHITECTURAL_UNDERSTANDING: TaskCategory = {
  name: 'architectural-understanding',
  description:
    'Understanding system-level data flows, initialization sequences, and component interactions. Requires piecing together multiple components and their relationships.',
  grepDifficulty: 'impossible',
  searchAdvantage: 'critical',
  realWorldFrequency: 'common',
  exampleScenarios: [
    'How does data flow from API to database?',
    "What's the initialization sequence for the application?",
    'How are background jobs scheduled and executed?',
  ],
}

/**
 * All task categories in the taxonomy.
 * Ordered by grep difficulty (impossible -> hard -> possible)
 */
export const ALL_CATEGORIES: TaskCategory[] = [
  RELATIONSHIP_DISCOVERY,
  NEGATIVE_SPACE,
  ARCHITECTURAL_UNDERSTANDING,
  CONCEPTUAL_SIMILARITY,
  AMBIGUITY_RESOLUTION,
  CROSS_CUTTING_CONCERNS,
]

/**
 * Get a category by name.
 * @param name - The category name to look up
 * @returns The matching category, or undefined if not found
 */
export function getCategoryByName(name: string): TaskCategory | undefined {
  return ALL_CATEGORIES.find((category) => category.name === name)
}

/**
 * Get all categories with the specified grep difficulty.
 * @param difficulty - The grep difficulty level to filter by
 * @returns Array of categories matching the difficulty
 */
export function getCategoriesByGrepDifficulty(difficulty: TaskCategory['grepDifficulty']): TaskCategory[] {
  return ALL_CATEGORIES.filter((category) => category.grepDifficulty === difficulty)
}

/**
 * Get all categories with the specified search advantage.
 * @param advantage - The search advantage level to filter by
 * @returns Array of categories matching the advantage level
 */
export function getCategoriesBySearchAdvantage(advantage: TaskCategory['searchAdvantage']): TaskCategory[] {
  return ALL_CATEGORIES.filter((category) => category.searchAdvantage === advantage)
}

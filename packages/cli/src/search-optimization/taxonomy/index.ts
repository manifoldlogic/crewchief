/**
 * Task Taxonomy for Search Optimization
 *
 * A systematic framework for categorizing and creating search tasks that
 * demonstrate the value of semantic code search without coercing agents.
 *
 * The taxonomy consists of:
 * - 6 task categories based on grep difficulty and search advantage
 * - Difficulty classification system (impossible/hard/possible)
 * - Pattern templates for systematic task creation
 *
 * @module taxonomy
 */

// Categories
export {
  TaskCategory,
  RELATIONSHIP_DISCOVERY,
  CONCEPTUAL_SIMILARITY,
  AMBIGUITY_RESOLUTION,
  NEGATIVE_SPACE,
  CROSS_CUTTING_CONCERNS,
  ARCHITECTURAL_UNDERSTANDING,
  ALL_CATEGORIES,
  getCategoryByName,
  getCategoriesByGrepDifficulty,
  getCategoriesBySearchAdvantage,
} from './categories.js'

// Difficulty
export {
  DifficultyLevel,
  DIFFICULTY_THRESHOLDS,
  classifyDifficulty,
  validateDifficulty,
  getDifficultyRange,
  isTooEasy,
  isTooHard,
} from './difficulty.js'

// Patterns
export {
  PatternTemplate,
  TRANSITIVE_RELATIONSHIP_PATTERN,
  CONCEPTUAL_PATTERN_MATCH,
  ARCHITECTURAL_FLOW_PATTERN,
  NEGATIVE_CONSTRAINT_PATTERN,
  MULTI_PATTERN_AGGREGATION,
  CROSS_CUTTING_CONCERN_PATTERN,
  ALL_PATTERNS,
  getPatternsByCategory,
  getPatternByTemplate,
  getPatternsByGrepDifficulty,
  getPatternsBySearchAdvantage,
} from './patterns.js'

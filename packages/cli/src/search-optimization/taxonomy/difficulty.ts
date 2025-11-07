/**
 * Difficulty classification for task taxonomy.
 *
 * Classifies tasks based on grep success rates to create systematic
 * tiers for evaluation. Thresholds will be validated empirically in Phase 2.
 */

/**
 * Difficulty level based on expected grep success rate.
 *
 * - grep-impossible: Tasks grep fundamentally cannot solve (<30% success)
 * - grep-hard: Tasks where grep is inefficient (30-60% success)
 * - grep-possible: Tasks grep can solve adequately (>60% success)
 */
export enum DifficultyLevel {
  /** Grep fundamentally cannot solve (<30% success rate) */
  GREP_IMPOSSIBLE = 'grep-impossible',

  /** Grep is inefficient or unreliable (30-60% success rate) */
  GREP_HARD = 'grep-hard',

  /** Grep can solve adequately (>60% success rate) */
  GREP_POSSIBLE = 'grep-possible',
}

/**
 * Thresholds for difficulty classification based on grep success rate.
 *
 * These thresholds are initial estimates and will be validated empirically
 * when running actual grep baselines in Phase 2.
 */
export const DIFFICULTY_THRESHOLDS = {
  /** Upper bound for grep-impossible tasks */
  IMPOSSIBLE_MAX: 0.3,

  /** Lower bound for grep-hard tasks */
  HARD_MIN: 0.3,

  /** Upper bound for grep-hard tasks */
  HARD_MAX: 0.6,

  /** Lower bound for grep-possible tasks */
  POSSIBLE_MIN: 0.6,
} as const

/**
 * Classify a task's difficulty based on measured grep success rate.
 *
 * @param grepSuccessRate - Success rate from grep baseline (0-1)
 * @returns The difficulty level classification
 *
 * @example
 * ```typescript
 * classifyDifficulty(0.2)  // DifficultyLevel.GREP_IMPOSSIBLE
 * classifyDifficulty(0.45) // DifficultyLevel.GREP_HARD
 * classifyDifficulty(0.8)  // DifficultyLevel.GREP_POSSIBLE
 * ```
 */
export function classifyDifficulty(grepSuccessRate: number): DifficultyLevel {
  if (grepSuccessRate < DIFFICULTY_THRESHOLDS.IMPOSSIBLE_MAX) {
    return DifficultyLevel.GREP_IMPOSSIBLE
  }

  if (grepSuccessRate >= DIFFICULTY_THRESHOLDS.HARD_MIN && grepSuccessRate < DIFFICULTY_THRESHOLDS.HARD_MAX) {
    return DifficultyLevel.GREP_HARD
  }

  return DifficultyLevel.GREP_POSSIBLE
}

/**
 * Validate that a task meets the expected difficulty level.
 *
 * Used during task validation to ensure grep baseline results align
 * with the task's intended difficulty classification.
 *
 * @param expectedDifficulty - The difficulty level the task should have
 * @param actualSuccessRate - Measured grep success rate (0-1)
 * @returns True if the task meets the expected difficulty
 *
 * @example
 * ```typescript
 * // Task designed to be grep-impossible should have <30% success
 * validateDifficulty(DifficultyLevel.GREP_IMPOSSIBLE, 0.25) // true
 * validateDifficulty(DifficultyLevel.GREP_IMPOSSIBLE, 0.65) // false - too easy!
 * ```
 */
export function validateDifficulty(expectedDifficulty: DifficultyLevel, actualSuccessRate: number): boolean {
  const actualDifficulty = classifyDifficulty(actualSuccessRate)
  return actualDifficulty === expectedDifficulty
}

/**
 * Get the valid success rate range for a difficulty level.
 *
 * @param difficulty - The difficulty level
 * @returns Min and max success rates for this difficulty
 */
export function getDifficultyRange(difficulty: DifficultyLevel): {
  min: number
  max: number
} {
  switch (difficulty) {
    case DifficultyLevel.GREP_IMPOSSIBLE:
      return { min: 0, max: DIFFICULTY_THRESHOLDS.IMPOSSIBLE_MAX }
    case DifficultyLevel.GREP_HARD:
      return {
        min: DIFFICULTY_THRESHOLDS.HARD_MIN,
        max: DIFFICULTY_THRESHOLDS.HARD_MAX,
      }
    case DifficultyLevel.GREP_POSSIBLE:
      return { min: DIFFICULTY_THRESHOLDS.POSSIBLE_MIN, max: 1.0 }
  }
}

/**
 * Check if a task is too easy (grep succeeds too often).
 *
 * Tasks that are too easy don't test anything meaningful - if grep
 * works fine, there's no value demonstration for semantic search.
 *
 * @param successRate - Grep baseline success rate (0-1)
 * @returns True if task is too easy to be useful
 */
export function isTooEasy(successRate: number): boolean {
  // Tasks where grep succeeds >70% of the time are too easy
  return successRate > 0.7
}

/**
 * Check if a task is too hard (even semantic search struggles).
 *
 * Tasks that are too hard may indicate the task itself is poorly
 * designed or requires capabilities beyond what search provides.
 *
 * @param grepSuccessRate - Grep baseline success rate (0-1)
 * @param searchSuccessRate - Semantic search success rate (0-1)
 * @returns True if task is too hard for both tools
 */
export function isTooHard(grepSuccessRate: number, searchSuccessRate: number): boolean {
  // If both tools fail >90% of the time, task may be flawed
  return grepSuccessRate < 0.1 && searchSuccessRate < 0.1
}

/**
 * Validation functions for search tasks
 */

import { existsSync, readFileSync } from 'fs'
import type { AgentOutput, SearchResult, SearchTarget, TaskScore, TaskValidator, WorkResult } from './types.js'

/**
 * Validate search quality based on target and results
 *
 * @param searchResults - All search results from agent
 * @param target - What the agent should have found
 * @returns Score 0-1 (1.0 = found in top 3, 0.7 = top 10, 0.4 = top 20, 0.0 = not found)
 */
export function validateSearchQuality(searchResults: SearchResult[], target: SearchTarget): number {
  // Check all search results to find where target appeared
  let bestRank: number | null = null

  for (const searchResult of searchResults) {
    if (!searchResult.results || searchResult.results.length === 0) {
      continue
    }

    // Check each result for target match
    for (let i = 0; i < searchResult.results.length; i++) {
      const result = searchResult.results[i]
      const rank = i + 1

      if (matchesTarget(result, target)) {
        if (bestRank === null || rank < bestRank) {
          bestRank = rank
        }
      }
    }
  }

  if (bestRank === null) {
    return 0.0 // Target not found
  }

  // Score based on rank
  if (bestRank <= 3) return 1.0
  if (bestRank <= 10) return 0.7
  if (bestRank <= 20) return 0.4
  return 0.0
}

/**
 * Check if a search result matches the target
 */
function matchesTarget(result: Record<string, unknown>, target: SearchTarget): boolean {
  switch (target.type) {
    case 'file': {
      // Check if file path matches
      const filePath = (result.relpath || result.path || result.file) as string | undefined
      if (!filePath) return false

      // Check main path
      if (target.path && filePath.includes(target.path)) return true

      // Check alternatives
      if (target.alternatives) {
        return target.alternatives.some((alt) => filePath.includes(alt))
      }
      return false
    }

    case 'function':
    case 'class': {
      // Check if name matches in result content or metadata
      const name = target.name
      if (!name) return false

      const content = (result.content || result.text || '') as string
      const symbolName = (result.symbol || result.name || '') as string

      return content.includes(name) || symbolName.includes(name)
    }

    case 'pattern': {
      // Check if pattern matches result content
      if (!target.pattern) return false

      const text = (result.content || result.text || '') as string
      return target.pattern.test(text)
    }

    default:
      return false
  }
}

/**
 * Validate task completion based on work result
 *
 * @param workResult - Agent's work output
 * @param validator - Task validator configuration
 * @returns Score 0-1 (1.0 = fully completed, 0.5 = partially, 0.0 = failed)
 */
export function validateTaskCompletion(workResult: WorkResult, validator: TaskValidator): number {
  if (!workResult.success) {
    return 0.0
  }

  switch (validator.type) {
    case 'code_change':
      return validateCodeChange(workResult, validator)

    case 'explanation':
      return validateExplanation(workResult, validator)

    case 'file_creation':
      return validateFileCreation(workResult, validator)

    default:
      return 0.0
  }
}

/**
 * Validate code change task
 */
function validateCodeChange(workResult: WorkResult, validator: TaskValidator): number {
  if (!validator.fileChanged) {
    return 0.0
  }

  // Check if file was changed
  const wasChanged = workResult.filesChanged?.some((file) => file.includes(validator.fileChanged!))

  if (!wasChanged) {
    return 0.0
  }

  // If pattern specified, check file content
  if (validator.containsPattern) {
    try {
      const content = readFileSync(validator.fileChanged, 'utf-8')
      if (validator.containsPattern.test(content)) {
        return 1.0 // Fully completed
      }
      return 0.5 // File changed but pattern not found
    } catch {
      return 0.5 // File changed but can't read
    }
  }

  return 1.0 // File changed
}

/**
 * Validate explanation task
 */
function validateExplanation(workResult: WorkResult, validator: TaskValidator): number {
  if (!workResult.explanationText) {
    return 0.0
  }

  let score = 0.5 // Base score for having explanation

  // Check if mentions required files
  if (validator.mentionsFiles) {
    const mentionsAll = validator.mentionsFiles.every((file) => workResult.explanationText!.includes(file))

    if (mentionsAll) {
      score = 0.8
    }
  }

  // Check if matches pattern
  if (validator.mentionsPattern) {
    if (validator.mentionsPattern.test(workResult.explanationText)) {
      score = 1.0 // Fully completed
    }
  }

  return score
}

/**
 * Validate file creation task
 */
function validateFileCreation(workResult: WorkResult, validator: TaskValidator): number {
  if (!validator.fileCreated) {
    return 0.0
  }

  // Check if file was created
  const wasCreated = workResult.filesCreated?.some((file) => file.includes(validator.fileCreated!))

  if (!wasCreated) {
    // Also check if file exists on disk
    if (!existsSync(validator.fileCreated)) {
      return 0.0
    }
  }

  // If pattern specified, check file content
  if (validator.contentPattern) {
    try {
      const content = readFileSync(validator.fileCreated, 'utf-8')
      if (validator.contentPattern.test(content)) {
        return 1.0 // Fully completed
      }
      return 0.5 // File created but pattern not found
    } catch {
      return 0.5 // File created but can't read
    }
  }

  return 1.0 // File created
}

/**
 * Calculate efficiency score based on resource usage
 *
 * @param searchCount - Number of searches performed
 * @param toolCallCount - Total tool calls made
 * @param durationSeconds - Time taken
 * @returns Score 0-1 (higher is more efficient)
 */
export function calculateEfficiency(searchCount: number, toolCallCount: number, durationSeconds: number): number {
  // Normalize each factor
  // Fewer searches is better (1-10 searches is reasonable)
  const searchScore = Math.max(0, 1 - (searchCount - 1) / 9)

  // Fewer tool calls is better (5-30 tool calls is reasonable)
  const toolCallScore = Math.max(0, 1 - (toolCallCount - 5) / 25)

  // Faster is better (30-300 seconds is reasonable)
  const timeScore = Math.max(0, 1 - (durationSeconds - 30) / 270)

  // Weighted average
  return searchScore * 0.4 + toolCallScore * 0.3 + timeScore * 0.3
}

/**
 * Create a task validator function
 *
 * @param task - The task to validate
 * @returns Validator function
 */
export function createTaskValidator(task: {
  searchTarget: SearchTarget
  followUpTask: { validator: TaskValidator }
}): (result: AgentOutput) => TaskScore {
  return (result: AgentOutput): TaskScore => {
    const searchQuality = validateSearchQuality(result.searchResults, task.searchTarget)

    const taskCompletion = validateTaskCompletion(result.workResult, task.followUpTask.validator)

    const efficiency = calculateEfficiency(result.searchCount, result.toolCallCount, result.durationSeconds)

    const total = Math.min(1, Math.max(0, searchQuality * 0.4 + taskCompletion * 0.4 + efficiency * 0.2))

    const details = [
      `Search quality: ${(searchQuality * 100).toFixed(0)}% (target ${searchQuality >= 0.7 ? 'found' : 'not found well'})`,
      `Task completion: ${(taskCompletion * 100).toFixed(0)}% (${taskCompletion >= 0.8 ? 'success' : 'needs work'})`,
      `Efficiency: ${(efficiency * 100).toFixed(0)}% (${result.searchCount} searches, ${result.toolCallCount} calls, ${result.durationSeconds}s)`,
      `Total score: ${(total * 100).toFixed(0)}%`,
    ].join(', ')

    return { searchQuality, taskCompletion, efficiency, total, details }
  }
}

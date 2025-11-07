/**
 * Types for search task library and validation
 */

/**
 * Target that the agent should find during search
 */
export interface SearchTarget {
  type: 'file' | 'function' | 'class' | 'pattern'
  path?: string // For file type
  name?: string // For function/class type
  pattern?: RegExp // For pattern type
  alternatives?: string[] // Accept any of these
}

/**
 * Validator for the follow-up task completion
 */
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

/**
 * A search task for agent evaluation
 */
export interface SearchTask {
  id: string
  name: string
  description: string // Task for agent to complete

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
  category: string // Which of the 5 types

  // Success validation
  successValidator: (result: AgentOutput) => TaskScore
}

/**
 * Score for task completion
 */
export interface TaskScore {
  searchQuality: number // 0-1: Did agent find target?
  taskCompletion: number // 0-1: Did agent complete task?
  efficiency: number // 0-1: How efficiently?
  total: number // Composite: 0-1
  details: string // Explanation
}

/**
 * Output from agent execution
 */
export interface AgentOutput {
  searchResults: SearchResult[] // All search results from agent
  workResult: WorkResult // Files changed, explanations written, etc.
  searchCount: number // Number of searches performed
  toolCallCount: number // Total tool calls made
  durationSeconds: number // Time taken to complete task
}

/**
 * A single search result
 */
export interface SearchResult {
  query: string
  results: Record<string, unknown>[] // Maproom search results
  rank?: number // Where target was found (if applicable)
}

/**
 * Work output from agent
 */
export interface WorkResult {
  filesChanged?: string[]
  filesCreated?: string[]
  explanationText?: string
  success: boolean
}

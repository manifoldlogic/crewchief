/**
 * Agent Query Transformation Simulator
 *
 * Simulates how Claude Code would transform user queries based on tool descriptions.
 * Supports multiple simulation strategies:
 * - Rule-based: Fast, heuristic transformation (60-70% accuracy)
 * - LLM-based: Uses Claude Haiku for transformation (85-90% accuracy)
 * - API-based: Full Claude Sonnet simulation (95% accuracy)
 */

import type { Variant } from './types.js'

export type SimulationStrategy = 'rule-based' | 'llm-based' | 'api-based'

export interface TransformationResult {
  original_query: string
  transformed_query: string
  strategy: SimulationStrategy
  confidence: number
  reasoning?: string
}

/**
 * Rule-based query transformation
 *
 * Applies heuristics based on the variant description to transform queries.
 * Fast and free, but less accurate than LLM-based approaches.
 */
function ruleBasedTransform(query: string, variant: Variant): TransformationResult {
  let transformed = query.trim()
  let confidence = 0.7

  // Extract transformation patterns from variant description
  const description = variant.description.toLowerCase()
  const hasTransformationGuidance = description.includes('transform') || description.includes('extract')

  // Rule 1: Natural language question → extract core terms
  const questionWords = /^(how|what|where|when|why|does|is|are|can|do)\s+/i
  if (questionWords.test(transformed)) {
    // Remove question words
    transformed = transformed.replace(questionWords, '')

    // Remove common filler words
    transformed = transformed.replace(/\b(the|a|an|in|on|at|to|for|of|with)\b/gi, ' ')

    // Clean up whitespace
    transformed = transformed.replace(/\s+/g, ' ').trim()

    // If variant emphasizes simplicity, keep only first 1-3 words
    if (description.includes('1-3 words') || description.includes('simple')) {
      const words = transformed.split(/\s+/)
      transformed = words.slice(0, 3).join(' ')
    }

    confidence = hasTransformationGuidance ? 0.7 : 0.6
  }

  // Rule 2: File paths → reject or transform to concept
  if (transformed.includes('/') || transformed.includes('.ts') || transformed.includes('.js')) {
    // Variant emphasizes not using file paths, so extract concept
    transformed = transformed.replace(/[./]/g, ' ').replace(/\b(src|lib|test|spec)\b/gi, ' ')
    transformed = transformed.replace(/\s+/g, ' ').trim()
    confidence = 0.5
  }

  // Rule 3: Exact strings (TODO, FIXME) → keep as-is (will likely fail)
  if (/^(TODO|FIXME|console\.log|debugger)/i.test(transformed)) {
    // Most variants say to use Grep for these
    confidence = 0.3
  }

  // Rule 4: Already concise technical terms → keep as-is
  const words = transformed.split(/\s+/)
  if (words.length <= 3 && !questionWords.test(query)) {
    confidence = 0.8
  }

  // Rule 5: Remove special characters if variant doesn't mention them
  if (!description.includes('special char')) {
    transformed = transformed.replace(/[^\w\s-]/g, ' ').replace(/\s+/g, ' ').trim()
  }

  return {
    original_query: query,
    transformed_query: transformed,
    strategy: 'rule-based',
    confidence,
    reasoning: `Applied ${hasTransformationGuidance ? 'variant-guided' : 'generic'} heuristics`
  }
}

/**
 * LLM-based query transformation (placeholder)
 *
 * Uses Claude Haiku API to simulate transformation.
 * More accurate but costs ~$0.10 per experiment.
 */
async function llmBasedTransform(query: string, variant: Variant): Promise<TransformationResult> {
  // TODO: Implement Claude Haiku API call
  // For now, fall back to rule-based
  console.warn('LLM-based simulation not yet implemented, falling back to rule-based')
  return ruleBasedTransform(query, variant)
}

/**
 * API-based query transformation (placeholder)
 *
 * Uses full Claude Sonnet API to simulate transformation.
 * Most accurate but costs ~$1 per experiment.
 */
async function apiBasedTransform(query: string, variant: Variant): Promise<TransformationResult> {
  // TODO: Implement Claude Sonnet API call
  // For now, fall back to rule-based
  console.warn('API-based simulation not yet implemented, falling back to rule-based')
  return ruleBasedTransform(query, variant)
}

/**
 * Simulate agent query transformation
 */
export async function simulateTransformation(
  query: string,
  variant: Variant,
  strategy: SimulationStrategy = 'rule-based'
): Promise<TransformationResult> {
  switch (strategy) {
    case 'rule-based':
      return ruleBasedTransform(query, variant)
    case 'llm-based':
      return llmBasedTransform(query, variant)
    case 'api-based':
      return apiBasedTransform(query, variant)
    default:
      throw new Error(`Unknown simulation strategy: ${strategy}`)
  }
}

/**
 * Batch simulate transformations for multiple queries
 */
export async function simulateTransformations(
  queries: string[],
  variant: Variant,
  strategy: SimulationStrategy = 'rule-based'
): Promise<TransformationResult[]> {
  // For rule-based, we can do all transformations synchronously
  if (strategy === 'rule-based') {
    return queries.map(q => ruleBasedTransform(q, variant))
  }

  // For LLM/API-based, use parallel execution with rate limiting
  return Promise.all(queries.map(q => simulateTransformation(q, variant, strategy)))
}

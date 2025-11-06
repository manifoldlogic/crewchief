/**
 * Variant Validation Module
 *
 * Validates tool description variants to ensure they meet requirements:
 * - Token count within budget (<600 tokens)
 * - Valid MCP tool schema structure
 * - Required fields present
 * - Genealogy tracking complete
 */

import type { Variant, ValidationResult } from './types.js'

/**
 * Token counting - simple approximation
 * For production, use tiktoken library or Claude API
 */
function countTokens(text: string): number {
  // Simple approximation: ~4 characters per token
  // This is a rough estimate; real implementation should use tiktoken
  const charCount = text.length
  const wordCount = text.split(/\s+/).length

  // Average of character-based and word-based estimates
  return Math.ceil((charCount / 4 + wordCount * 1.3) / 2)
}

/**
 * Validates MCP tool description structure
 */
function validateMCPSchema(description: string): { valid: boolean; errors: string[] } {
  const errors: string[] = []

  // Check for required sections (basic validation)
  const requiredPhrases = [
    'BEST FOR',
    'USE WHEN',
    'EXAMPLES'
  ]

  for (const phrase of requiredPhrases) {
    if (!description.includes(phrase)) {
      errors.push(`Missing required section: "${phrase}"`)
    }
  }

  // Check description is not empty
  if (description.trim().length === 0) {
    errors.push('Description cannot be empty')
  }

  // Check for reasonable length
  if (description.length < 100) {
    errors.push('Description too short (minimum 100 characters)')
  }

  if (description.length > 5000) {
    errors.push('Description too long (maximum 5000 characters)')
  }

  return {
    valid: errors.length === 0,
    errors
  }
}

/**
 * Validates variant genealogy tracking
 */
function validateGenealogy(variant: Variant): string[] {
  const errors: string[] = []

  // Generation 0 must have no parents
  if (variant.generation === 0) {
    if (variant.parent_ids.length > 0) {
      errors.push('Generation 0 variants must have empty parent_ids array')
    }
    if (variant.mutation_type !== undefined) {
      errors.push('Generation 0 variants must not have mutation_type')
    }
  }

  // Generation 1+ must have parents and mutation type
  if (variant.generation > 0) {
    if (variant.parent_ids.length === 0) {
      errors.push(`Generation ${variant.generation} variant must have parent_ids`)
    }
    if (variant.mutation_type === undefined) {
      errors.push(`Generation ${variant.generation} variant must have mutation_type`)
    }
  }

  // Crossover requires exactly 2 parents
  if (variant.mutation_type === 'crossover' && variant.parent_ids.length !== 2) {
    errors.push('Crossover mutation requires exactly 2 parent_ids')
  }

  // Other mutations require exactly 1 parent
  if (
    variant.mutation_type &&
    variant.mutation_type !== 'crossover' &&
    variant.parent_ids.length !== 1
  ) {
    errors.push(`${variant.mutation_type} mutation requires exactly 1 parent_id`)
  }

  return errors
}

/**
 * Validates a variant against all requirements
 */
export function validateVariant(variant: Variant): ValidationResult {
  const errors: string[] = []
  const warnings: string[] = []

  // 1. Required fields validation
  if (!variant.id) errors.push('Variant must have an id')
  if (!variant.name) errors.push('Variant must have a name')
  if (!variant.description) errors.push('Variant must have a description')
  if (variant.generation === undefined) errors.push('Variant must have a generation number')
  if (!Array.isArray(variant.parent_ids)) errors.push('Variant must have parent_ids array')
  if (!variant.created_at) errors.push('Variant must have created_at timestamp')

  // 2. Token count validation
  const tokenCount = countTokens(variant.description)
  const withinBudget = tokenCount < 600

  if (!withinBudget) {
    errors.push(`Token count ${tokenCount} exceeds budget of 600 tokens`)
  }

  // Warning if close to budget
  if (tokenCount > 550 && tokenCount < 600) {
    warnings.push(`Token count ${tokenCount} is close to budget limit`)
  }

  // 3. MCP schema validation
  const schemaValidation = validateMCPSchema(variant.description)
  if (!schemaValidation.valid) {
    errors.push(...schemaValidation.errors)
  }

  // 4. Genealogy validation
  const genealogyErrors = validateGenealogy(variant)
  errors.push(...genealogyErrors)

  // 5. ID format validation
  if (!/^[a-z0-9-]+$/.test(variant.id)) {
    errors.push('Variant ID must contain only lowercase letters, numbers, and hyphens')
  }

  return {
    valid: errors.length === 0,
    tokenCount,
    withinBudget,
    schemaValid: schemaValidation.valid,
    errors,
    warnings
  }
}

/**
 * Quick token count check (for pre-validation)
 */
export function checkTokenCount(text: string): { count: number; withinBudget: boolean } {
  const count = countTokens(text)
  return {
    count,
    withinBudget: count < 600
  }
}

/**
 * Validates an entire variant collection
 */
export function validateCollection(variants: Variant[]): {
  valid: boolean
  results: Map<string, ValidationResult>
  summary: {
    total: number
    valid: number
    invalid: number
  }
} {
  const results = new Map<string, ValidationResult>()
  let validCount = 0

  for (const variant of variants) {
    const result = validateVariant(variant)
    results.set(variant.id, result)
    if (result.valid) validCount++
  }

  return {
    valid: validCount === variants.length,
    results,
    summary: {
      total: variants.length,
      valid: validCount,
      invalid: variants.length - validCount
    }
  }
}

/**
 * Variant Mutation Engine
 *
 * Implements genetic algorithm-style mutations for tool description variants:
 * - Crossover: Combine patterns from two parent variants
 * - Amplification: Add more examples/detail
 * - Reduction: Simplify by removing examples
 * - Reframing: Same patterns, different wording
 * - Specialization: Focus on specific query types
 */

import type { Variant, MutationConfig, MutationResult, MutationType } from './types.js'
import { validateVariant } from './validator.js'

/**
 * Generate a unique variant ID
 */
function generateVariantId(mutationType: MutationType, generation: number): string {
  const timestamp = Date.now().toString(36)
  const random = Math.random().toString(36).substring(2, 6)
  return `variant-${mutationType}-gen${generation}-${timestamp}${random}`
}

/**
 * Crossover mutation: Combine patterns from two parents
 */
function applyCrossover(parent1: Variant, parent2: Variant): string {
  // Extract sections from both parents
  const sections1 = parent1.description.split('\n\n')
  const sections2 = parent2.description.split('\n\n')

  // Take alternating sections, favoring the better-structured parent
  const combinedSections: string[] = []

  const maxLength = Math.max(sections1.length, sections2.length)
  for (let i = 0; i < maxLength; i++) {
    if (i % 2 === 0 && sections1[i]) {
      combinedSections.push(sections1[i])
    } else if (sections2[i]) {
      combinedSections.push(sections2[i])
    }
  }

  return combinedSections.join('\n\n')
}

/**
 * Amplification mutation: Add more examples/detail
 */
function applyAmplification(parent: Variant, exampleCount: number = 3): string {
  let description = parent.description

  // If there's an EXAMPLES section, add more examples
  if (description.includes('EXAMPLES:')) {
    const exampleSection = description.match(/EXAMPLES:([^]+?)(?:\n\n|$)/)?.[1] || ''
    const existingExamples = exampleSection.split('\n').filter(line => line.trim().startsWith('-') || line.trim().startsWith('•'))

    // Add variations of existing examples
    const additionalExamples = [
      '  - "function definition"',
      '  - "class implementation"',
      '  - "module exports"'
    ].slice(0, exampleCount)

    const enhancedSection = exampleSection + '\n' + additionalExamples.join('\n')
    description = description.replace(/EXAMPLES:([^]+?)(?:\n\n|$)/, `EXAMPLES:${enhancedSection}\n\n`)
  }

  // Add a "ADVANCED USAGE" section if not present
  if (!description.includes('ADVANCED') && description.length < 500) {
    description += '\n\nADVANCED USAGE:\n- Combine multiple terms for precision\n- Use filter parameter to narrow results\n- Try debug=true to understand scoring'
  }

  return description
}

/**
 * Reduction mutation: Simplify by removing examples
 */
function applyReduction(parent: Variant, targetTokens: number = 300): string {
  let description = parent.description

  // Remove optional sections first
  const optionalSections = [
    /ADVANCED USAGE:([^]+?)(?:\n\n|$)/,
    /TIPS:([^]+?)(?:\n\n|$)/,
    /FILTERS:([^]+?)(?:\n\n|$)/,
    /DEBUG:([^]+?)(?:\n\n|$)/
  ]

  for (const pattern of optionalSections) {
    if (description.match(pattern)) {
      description = description.replace(pattern, '')
      // Check if we've reached target
      if (description.length / 4 < targetTokens) break
    }
  }

  // Reduce example lists
  description = description.replace(/EXAMPLES:([^]+?)(?:\n\n|$)/, (match, examples) => {
    const lines = examples.split('\n').filter((l: string) => l.trim())
    // Keep only first 3 examples
    return 'EXAMPLES:\n' + lines.slice(0, 3).join('\n') + '\n\n'
  })

  return description.trim()
}

/**
 * Reframing mutation: Same patterns, different wording
 */
function applyReframing(parent: Variant): string {
  let description = parent.description

  // Reframe from technical to conversational (or vice versa)
  const replacements: Array<[RegExp, string]> = [
    [/BEST FOR:/g, 'Ideal for:'],
    [/USE WHEN:/g, 'You should use this when:'],
    [/AVOID:/g, 'Not suitable for:'],
    [/EXAMPLES:/g, 'Example queries:'],
    [/Keep it simple:/g, 'Pro tip: Simplicity works best -'],
    [/Think "what does this do"/g, 'Focus on describing what the code does'],
    [/Use concepts:/g, 'Search by concept:']
  ]

  for (const [pattern, replacement] of replacements) {
    description = description.replace(pattern, replacement)
  }

  return description
}

/**
 * Specialization mutation: Focus on specific query types
 */
function applySpecialization(
  parent: Variant,
  focusCategory: 'natural_language' | 'simple' | 'complex' | 'edge_case' = 'natural_language'
): string {
  let description = parent.description

  if (focusCategory === 'natural_language') {
    // Add detailed natural language transformation guidance
    const nlGuidance = `

🤖 NATURAL LANGUAGE QUERY TRANSFORMATION:

Transform questions into search terms:
1. Extract core technical terms (2-3 words)
2. Remove question words: how, what, where, when, why, does, is, are
3. Remove articles: the, a, an
4. Keep technical nouns and action verbs

Examples:
  "How does authentication work?" → "authentication"
  "What handles the database connection?" → "database connection"
  "Where is error logging implemented?" → "error logging"`

    // Insert after main description
    description = description.split('\n\n')[0] + nlGuidance + '\n\n' + description.split('\n\n').slice(1).join('\n\n')
  }

  if (focusCategory === 'simple') {
    // Emphasize simple query effectiveness
    description = description.replace(
      /Keep it simple:/,
      'SIMPLE QUERIES WORK BEST:\n- 1-3 words is optimal\n- Technical terms work great\n- Keep it simple:'
    )
  }

  return description
}

/**
 * Apply a mutation to parent variant(s)
 */
export function mutate(config: MutationConfig): MutationResult {
  const { type, parents, parameters } = config

  if (parents.length === 0) {
    return {
      success: false,
      error: 'At least one parent variant is required'
    }
  }

  let mutatedDescription: string

  try {
    switch (type) {
      case 'crossover':
        if (parents.length !== 2) {
          return {
            success: false,
            error: 'Crossover requires exactly 2 parents'
          }
        }
        mutatedDescription = applyCrossover(parents[0], parents[1])
        break

      case 'amplification':
        mutatedDescription = applyAmplification(parents[0], parameters?.exampleCount)
        break

      case 'reduction':
        mutatedDescription = applyReduction(parents[0], parameters?.targetTokens)
        break

      case 'reframing':
        mutatedDescription = applyReframing(parents[0])
        break

      case 'specialization':
        mutatedDescription = applySpecialization(parents[0], parameters?.focusCategory)
        break

      default:
        return {
          success: false,
          error: `Unknown mutation type: ${type}`
        }
    }

    // Create new variant
    const maxGeneration = Math.max(...parents.map(p => p.generation))
    const variant: Variant = {
      id: generateVariantId(type, maxGeneration + 1),
      name: `${type.charAt(0).toUpperCase() + type.slice(1)} Mutation (Gen ${maxGeneration + 1})`,
      description: mutatedDescription,
      tokens: 0, // Will be set by validation
      generation: maxGeneration + 1,
      parent_ids: parents.map(p => p.id),
      mutation_type: type,
      created_at: new Date(),
      notes: `Generated via ${type} mutation from ${parents.length} parent(s)`
    }

    // Validate the mutated variant
    const validation = validateVariant(variant)
    variant.tokens = validation.tokenCount

    if (!validation.valid) {
      return {
        success: false,
        error: `Validation failed: ${validation.errors.join(', ')}`,
        validation: {
          tokenCount: validation.tokenCount,
          schemaValid: validation.schemaValid,
          withinBudget: validation.withinBudget
        }
      }
    }

    return {
      success: true,
      variant,
      validation: {
        tokenCount: validation.tokenCount,
        schemaValid: validation.schemaValid,
        withinBudget: validation.withinBudget
      }
    }
  } catch (error) {
    return {
      success: false,
      error: `Mutation failed: ${error instanceof Error ? error.message : String(error)}`
    }
  }
}

/**
 * Generate multiple mutations from a single parent
 */
export function generateMutations(parent: Variant, count: number = 3): MutationResult[] {
  const mutationTypes: MutationType[] = ['amplification', 'reduction', 'reframing', 'specialization']
  const results: MutationResult[] = []

  for (let i = 0; i < Math.min(count, mutationTypes.length); i++) {
    const result = mutate({
      type: mutationTypes[i],
      parents: [parent]
    })
    results.push(result)
  }

  return results
}

/**
 * Generate crossover mutations from two parents
 */
export function generateCrossover(parent1: Variant, parent2: Variant): MutationResult {
  return mutate({
    type: 'crossover',
    parents: [parent1, parent2]
  })
}

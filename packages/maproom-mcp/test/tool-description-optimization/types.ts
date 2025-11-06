/**
 * Variant Generation System Types
 *
 * Data structures for managing tool description variants in the
 * AI agent query optimization testing framework (AGENTOPT Phase 0).
 */

/**
 * Represents a single tool description variant for competitive testing
 */
export interface Variant {
  /** Unique identifier (e.g., "variant-a-detailed", "variant-control") */
  id: string

  /** Human-readable name for the variant */
  name: string

  /** The full tool description text (MCP tool description format) */
  description: string

  /** Token count of the description (must be <600) */
  tokens: number

  /** Generation number (0 = manual baseline, 1+ = mutation) */
  generation: number

  /** Array of parent variant IDs (empty for generation 0) */
  parent_ids: string[]

  /** Type of mutation applied (undefined for generation 0) */
  mutation_type?: MutationType

  /** Timestamp when variant was created */
  created_at: Date

  /** Optional notes about the variant's design philosophy */
  notes?: string
}

/**
 * Supported mutation types for genetic algorithm optimization
 */
export type MutationType =
  | 'crossover'        // Combine patterns from two parent variants
  | 'amplification'    // Add more examples/detail to existing patterns
  | 'reduction'        // Simplify by removing examples or detail
  | 'reframing'        // Same patterns, different wording/perspective
  | 'specialization'   // Focus on specific query types (NL vs simple)

/**
 * Configuration for mutation operations
 */
export interface MutationConfig {
  /** Type of mutation to perform */
  type: MutationType

  /** Parent variant(s) - 1 for most mutations, 2 for crossover */
  parents: Variant[]

  /** Optional parameters for mutation behavior */
  parameters?: {
    /** For amplification: number of examples to add */
    exampleCount?: number

    /** For reduction: target token count */
    targetTokens?: number

    /** For specialization: which category to focus on */
    focusCategory?: 'natural_language' | 'simple' | 'complex' | 'edge_case'
  }
}

/**
 * Result of a mutation operation
 */
export interface MutationResult {
  /** Whether the mutation was successful */
  success: boolean

  /** The generated variant (if successful) */
  variant?: Variant

  /** Error message (if unsuccessful) */
  error?: string

  /** Validation details */
  validation?: {
    tokenCount: number
    schemaValid: boolean
    withinBudget: boolean
  }
}

/**
 * Validation result for a variant
 */
export interface ValidationResult {
  /** Whether the variant passes all validation checks */
  valid: boolean

  /** Token count */
  tokenCount: number

  /** Whether token count is within budget (<600) */
  withinBudget: boolean

  /** Whether MCP schema is valid */
  schemaValid: boolean

  /** Array of validation errors (if any) */
  errors: string[]

  /** Array of validation warnings (if any) */
  warnings: string[]
}

/**
 * Metadata for the entire variant collection
 */
export interface VariantCollection {
  /** All variants in the collection */
  variants: Variant[]

  /** Statistics about the collection */
  stats: {
    total: number
    byGeneration: Record<number, number>
    byMutationType: Record<string, number>
  }

  /** Last updated timestamp */
  lastUpdated: Date
}

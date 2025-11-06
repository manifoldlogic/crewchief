/**
 * Tests for query transformation simulator
 */

import { describe, it, expect } from 'vitest'
import { simulateTransformation, simulateTransformations } from './simulator.js'
import type { Variant } from './types.js'

describe('Query Transformation Simulator', () => {
  const mockVariant: Variant = {
    id: 'test-variant',
    name: 'Test Variant',
    description: 'BEST FOR: testing. USE WHEN: testing. EXAMPLES: test queries. Transform natural language to simple terms. Use 1-3 words.',
    tokens: 100,
    generation: 0,
    parent_ids: [],
    created_at: new Date()
  }

  describe('rule-based transformation', () => {
    it('should extract core terms from natural language questions', async () => {
      const result = await simulateTransformation('How does authentication work?', mockVariant, 'rule-based')

      expect(result.original_query).toBe('How does authentication work?')
      expect(result.transformed_query).toContain('authentication')
      expect(result.strategy).toBe('rule-based')
      expect(result.confidence).toBeGreaterThan(0)
    })

    it('should remove question words and filler words', async () => {
      const result = await simulateTransformation('What is the database connection?', mockVariant, 'rule-based')

      expect(result.transformed_query).not.toContain('What')
      expect(result.transformed_query).not.toContain('is')
      expect(result.transformed_query).not.toContain('the')
      expect(result.transformed_query).toContain('database')
    })

    it('should handle file paths by extracting concepts', async () => {
      const result = await simulateTransformation('src/auth/login.ts', mockVariant, 'rule-based')

      expect(result.confidence).toBeLessThan(0.7) // Lower confidence for file paths
      expect(result.transformed_query).not.toContain('/')
    })

    it('should keep concise technical terms as-is', async () => {
      const result = await simulateTransformation('error handling', mockVariant, 'rule-based')

      expect(result.transformed_query).toBe('error handling')
      expect(result.confidence).toBeGreaterThan(0.7)
    })

    it('should handle exact strings with low confidence', async () => {
      const result = await simulateTransformation('TODO', mockVariant, 'rule-based')

      expect(result.transformed_query).toBe('TODO')
      expect(result.confidence).toBeLessThan(0.5)
    })

    it('should limit to 1-3 words when variant emphasizes simplicity', async () => {
      const result = await simulateTransformation('How does the authentication system work in the application?', mockVariant, 'rule-based')

      const wordCount = result.transformed_query.split(/\s+/).length
      expect(wordCount).toBeLessThanOrEqual(3)
    })
  })

  describe('batch transformation', () => {
    it('should transform multiple queries', async () => {
      const queries = [
        'How does auth work?',
        'error handling',
        'What is the database connection?'
      ]

      const results = await simulateTransformations(queries, mockVariant, 'rule-based')

      expect(results).toHaveLength(3)
      expect(results[0].original_query).toBe(queries[0])
      expect(results[1].original_query).toBe(queries[1])
      expect(results[2].original_query).toBe(queries[2])

      results.forEach(result => {
        expect(result.strategy).toBe('rule-based')
        expect(result.confidence).toBeGreaterThan(0)
      })
    })
  })
})

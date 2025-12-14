/**
 * Validation Messages Tests - SRCHTRN-3002
 *
 * Tests for enhanced Zod validation error messages.
 * Validates that error messages are clear, actionable, and helpful.
 */

import { describe, it, expect } from 'vitest'
import { SearchParamsSchema } from '../src/tools/search_schema.js'

describe('Validation error messages', () => {
  it('should provide clear error for empty query', () => {
    const result = SearchParamsSchema.safeParse({
      query: '',
      repo: 'crewchief',
    })

    expect(result.success).toBe(false)
    if (!result.success) {
      const queryError = result.error.errors.find((e) => e.path[0] === 'query')
      expect(queryError?.message).toContain('Query cannot be empty')
      expect(queryError?.message).toContain('search query')
    }
  })

  it('should provide clear error for invalid mode', () => {
    const result = SearchParamsSchema.safeParse({
      query: 'test',
      repo: 'crewchief',
      mode: 'invalid',
    })

    expect(result.success).toBe(false)
    if (!result.success) {
      const modeError = result.error.errors.find((e) => e.path[0] === 'mode')
      expect(modeError?.message).toContain('fts')
      expect(modeError?.message).toContain('vector')
      expect(modeError?.message).toContain('hybrid')
    }
  })

  it('should provide clear error for missing repo', () => {
    const result = SearchParamsSchema.safeParse({
      query: 'test',
    })

    expect(result.success).toBe(false)
    if (!result.success) {
      const repoError = result.error.errors.find((e) => e.path[0] === 'repo')
      expect(repoError?.message).toContain('Repository name is required')
      expect(repoError?.message).toContain('crewchief status')
    }
  })

  it('should provide clear error for limit out of range', () => {
    const result = SearchParamsSchema.safeParse({
      query: 'test',
      repo: 'crewchief',
      limit: 2000,
    })

    expect(result.success).toBe(false)
    if (!result.success) {
      const limitError = result.error.errors.find((e) => e.path[0] === 'limit')
      expect(limitError?.message).toContain('1000')
    }
  })
})

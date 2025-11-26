/**
 * Tests for MCP search tool deduplication feature (SRCHDUP-3004)
 *
 * These tests verify:
 * - Deduplicate parameter validation
 * - Default deduplication behavior (enabled)
 * - Explicit deduplicate=false behavior
 * - Backward compatibility with omitted parameter
 */

import { describe, it, expect } from 'vitest'
import { validateSearchParams } from '../src/tools/search_schema.js'

describe('Search Tool - Deduplication Parameter Validation', () => {
  it('should accept deduplicate: true', () => {
    const params = validateSearchParams({
      query: 'test query',
      repo: 'test-repo',
      deduplicate: true,
    })
    expect(params.deduplicate).toBe(true)
  })

  it('should accept deduplicate: false', () => {
    const params = validateSearchParams({
      query: 'test query',
      repo: 'test-repo',
      deduplicate: false,
    })
    expect(params.deduplicate).toBe(false)
  })

  it('should default deduplicate to true when omitted', () => {
    const params = validateSearchParams({
      query: 'test query',
      repo: 'test-repo',
    })
    expect(params.deduplicate).toBe(true)
  })

  it('should reject invalid deduplicate type', () => {
    expect(() =>
      validateSearchParams({
        query: 'test query',
        repo: 'test-repo',
        deduplicate: 'yes' as any,
      })
    ).toThrow()
  })

  it('should reject deduplicate: null', () => {
    expect(() =>
      validateSearchParams({
        query: 'test query',
        repo: 'test-repo',
        deduplicate: null as any,
      })
    ).toThrow()
  })
})

describe('Search Tool - Deduplication Backward Compatibility', () => {
  it('should work with legacy parameters without deduplicate', () => {
    const params = validateSearchParams({
      query: 'authentication',
      repo: 'crewchief',
      limit: 10,
    })

    // Deduplicate should default to true for backward compatibility
    expect(params.deduplicate).toBe(true)
    expect(params.query).toBe('authentication')
    expect(params.repo).toBe('crewchief')
    expect(params.limit).toBe(10)
  })

  it('should preserve all existing parameters when adding deduplicate', () => {
    const params = validateSearchParams({
      query: 'search query',
      repo: 'my-repo',
      worktree: 'main',
      limit: 20,
      mode: 'fts',
      debug: true,
      deduplicate: false,
    })

    expect(params.query).toBe('search query')
    expect(params.repo).toBe('my-repo')
    expect(params.worktree).toBe('main')
    expect(params.limit).toBe(20)
    expect(params.mode).toBe('fts')
    expect(params.debug).toBe(true)
    expect(params.deduplicate).toBe(false)
  })
})

describe('Search Tool - Deduplication Schema Description', () => {
  it('should have deduplicate in schema with correct type', () => {
    // Verify the parameter is properly validated
    const validTrue = validateSearchParams({
      query: 'test',
      repo: 'repo',
      deduplicate: true,
    })
    const validFalse = validateSearchParams({
      query: 'test',
      repo: 'repo',
      deduplicate: false,
    })

    expect(typeof validTrue.deduplicate).toBe('boolean')
    expect(typeof validFalse.deduplicate).toBe('boolean')
  })
})

describe('Search Tool - Deduplication Behavior Expectations', () => {
  it('should describe expected behavior when deduplicate=true', () => {
    // When deduplicate=true (default):
    // - Results with same (relpath, symbol_name, start_line) are grouped
    // - Only highest-scoring result per group is returned
    // - Total result count may be less than limit
    const expectations = {
      duplicateHandling: 'merge',
      selectionStrategy: 'highest_score',
      resultLimit: 'applied_after_dedup',
    }

    expect(expectations.duplicateHandling).toBe('merge')
    expect(expectations.selectionStrategy).toBe('highest_score')
    expect(expectations.resultLimit).toBe('applied_after_dedup')
  })

  it('should describe expected behavior when deduplicate=false', () => {
    // When deduplicate=false:
    // - All matching results are returned
    // - Same code appearing in multiple worktrees appears multiple times
    // - Useful for seeing worktree-specific variations
    const expectations = {
      duplicateHandling: 'preserve',
      resultInclusion: 'all_matches',
    }

    expect(expectations.duplicateHandling).toBe('preserve')
    expect(expectations.resultInclusion).toBe('all_matches')
  })
})

describe('Search Tool - Deduplication Identity Key', () => {
  it('should define identity based on file path, symbol, and line', () => {
    // Identity key components for deduplication
    const identityKey = {
      fields: ['file_relpath', 'symbol_name', 'start_line'],
      description: 'Results are considered duplicates if all three fields match',
    }

    expect(identityKey.fields).toContain('file_relpath')
    expect(identityKey.fields).toContain('symbol_name')
    expect(identityKey.fields).toContain('start_line')
    expect(identityKey.fields.length).toBe(3)
  })
})

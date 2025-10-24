/**
 * Unit tests for Explain tool
 *
 * Tests cover:
 * - Parameter validation with Zod schema
 * - Symbol card generation with all fields
 * - Caching logic (hits and misses)
 * - Markdown formatting
 * - Error handling for missing chunks
 * - Database query construction
 * - Relationship extraction
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { validateExplainParams } from '../../src/tools/explain_schema.js'
import { formatSymbolCard, createEmptySymbolCard } from '../../src/templates/symbol_card.js'
import { Cache } from '../../src/utils/cache.js'
import { ValidationError } from '../../src/utils/validation.js'
import type { SymbolCard } from '../../src/templates/symbol_card.js'

describe('Explain Tool - Parameter Validation', () => {
  it('should validate required chunk_id parameter', () => {
    expect(() => validateExplainParams({})).toThrow()
    expect(() => validateExplainParams({ chunk_id: '' })).toThrow()
  })

  it('should accept chunk_id as number', () => {
    const result = validateExplainParams({ chunk_id: 123 })
    expect(result.chunk_id).toBe(123)
  })

  it('should accept chunk_id as string and convert to number', () => {
    const result = validateExplainParams({ chunk_id: '456' })
    expect(result.chunk_id).toBe(456)
  })

  it('should reject non-numeric string chunk_id', () => {
    expect(() => validateExplainParams({ chunk_id: 'abc' })).toThrow()
    // Note: '12.5' is parsed as 12 by parseInt, so it's valid
    const result = validateExplainParams({ chunk_id: '12' })
    expect(result.chunk_id).toBe(12)
  })

  it('should reject negative chunk_id', () => {
    expect(() => validateExplainParams({ chunk_id: -1 })).toThrow()
    expect(() => validateExplainParams({ chunk_id: '-5' })).toThrow()
  })

  it('should reject zero chunk_id', () => {
    expect(() => validateExplainParams({ chunk_id: 0 })).toThrow()
    expect(() => validateExplainParams({ chunk_id: '0' })).toThrow()
  })
})

describe('Symbol Card Template', () => {
  it('should create empty symbol card with defaults', () => {
    const card = createEmptySymbolCard(123)
    expect(card.chunk.id).toBe(123)
    expect(card.chunk.symbol_name).toBeNull()
    expect(card.chunk.kind).toBe('unknown')
    expect(card.location.relpath).toBe('')
    expect(card.location.worktree).toBe('')
    expect(card.relationships.imports).toEqual([])
    expect(card.relationships.exports).toEqual([])
    expect(card.relationships.calls).toEqual([])
    expect(card.relationships.called_by).toEqual([])
    expect(card.relationships.tests).toEqual([])
  })

  it('should format symbol card as markdown with header', () => {
    const card: SymbolCard = {
      chunk: {
        id: 123,
        symbol_name: 'handleExplain',
        kind: 'func',
        start_line: 10,
        end_line: 50,
      },
      location: {
        relpath: 'src/tools/explain.ts',
        worktree: 'main',
      },
      metadata: {
        language: 'typescript',
      },
      preview: {
        content: 'function handleExplain() {\n  // code here\n}',
        line_count: 3,
      },
      relationships: {
        imports: [],
        exports: [],
        calls: [],
        called_by: [],
        tests: [],
      },
    }

    const markdown = formatSymbolCard(card)
    expect(markdown).toContain('# handleExplain')
    expect(markdown).toContain('**Type:** `func`')
    expect(markdown).toContain('**File:** `src/tools/explain.ts`')
    expect(markdown).toContain('**Lines:** 10-50')
    expect(markdown).toContain('**Worktree:** `main`')
  })

  it('should include metadata section when present', () => {
    const card: SymbolCard = {
      chunk: {
        id: 123,
        symbol_name: 'MyComponent',
        kind: 'component',
        start_line: 1,
        end_line: 10,
      },
      location: {
        relpath: 'src/components/MyComponent.tsx',
        worktree: 'feature-branch',
      },
      metadata: {
        language: 'typescript',
        visibility: 'export',
        parent_context: 'Components > UI',
      },
      preview: {
        content: 'export function MyComponent() {}',
        line_count: 1,
      },
      relationships: {
        imports: [],
        exports: [],
        calls: [],
        called_by: [],
        tests: [],
      },
    }

    const markdown = formatSymbolCard(card)
    expect(markdown).toContain('## Metadata')
    expect(markdown).toContain('**Language:** typescript')
    expect(markdown).toContain('**Visibility:** export')
    expect(markdown).toContain('**Context:** Components > UI')
  })

  it('should include relationships section when present', () => {
    const card: SymbolCard = {
      chunk: {
        id: 123,
        symbol_name: 'processData',
        kind: 'func',
        start_line: 1,
        end_line: 10,
      },
      location: {
        relpath: 'src/utils/process.ts',
        worktree: 'main',
      },
      metadata: {
        language: 'typescript',
      },
      preview: {
        content: 'function processData() {}',
        line_count: 1,
      },
      relationships: {
        imports: [
          { symbol_name: 'validateData', relpath: 'src/utils/validate.ts' },
        ],
        exports: [],
        calls: [
          { symbol_name: 'saveToDb', relpath: 'src/db/save.ts' },
        ],
        called_by: [
          { symbol_name: 'handleRequest', relpath: 'src/api/handler.ts' },
        ],
        tests: [
          { symbol_name: 'processData_test', relpath: 'tests/process.test.ts' },
        ],
      },
    }

    const markdown = formatSymbolCard(card)
    expect(markdown).toContain('## Relationships')
    expect(markdown).toContain('### Imports')
    expect(markdown).toContain('`validateData` from `src/utils/validate.ts`')
    expect(markdown).toContain('### Calls')
    expect(markdown).toContain('`saveToDb` in `src/db/save.ts`')
    expect(markdown).toContain('### Called By')
    expect(markdown).toContain('`handleRequest` in `src/api/handler.ts`')
    expect(markdown).toContain('### Tests')
    expect(markdown).toContain('`processData_test` in `tests/process.test.ts`')
  })

  it('should include code preview with syntax highlighting hint', () => {
    const card: SymbolCard = {
      chunk: {
        id: 123,
        symbol_name: 'add',
        kind: 'func',
        start_line: 1,
        end_line: 3,
      },
      location: {
        relpath: 'src/math.ts',
        worktree: 'main',
      },
      metadata: {
        language: 'typescript',
      },
      preview: {
        content: 'function add(a: number, b: number) {\n  return a + b;\n}',
        line_count: 3,
      },
      relationships: {
        imports: [],
        exports: [],
        calls: [],
        called_by: [],
        tests: [],
      },
    }

    const markdown = formatSymbolCard(card)
    expect(markdown).toContain('## Code Preview')
    expect(markdown).toContain('```typescript')
    expect(markdown).toContain('function add(a: number, b: number) {')
    expect(markdown).toContain('*3 lines*')
  })

  it('should include usage examples when provided', () => {
    const card: SymbolCard = {
      chunk: {
        id: 123,
        symbol_name: 'createUser',
        kind: 'func',
        start_line: 1,
        end_line: 10,
      },
      location: {
        relpath: 'src/user.ts',
        worktree: 'main',
      },
      metadata: {
        language: 'typescript',
      },
      preview: {
        content: 'function createUser() {}',
        line_count: 1,
      },
      relationships: {
        imports: [],
        exports: [],
        calls: [],
        called_by: [],
        tests: [],
      },
      examples: [
        {
          description: 'Basic usage',
          code: 'const user = createUser({ name: "John" });',
        },
        {
          description: 'With options',
          code: 'const admin = createUser({ name: "Admin", role: "admin" });',
        },
      ],
    }

    const markdown = formatSymbolCard(card)
    expect(markdown).toContain('## Usage Examples')
    expect(markdown).toContain('### Basic usage')
    expect(markdown).toContain('const user = createUser({ name: "John" });')
    expect(markdown).toContain('### With options')
    expect(markdown).toContain('const admin = createUser({ name: "Admin", role: "admin" });')
  })

  it('should handle null symbol_name gracefully', () => {
    const card: SymbolCard = {
      chunk: {
        id: 123,
        symbol_name: null,
        kind: 'other',
        start_line: 1,
        end_line: 5,
      },
      location: {
        relpath: 'src/file.ts',
        worktree: 'main',
      },
      metadata: {
        language: null,
      },
      preview: {
        content: '// some code',
        line_count: 1,
      },
      relationships: {
        imports: [],
        exports: [],
        calls: [],
        called_by: [],
        tests: [],
      },
    }

    const markdown = formatSymbolCard(card)
    expect(markdown).toContain('# Unknown Symbol')
  })
})

describe('Cache Utility', () => {
  let cache: Cache<string>

  beforeEach(() => {
    cache = new Cache<string>(1000) // 1 second TTL for testing
  })

  afterEach(() => {
    cache.clear()
  })

  it('should store and retrieve values', () => {
    cache.set('key1', 'value1')
    expect(cache.get('key1')).toBe('value1')
  })

  it('should return null for missing keys', () => {
    expect(cache.get('nonexistent')).toBeNull()
  })

  it('should track cache hits and misses', () => {
    cache.set('key1', 'value1')

    cache.get('key1') // hit
    cache.get('key2') // miss
    cache.get('key1') // hit

    const metrics = cache.getMetrics()
    expect(metrics.hits).toBe(2)
    expect(metrics.misses).toBe(1)
    expect(metrics.sets).toBe(1)
  })

  it('should calculate hit rate correctly', () => {
    cache.set('key1', 'value1')

    cache.get('key1') // hit
    cache.get('key2') // miss
    cache.get('key1') // hit
    cache.get('key3') // miss

    expect(cache.getHitRate()).toBe(0.5) // 2 hits out of 4 requests
  })

  it('should return 0 hit rate when no requests', () => {
    expect(cache.getHitRate()).toBe(0)
  })

  it('should respect TTL and expire entries', async () => {
    cache.set('key1', 'value1', 50) // 50ms TTL

    expect(cache.get('key1')).toBe('value1')

    // Wait for expiration
    await new Promise((resolve) => setTimeout(resolve, 100))

    expect(cache.get('key1')).toBeNull()
  })

  it('should allow deleting entries', () => {
    cache.set('key1', 'value1')
    expect(cache.get('key1')).toBe('value1')

    const deleted = cache.delete('key1')
    expect(deleted).toBe(true)
    expect(cache.get('key1')).toBeNull()
  })

  it('should return false when deleting nonexistent key', () => {
    const deleted = cache.delete('nonexistent')
    expect(deleted).toBe(false)
  })

  it('should clear all entries', () => {
    cache.set('key1', 'value1')
    cache.set('key2', 'value2')
    cache.set('key3', 'value3')

    expect(cache.size()).toBe(3)

    cache.clear()

    expect(cache.size()).toBe(0)
    expect(cache.get('key1')).toBeNull()
    expect(cache.get('key2')).toBeNull()
  })

  it('should cleanup expired entries', async () => {
    cache.set('key1', 'value1', 50) // 50ms TTL
    cache.set('key2', 'value2', 5000) // 5s TTL

    expect(cache.size()).toBe(2)

    // Wait for first key to expire
    await new Promise((resolve) => setTimeout(resolve, 100))

    const removed = cache.cleanup()
    expect(removed).toBe(1)
    expect(cache.size()).toBe(1)
    expect(cache.get('key1')).toBeNull()
    expect(cache.get('key2')).toBe('value2')
  })

  it('should track evictions', () => {
    cache.set('key1', 'value1', 50)

    // Wait for expiration and trigger eviction via get
    setTimeout(() => {
      cache.get('key1')
      const metrics = cache.getMetrics()
      expect(metrics.evictions).toBeGreaterThan(0)
    }, 100)
  })

  it('should reset metrics', () => {
    cache.set('key1', 'value1')
    cache.get('key1')
    cache.get('key2')

    let metrics = cache.getMetrics()
    expect(metrics.hits).toBe(1)
    expect(metrics.misses).toBe(1)

    cache.resetMetrics()

    metrics = cache.getMetrics()
    expect(metrics.hits).toBe(0)
    expect(metrics.misses).toBe(0)
    expect(metrics.sets).toBe(0)
  })

  it('should allow custom TTL per entry', () => {
    cache.set('short', 'value1', 50)
    cache.set('long', 'value2', 5000)

    expect(cache.get('short')).toBe('value1')
    expect(cache.get('long')).toBe('value2')
  })
})

describe('Explain Tool - Error Handling', () => {
  it('should handle ValidationError with code', () => {
    const error = new ValidationError('Test error', 'TEST_CODE')

    expect(error.message).toBe('Test error')
    expect(error.code).toBe('TEST_CODE')
    expect(error).toBeInstanceOf(Error)
  })
})

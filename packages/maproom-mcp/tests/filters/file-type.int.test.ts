import { describe, it, expect } from 'vitest'
import { buildFilterClauses } from '../../src/index.js'

describe('buildFilterClauses - File Type Integration Tests', () => {
  // Single extension SQL (P0)
  it('generates correct SQL for single extension', () => {
    const args: any[] = [1] // repoId
    const filters = { file_type: 'ts' }
    const clauses = buildFilterClauses(filters, 'all', args)

    expect(clauses).toContain("f.relpath LIKE $2")
    expect(args).toContain('%.ts')
    expect(args.length).toBe(2)
  })

  // Multi-extension SQL (P0)
  it('generates correct OR clause for multiple extensions', () => {
    const args: any[] = [1]
    const filters = { file_type: 'ts,tsx,js' }
    const clauses = buildFilterClauses(filters, 'all', args)

    expect(clauses).toContain('(f.relpath LIKE')
    expect(clauses).toContain(' OR ')
    expect(args).toContain('%.ts')
    expect(args).toContain('%.tsx')
    expect(args).toContain('%.js')
    expect(args.length).toBe(4) // repoId + 3 extensions
  })

  // Parameterization (P0)
  it('uses parameterized queries (SQL injection safe)', () => {
    const args: any[] = [1]
    const filters = { file_type: "ts'; DROP TABLE files; --" }
    const clauses = buildFilterClauses(filters, 'all', args)

    // Should NOT contain the malicious string directly in SQL
    expect(clauses).not.toContain("DROP TABLE")
    // Should use parameter placeholders
    expect(clauses).toContain("$")
  })

  // Empty filter (P0)
  it('handles empty file_type gracefully', () => {
    const args: any[] = [1]
    const filters = { file_type: '' }
    const clausesBefore = args.length
    const clauses = buildFilterClauses(filters, 'all', args)

    // No filter added, args unchanged
    expect(args.length).toBe(clausesBefore)
    expect(clauses).not.toContain('f.relpath LIKE')
  })

  // Filter combination - recency (P1)
  it('combines file_type with recency_threshold', () => {
    const args: any[] = [1]
    const filters = {
      file_type: 'ts',
      recency_threshold: '7 days'
    }
    const clauses = buildFilterClauses(filters, 'all', args)

    expect(clauses).toContain('f.relpath LIKE')
    expect(clauses).toContain('f.last_modified >')
    expect(args.length).toBe(3) // repoId, file_type, recency
  })

  // Filter combination - repo_id (P1)
  it('combines file_type with repo_id', () => {
    const args: any[] = [1]
    const filters = {
      file_type: 'ts',
      repo_id: 42
    }
    const clauses = buildFilterClauses(filters, 'all', args)

    expect(clauses).toContain('f.relpath LIKE')
    expect(clauses).toContain('f.repo_id =')
    expect(args.length).toBe(3) // initial repoId + file_type + repo_id filter
  })

  // Legacy filter coexistence (P1)
  it('works with legacy filter parameter', () => {
    const args: any[] = [1]
    const filters = { file_type: 'ts' }
    const clauses = buildFilterClauses(filters, 'code', args)

    // Both filters applied
    expect(clauses).toContain('f.relpath LIKE') // file_type
    expect(clauses).toContain("NOT LIKE '%.md'") // legacy "code" filter
  })

  // Case handling (P0)
  it('normalizes case before SQL generation', () => {
    const args: any[] = [1]
    const filters = { file_type: 'TS,TSX' }
    const clauses = buildFilterClauses(filters, 'all', args)

    expect(args).toContain('%.ts') // Lowercased
    expect(args).toContain('%.tsx')
    expect(args).not.toContain('%.TS') // Not uppercase
  })

  // Limit enforcement (P1)
  it('truncates >20 extensions gracefully', () => {
    const args: any[] = [1]
    const twentyOne = Array(21).fill('ts').map((_, i) => `ext${i}`).join(',')
    const filters = { file_type: twentyOne }
    const clauses = buildFilterClauses(filters, 'all', args)

    // Should truncate to 20 (or handle gracefully)
    // args.length = repoId (1) + extensions (max 20) = max 21
    expect(args.length).toBeLessThanOrEqual(21)
  })

  // Whitespace/dot handling (P1)
  it('handles dots and whitespace before SQL', () => {
    const args: any[] = [1]
    const filters = { file_type: '  .ts , .tsx  ' }
    const clauses = buildFilterClauses(filters, 'all', args)

    expect(args).toContain('%.ts')
    expect(args).toContain('%.tsx')
    expect(args).not.toContain('%..ts') // No double dot
  })
})

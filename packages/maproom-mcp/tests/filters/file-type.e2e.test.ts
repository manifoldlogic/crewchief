import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import { Client } from 'pg'
import { handleSearch } from '../../src/index.js'

let testDb: Client
let dbAvailable = false

beforeAll(async () => {
  // Setup test database connection
  const connectionString = process.env.TEST_DATABASE_URL || process.env.MAPROOM_DATABASE_URL
  if (!connectionString) {
    console.log('❌ No TEST_DATABASE_URL or MAPROOM_DATABASE_URL set, skipping E2E tests')
    return
  }

  console.log('✓ Found database URL, attempting connection...')

  try {
    testDb = new Client({ connectionString })
    await testDb.connect()
    console.log('✓ Connected to database')

    // Verify database has data
    const result = await testDb.query('SELECT COUNT(*) as count FROM maproom.files LIMIT 1')
    const fileCount = parseInt(result.rows[0].count)
    console.log(`✓ Database has ${fileCount} files`)

    if (fileCount > 0) {
      dbAvailable = true
      console.log('✓ Database ready for E2E tests')
    } else {
      console.log('❌ Database has no files, skipping E2E tests')
    }
  } catch (err) {
    console.log('❌ Failed to connect to database for E2E tests:', err.message)
    dbAvailable = false
  }
})

afterAll(async () => {
  if (testDb) {
    await testDb.end()
  }
})

describe('File Type Filter - E2E Tests', () => {
  // Single extension (P0)
  it('returns only TypeScript files for file_type=ts', async () => {
    if (!dbAvailable) {
      console.log('Skipping: database not available')
      return
    }
    const result = await handleSearch({
      repo: 'crewchief',
      worktree: 'main',
      query: 'function',
      filters: { file_type: 'ts' },
      k: 20
    })

    expect(result.hits).toBeDefined()
    expect(Array.isArray(result.hits)).toBe(true)

    // All results should be .ts files (not .tsx or other extensions)
    if (result.hits.length > 0) {
      expect(result.hits.every(hit => hit.relpath.endsWith('.ts'))).toBe(true)
      expect(result.hits.every(hit => !hit.relpath.endsWith('.tsx'))).toBe(true)
      expect(result.hits.every(hit => !hit.relpath.endsWith('.md'))).toBe(true)
    }
  })

  // Multi-extension (P0)
  it('returns union of multiple file types', async () => {
    if (!dbAvailable) {
      console.log('Skipping: database not available')
      return
    }
    const result = await handleSearch({
      repo: 'crewchief',
      worktree: 'main',
      query: 'import',
      filters: { file_type: 'ts,tsx,js' },
      k: 30
    })

    expect(result.hits).toBeDefined()
    expect(Array.isArray(result.hits)).toBe(true)

    if (result.hits.length > 0) {
      const extensions = result.hits.map(hit => {
        const parts = hit.relpath.split('.')
        return parts[parts.length - 1]
      })

      // Should have at least one of the requested types
      const hasRequestedType = extensions.some(ext =>
        ext === 'ts' || ext === 'tsx' || ext === 'js'
      )
      expect(hasRequestedType).toBe(true)

      // Should NOT have unrequested types like .md or .json
      expect(extensions.every(ext =>
        ext === 'ts' || ext === 'tsx' || ext === 'js'
      )).toBe(true)
    }
  })

  // Case insensitive (P0)
  it('handles uppercase extensions same as lowercase', async () => {
    if (!dbAvailable) {
      console.log('Skipping: database not available')
      return
    }
    const lower = await handleSearch({
      repo: 'crewchief',
      worktree: 'main',
      query: 'export',
      filters: { file_type: 'ts' },
      k: 10
    })

    const upper = await handleSearch({
      repo: 'crewchief',
      worktree: 'main',
      query: 'export',
      filters: { file_type: 'TS' },
      k: 10
    })

    expect(lower.hits).toBeDefined()
    expect(upper.hits).toBeDefined()

    // Should return same results regardless of case
    if (lower.hits.length > 0 && upper.hits.length > 0) {
      expect(lower.hits.length).toBe(upper.hits.length)

      const lowerIds = lower.hits.map(h => h.chunk_id).sort()
      const upperIds = upper.hits.map(h => h.chunk_id).sort()
      expect(lowerIds).toEqual(upperIds)
    }
  })

  // Empty filter (P0)
  it('returns all file types when file_type is empty', async () => {
    if (!dbAvailable) {
      console.log('Skipping: database not available')
      return
    }
    const withFilter = await handleSearch({
      repo: 'crewchief',
      worktree: 'main',
      query: 'class',
      filters: { file_type: 'ts' },
      k: 10
    })

    const withoutFilter = await handleSearch({
      repo: 'crewchief',
      worktree: 'main',
      query: 'class',
      filters: { file_type: '' },
      k: 10
    })

    expect(withFilter.hits).toBeDefined()
    expect(withoutFilter.hits).toBeDefined()

    // Empty filter should return at least as many results as filtered
    // (could return more because it includes other file types)
    if (withFilter.hits.length > 0) {
      expect(withoutFilter.hits.length).toBeGreaterThanOrEqual(withFilter.hits.length)
    }
  })

  // Performance (P2)
  it('completes search with filter in <200ms', async () => {
    if (!dbAvailable) {
      console.log('Skipping: database not available')
      return
    }
    const start = Date.now()

    await handleSearch({
      repo: 'crewchief',
      worktree: 'main',
      query: 'function',
      filters: { file_type: 'ts,tsx,js' },
      k: 10
    })

    const duration = Date.now() - start
    expect(duration).toBeLessThan(200)
  })
})

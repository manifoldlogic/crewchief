import { describe, it, expect, beforeEach } from 'vitest'
import { validateMaproomEnvironment } from '../../src/cli/maproom-validation.js'

describe('validateMaproomEnvironment', () => {
  beforeEach(() => {
    // Clean environment before each test to prevent cross-contamination
    delete process.env.MAPROOM_DATABASE_URL
    delete process.env.MAPROOM_DB_HOST
    delete process.env.PG_DATABASE_URL
    delete process.env.DATABASE_URL
    delete process.env.MAPROOM_EMBEDDING_PROVIDER
    delete process.env.OPENAI_API_KEY
    delete process.env.MAPROOM_OPENAI_API_KEY
    delete process.env.GOOGLE_PROJECT_ID
    delete process.env.MAPROOM_GOOGLE_PROJECT_ID
  })

  it('returns valid when MAPROOM_DATABASE_URL is set', () => {
    process.env.MAPROOM_DATABASE_URL = 'postgresql://localhost/test'
    const result = validateMaproomEnvironment()
    expect(result.valid).toBe(true)
    expect(result.errors).toHaveLength(0)
    expect(result.warnings).toHaveLength(1) // Warning about missing provider
  })

  it('returns valid when no database URL is set (uses default ~/.maproom/maproom.db)', () => {
    // All DB env vars unset - should use default ~/.maproom/maproom.db
    const result = validateMaproomEnvironment()
    expect(result.valid).toBe(true)
    expect(result.errors).toHaveLength(0)
    // Only warning should be about missing embedding provider
    expect(result.warnings).toHaveLength(1)
    expect(result.warnings[0]).toContain('MAPROOM_EMBEDDING_PROVIDER')
  })

  it('returns warning when MAPROOM_EMBEDDING_PROVIDER not set', () => {
    process.env.MAPROOM_DATABASE_URL = 'postgresql://localhost/test'
    const result = validateMaproomEnvironment()
    expect(result.valid).toBe(true)
    expect(result.warnings.length).toBeGreaterThan(0)
    expect(result.warnings[0]).toContain('MAPROOM_EMBEDDING_PROVIDER')
  })

  it('returns error for invalid provider value', () => {
    process.env.MAPROOM_DATABASE_URL = 'postgresql://localhost/test'
    process.env.MAPROOM_EMBEDDING_PROVIDER = 'invalid-provider'
    const result = validateMaproomEnvironment()
    expect(result.valid).toBe(false)
    expect(result.errors.some((err) => err.includes('Invalid'))).toBe(true)
  })

  it('returns error when OpenAI provider missing API key', () => {
    process.env.MAPROOM_DATABASE_URL = 'postgresql://localhost/test'
    process.env.MAPROOM_EMBEDDING_PROVIDER = 'openai'
    const result = validateMaproomEnvironment()
    expect(result.valid).toBe(false)
    expect(result.errors.some((err) => err.includes('OPENAI_API_KEY'))).toBe(true)
  })

  it('returns valid when OpenAI provider has API key', () => {
    process.env.MAPROOM_DATABASE_URL = 'postgresql://localhost/test'
    process.env.MAPROOM_EMBEDDING_PROVIDER = 'openai'
    process.env.OPENAI_API_KEY = 'sk-test-key'
    const result = validateMaproomEnvironment()
    expect(result.valid).toBe(true)
  })

  it('returns error when Google provider missing project ID', () => {
    process.env.MAPROOM_DATABASE_URL = 'postgresql://localhost/test'
    process.env.MAPROOM_EMBEDDING_PROVIDER = 'google'
    const result = validateMaproomEnvironment()
    expect(result.valid).toBe(false)
    expect(result.errors.some((err) => err.includes('GOOGLE_PROJECT_ID'))).toBe(true)
  })

  it('returns valid when Google provider has project ID', () => {
    process.env.MAPROOM_DATABASE_URL = 'postgresql://localhost/test'
    process.env.MAPROOM_EMBEDDING_PROVIDER = 'google'
    process.env.GOOGLE_PROJECT_ID = 'my-project'
    const result = validateMaproomEnvironment()
    expect(result.valid).toBe(true)
  })

  it('returns valid when Ollama provider set (no additional requirements)', () => {
    process.env.MAPROOM_DATABASE_URL = 'postgresql://localhost/test'
    process.env.MAPROOM_EMBEDDING_PROVIDER = 'ollama'
    const result = validateMaproomEnvironment()
    expect(result.valid).toBe(true)
    expect(result.errors).toHaveLength(0)
  })

  it('does not leak credentials in error messages', () => {
    process.env.OPENAI_API_KEY = 'sk-super-secret-key-12345'
    process.env.MAPROOM_EMBEDDING_PROVIDER = 'openai'

    const result = validateMaproomEnvironment()

    // Check that no credential values appear in messages
    const allMessages = [...result.errors, ...result.warnings].join(' ')
    expect(allMessages).not.toContain('sk-super-secret-key')
  })

  it('accepts PG_DATABASE_URL as fallback', () => {
    process.env.PG_DATABASE_URL = 'postgresql://localhost/test'
    const result = validateMaproomEnvironment()
    expect(result.valid).toBe(true)
  })

  it('accepts DATABASE_URL as fallback', () => {
    process.env.DATABASE_URL = 'postgresql://localhost/test'
    const result = validateMaproomEnvironment()
    expect(result.valid).toBe(true)
  })

  it('accepts MAPROOM_DB_HOST as fallback', () => {
    process.env.MAPROOM_DB_HOST = 'localhost'
    const result = validateMaproomEnvironment()
    expect(result.valid).toBe(true)
  })
})

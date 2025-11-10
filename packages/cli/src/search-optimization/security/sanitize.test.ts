import { describe, it, expect } from 'vitest'
import { sanitizeDbUrl, sanitizeEnvironment, sanitizeAgentResult } from './sanitize'

describe('Sensitive Data Sanitization', () => {
  describe('sanitizeDbUrl', () => {
    it('redacts credentials from PostgreSQL URL', () => {
      const url = 'postgresql://maproom:secret123@localhost:5432/maproom'
      const sanitized = sanitizeDbUrl(url)
      expect(sanitized).toBe('postgresql://***:***@localhost:5432/maproom')
      expect(sanitized).not.toContain('secret123')
      expect(sanitized).not.toContain('maproom:secret')
    })

    it('handles URLs without credentials', () => {
      const url = 'postgresql://localhost:5432/maproom'
      const sanitized = sanitizeDbUrl(url)
      expect(sanitized).toBe('postgresql://localhost:5432/maproom')
    })

    it('redacts complex passwords with special characters', () => {
      const url = 'postgresql://user:p@ssw0rd!@#$@localhost:5432/db'
      const sanitized = sanitizeDbUrl(url)
      expect(sanitized).toBe('postgresql://***:***@localhost:5432/db')
      expect(sanitized).not.toContain('p@ssw0rd')
    })

    it('handles different database URL schemes', () => {
      const urls = [
        'postgresql://user:pass@host:5432/db',
        'postgres://user:pass@host:5432/db',
        'mysql://user:pass@host:3306/db',
      ]

      for (const url of urls) {
        const sanitized = sanitizeDbUrl(url)
        expect(sanitized).toContain('://***:***@')
        expect(sanitized).not.toContain(':pass@')
      }
    })
  })

  describe('sanitizeEnvironment', () => {
    it('redacts sensitive environment variables', () => {
      const env = {
        MAPROOM_DATABASE_URL: 'postgresql://user:pass@localhost/db',
        ANTHROPIC_API_KEY: 'sk-ant-1234567890',
        NODE_ENV: 'development',
        PATH: '/usr/bin',
      }

      const sanitized = sanitizeEnvironment(env)

      expect(sanitized.MAPROOM_DATABASE_URL).toBe('postgresql://***:***@localhost/db')
      expect(sanitized.ANTHROPIC_API_KEY).toBe('***')
      expect(sanitized.NODE_ENV).toBe('development') // Not sensitive
      expect(sanitized.PATH).toBe('/usr/bin') // Not sensitive
    })

    it('handles multiple sensitive keys', () => {
      const env = {
        DATABASE_URL: 'postgresql://user:pass@localhost/db',
        OPENAI_API_KEY: 'sk-1234567890',
        MY_PASSWORD: 'secret',
        MY_SECRET: 'hidden',
        PUBLIC_KEY: 'safe-to-show',
      }

      const sanitized = sanitizeEnvironment(env)

      expect(sanitized.DATABASE_URL).toBe('postgresql://***:***@localhost/db')
      expect(sanitized.OPENAI_API_KEY).toBe('***')
      expect(sanitized.MY_PASSWORD).toBe('***')
      expect(sanitized.MY_SECRET).toBe('***')
      expect(sanitized.PUBLIC_KEY).toBe('safe-to-show')
    })

    it('handles empty environment', () => {
      const sanitized = sanitizeEnvironment({})
      expect(sanitized).toEqual({})
    })

    it('skips undefined values', () => {
      const env = {
        DEFINED: 'value',
        UNDEFINED: undefined,
      }

      const sanitized = sanitizeEnvironment(env)

      expect(sanitized.DEFINED).toBe('value')
      expect(sanitized.UNDEFINED).toBeUndefined()
    })

    it('does not mutate original environment', () => {
      const env = {
        ANTHROPIC_API_KEY: 'sk-ant-1234567890',
        NODE_ENV: 'development',
      }

      const sanitized = sanitizeEnvironment(env)

      expect(env.ANTHROPIC_API_KEY).toBe('sk-ant-1234567890')
      expect(sanitized.ANTHROPIC_API_KEY).toBe('***')
    })
  })

  describe('sanitizeAgentResult', () => {
    it('sanitizes environment in result', () => {
      const result = {
        variantId: 'variant-a',
        success: true,
        environment: {
          MAPROOM_DATABASE_URL: 'postgresql://user:pass@localhost/db',
          NODE_ENV: 'test',
        },
      }

      const sanitized = sanitizeAgentResult(result)

      expect(sanitized.variantId).toBe('variant-a')
      expect(sanitized.success).toBe(true)
      expect(sanitized.environment?.MAPROOM_DATABASE_URL).toBe('postgresql://***:***@localhost/db')
      expect(sanitized.environment?.NODE_ENV).toBe('test')
    })

    it('handles result without environment', () => {
      const result = {
        variantId: 'variant-a',
        success: true,
      }

      const sanitized = sanitizeAgentResult(result)

      expect(sanitized.variantId).toBe('variant-a')
      expect(sanitized.success).toBe(true)
      expect(sanitized.environment).toBeUndefined()
    })

    it('does not mutate original result', () => {
      const result = {
        variantId: 'variant-a',
        success: true,
        environment: {
          ANTHROPIC_API_KEY: 'sk-ant-1234567890',
        },
      }

      const sanitized = sanitizeAgentResult(result)

      expect(result.environment?.ANTHROPIC_API_KEY).toBe('sk-ant-1234567890')
      expect(sanitized.environment?.ANTHROPIC_API_KEY).toBe('***')
    })
  })
})

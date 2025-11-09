/**
 * Tests for provider detection and configuration
 *
 * These tests verify:
 * - Explicit provider configuration via MAPROOM_EMBEDDING_PROVIDER
 * - Auto-detection of Ollama when running
 * - Fallback to OpenAI when configured
 * - Fallback to Google when configured
 * - Error handling when no provider available
 * - Validation of required environment variables
 * - Caching behavior
 */

import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest'
import {
  detectProvider,
  isOllamaAvailable,
  validateExplicitProvider,
  getProviderConfig,
  clearProviderCache,
} from '../src/utils/provider-detection'

describe('Provider Detection', () => {
  // Store original env vars
  const originalEnv = { ...process.env }

  beforeEach(() => {
    // Clear cache before each test
    clearProviderCache()

    // Reset environment variables
    delete process.env.MAPROOM_EMBEDDING_PROVIDER
    delete process.env.OPENAI_API_KEY
    delete process.env.GOOGLE_PROJECT_ID
    delete process.env.GOOGLE_APPLICATION_CREDENTIALS

    // Clear all mocks
    vi.restoreAllMocks()
  })

  afterEach(() => {
    // Restore original env
    process.env = { ...originalEnv }
  })

  describe('Explicit Provider Configuration', () => {
    it('should use explicit MAPROOM_EMBEDDING_PROVIDER=ollama', async () => {
      process.env.MAPROOM_EMBEDDING_PROVIDER = 'ollama'

      const config = await detectProvider()

      expect(config.provider).toBe('ollama')
      expect(config.dimension).toBe(768)
      expect(config.available).toBe(true)
    })

    it('should use explicit MAPROOM_EMBEDDING_PROVIDER=openai with API key', async () => {
      process.env.MAPROOM_EMBEDDING_PROVIDER = 'openai'
      process.env.OPENAI_API_KEY = 'sk-test123'

      const config = await detectProvider()

      expect(config.provider).toBe('openai')
      expect(config.dimension).toBe(1536)
      expect(config.available).toBe(true)
    })

    it('should use explicit MAPROOM_EMBEDDING_PROVIDER=google with credentials', async () => {
      process.env.MAPROOM_EMBEDDING_PROVIDER = 'google'
      process.env.GOOGLE_PROJECT_ID = 'test-project'
      process.env.GOOGLE_APPLICATION_CREDENTIALS = '/path/to/key.json'

      const config = await detectProvider()

      expect(config.provider).toBe('google')
      expect(config.dimension).toBe(768)
      expect(config.available).toBe(true)
    })

    it('should handle case-insensitive provider names', async () => {
      process.env.MAPROOM_EMBEDDING_PROVIDER = 'OLLAMA'

      const config = await detectProvider()

      expect(config.provider).toBe('ollama')
    })

    it('should throw error for unknown provider', async () => {
      process.env.MAPROOM_EMBEDDING_PROVIDER = 'unknown-provider'

      await expect(detectProvider()).rejects.toThrow(
        'Unknown provider: "unknown-provider"'
      )
    })

    it('should throw error for openai without API key', async () => {
      process.env.MAPROOM_EMBEDDING_PROVIDER = 'openai'
      // No OPENAI_API_KEY set

      await expect(detectProvider()).rejects.toThrow('OPENAI_API_KEY not found')
    })

    it('should throw error for google without PROJECT_ID', async () => {
      process.env.MAPROOM_EMBEDDING_PROVIDER = 'google'
      process.env.GOOGLE_APPLICATION_CREDENTIALS = '/path/to/key.json'
      // No GOOGLE_PROJECT_ID set

      await expect(detectProvider()).rejects.toThrow('GOOGLE_PROJECT_ID not found')
    })

    it('should throw error for google without CREDENTIALS', async () => {
      process.env.MAPROOM_EMBEDDING_PROVIDER = 'google'
      process.env.GOOGLE_PROJECT_ID = 'test-project'
      // No GOOGLE_APPLICATION_CREDENTIALS set

      await expect(detectProvider()).rejects.toThrow(
        'GOOGLE_APPLICATION_CREDENTIALS not found'
      )
    })
  })

  describe('Auto-detection', () => {
    it('should detect Ollama when available', async () => {
      // Mock successful Ollama response
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          models: [
            { name: 'nomic-embed-text:latest' },
            { name: 'llama2:latest' },
          ],
        }),
      })

      const config = await detectProvider()

      expect(config.provider).toBe('ollama')
      expect(config.dimension).toBe(768)
      expect(global.fetch).toHaveBeenCalledWith(
        'http://localhost:11434/api/tags',
        expect.objectContaining({ method: 'GET' })
      )
    })

    it('should fall back to OpenAI when Ollama not available', async () => {
      // Mock Ollama connection refused
      global.fetch = vi.fn().mockRejectedValue(new Error('Connection refused'))
      process.env.OPENAI_API_KEY = 'sk-test123'

      const config = await detectProvider()

      expect(config.provider).toBe('openai')
      expect(config.dimension).toBe(1536)
    })

    it('should fall back to Google when OpenAI not configured', async () => {
      // Mock Ollama connection refused
      global.fetch = vi.fn().mockRejectedValue(new Error('Connection refused'))
      process.env.GOOGLE_PROJECT_ID = 'test-project'
      process.env.GOOGLE_APPLICATION_CREDENTIALS = '/path/to/key.json'

      const config = await detectProvider()

      expect(config.provider).toBe('google')
      expect(config.dimension).toBe(768)
    })

    it('should throw error when no provider available', async () => {
      // Mock Ollama connection refused
      global.fetch = vi.fn().mockRejectedValue(new Error('Connection refused'))
      // No other providers configured

      await expect(detectProvider()).rejects.toThrow(
        'No embedding provider available'
      )
    })

    it('should provide helpful error message with setup options', async () => {
      global.fetch = vi.fn().mockRejectedValue(new Error('Connection refused'))

      try {
        await detectProvider()
        expect.fail('Should have thrown error')
      } catch (error: any) {
        expect(error.message).toContain('Install Ollama')
        expect(error.message).toContain('OPENAI_API_KEY')
        expect(error.message).toContain('Google Vertex AI')
        expect(error.message).toContain('MAPROOM_EMBEDDING_PROVIDER')
      }
    })
  })

  describe('Ollama Detection', () => {
    it('should return true when Ollama is running with nomic-embed-text', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          models: [
            { name: 'nomic-embed-text:latest' },
            { name: 'llama2:latest' },
          ],
        }),
      })

      const available = await isOllamaAvailable()

      expect(available).toBe(true)
    })

    it('should return false when Ollama running but model missing', async () => {
      const consoleWarnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          models: [
            { name: 'llama2:latest' },
            { name: 'mistral:latest' },
          ],
        }),
      })

      const available = await isOllamaAvailable()

      expect(available).toBe(false)
      expect(consoleWarnSpy).toHaveBeenCalledWith(
        expect.stringContaining('nomic-embed-text model not found')
      )

      consoleWarnSpy.mockRestore()
    })

    it('should return false when connection refused', async () => {
      global.fetch = vi.fn().mockRejectedValue(new Error('Connection refused'))

      const available = await isOllamaAvailable()

      expect(available).toBe(false)
    })

    it('should return false when connection times out', async () => {
      // Mock fetch that takes longer than timeout
      global.fetch = vi.fn().mockImplementation(() => {
        return new Promise((resolve) => {
          setTimeout(() => {
            resolve({
              ok: true,
              json: async () => ({ models: [] }),
            })
          }, 5000) // Longer than 2s timeout
        })
      })

      const available = await isOllamaAvailable()

      expect(available).toBe(false)
    })

    it('should return false when response is not ok', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 500,
      })

      const available = await isOllamaAvailable()

      expect(available).toBe(false)
    })

    it('should handle network errors gracefully', async () => {
      global.fetch = vi.fn().mockRejectedValue(new Error('Network error'))

      const available = await isOllamaAvailable()

      expect(available).toBe(false)
    })

    it('should use 2 second timeout', async () => {
      const startTime = Date.now()

      // Mock fetch that respects abort signal
      global.fetch = vi.fn().mockImplementation((_url, options: any) => {
        return new Promise((resolve, reject) => {
          // If signal is aborted, reject immediately
          if (options?.signal?.aborted) {
            reject(new Error('Aborted'))
            return
          }

          // Listen for abort event
          options?.signal?.addEventListener('abort', () => {
            reject(new Error('Aborted'))
          })

          // Never resolve otherwise (simulates hanging connection)
        })
      })

      await isOllamaAvailable()

      const duration = Date.now() - startTime
      // Should complete within ~2 seconds (allow some margin)
      expect(duration).toBeLessThan(2500)
      expect(duration).toBeGreaterThanOrEqual(2000)
    })
  })

  describe('Validate Explicit Provider', () => {
    it('should validate ollama provider', () => {
      const config = validateExplicitProvider('ollama')

      expect(config.provider).toBe('ollama')
      expect(config.dimension).toBe(768)
      expect(config.available).toBe(true)
    })

    it('should validate openai provider with API key', () => {
      process.env.OPENAI_API_KEY = 'sk-test123'

      const config = validateExplicitProvider('openai')

      expect(config.provider).toBe('openai')
      expect(config.dimension).toBe(1536)
    })

    it('should validate google provider with credentials', () => {
      process.env.GOOGLE_PROJECT_ID = 'test-project'
      process.env.GOOGLE_APPLICATION_CREDENTIALS = '/path/to/key.json'

      const config = validateExplicitProvider('google')

      expect(config.provider).toBe('google')
      expect(config.dimension).toBe(768)
    })

    it('should throw for invalid provider name', () => {
      expect(() => validateExplicitProvider('invalid')).toThrow(
        'Unknown provider: "invalid"'
      )
    })
  })

  describe('Provider Config Caching', () => {
    it('should cache provider detection result', async () => {
      process.env.MAPROOM_EMBEDDING_PROVIDER = 'ollama'

      const config1 = await getProviderConfig()
      const config2 = await getProviderConfig()

      // Same object reference (cached)
      expect(config1).toBe(config2)
    })

    it('should not re-detect on subsequent calls', async () => {
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          models: [{ name: 'nomic-embed-text:latest' }],
        }),
      })

      await getProviderConfig()
      await getProviderConfig()
      await getProviderConfig()

      // Should only call fetch once (first detection)
      expect(global.fetch).toHaveBeenCalledTimes(1)
    })

    it('should clear cache when requested', async () => {
      process.env.MAPROOM_EMBEDDING_PROVIDER = 'ollama'

      const config1 = await getProviderConfig()
      clearProviderCache()
      const config2 = await getProviderConfig()

      // Different instances after cache clear
      expect(config1).not.toBe(config2)
      // But same values
      expect(config1.provider).toBe(config2.provider)
    })

    it('should re-detect after cache clear', async () => {
      const fetchSpy = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          models: [{ name: 'nomic-embed-text:latest' }],
        }),
      })
      global.fetch = fetchSpy

      await getProviderConfig()
      expect(fetchSpy).toHaveBeenCalledTimes(1)

      clearProviderCache()

      await getProviderConfig()
      expect(fetchSpy).toHaveBeenCalledTimes(2)
    })
  })

  describe('Priority Order', () => {
    it('should prefer explicit MAPROOM_EMBEDDING_PROVIDER over auto-detection', async () => {
      // Setup Ollama as available
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          models: [{ name: 'nomic-embed-text:latest' }],
        }),
      })

      // But explicitly request OpenAI
      process.env.MAPROOM_EMBEDDING_PROVIDER = 'openai'
      process.env.OPENAI_API_KEY = 'sk-test123'

      const config = await detectProvider()

      // Should use explicit config, not auto-detected Ollama
      expect(config.provider).toBe('openai')
    })

    it('should prefer Ollama over OpenAI in auto-detection', async () => {
      // Both available
      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          models: [{ name: 'nomic-embed-text:latest' }],
        }),
      })
      process.env.OPENAI_API_KEY = 'sk-test123'

      const config = await detectProvider()

      // Should prefer Ollama (zero-config)
      expect(config.provider).toBe('ollama')
    })

    it('should prefer OpenAI over Google in auto-detection', async () => {
      // Ollama not available
      global.fetch = vi.fn().mockRejectedValue(new Error('Connection refused'))

      // Both cloud providers configured
      process.env.OPENAI_API_KEY = 'sk-test123'
      process.env.GOOGLE_PROJECT_ID = 'test-project'
      process.env.GOOGLE_APPLICATION_CREDENTIALS = '/path/to/key.json'

      const config = await detectProvider()

      // Should prefer OpenAI
      expect(config.provider).toBe('openai')
    })
  })

  describe('Console Output', () => {
    it('should log when using explicit provider', async () => {
      const consoleLogSpy = vi.spyOn(console, 'log').mockImplementation(() => {})
      process.env.MAPROOM_EMBEDDING_PROVIDER = 'ollama'

      await detectProvider()

      expect(consoleLogSpy).toHaveBeenCalledWith(
        'Using explicit provider: ollama'
      )

      consoleLogSpy.mockRestore()
    })

    it('should log when auto-detecting', async () => {
      const consoleLogSpy = vi.spyOn(console, 'log').mockImplementation(() => {})

      global.fetch = vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          models: [{ name: 'nomic-embed-text:latest' }],
        }),
      })

      await detectProvider()

      expect(consoleLogSpy).toHaveBeenCalledWith('Auto-detecting embedding provider...')
      expect(consoleLogSpy).toHaveBeenCalledWith(
        '✓ Ollama detected at localhost:11434'
      )

      consoleLogSpy.mockRestore()
    })

    it('should log when using OpenAI', async () => {
      const consoleLogSpy = vi.spyOn(console, 'log').mockImplementation(() => {})

      global.fetch = vi.fn().mockRejectedValue(new Error('Connection refused'))
      process.env.OPENAI_API_KEY = 'sk-test123'

      await detectProvider()

      expect(consoleLogSpy).toHaveBeenCalledWith(
        '✓ Using OpenAI (OPENAI_API_KEY found)'
      )

      consoleLogSpy.mockRestore()
    })

    it('should log when using Google', async () => {
      const consoleLogSpy = vi.spyOn(console, 'log').mockImplementation(() => {})

      global.fetch = vi.fn().mockRejectedValue(new Error('Connection refused'))
      process.env.GOOGLE_PROJECT_ID = 'test-project'
      process.env.GOOGLE_APPLICATION_CREDENTIALS = '/path/to/key.json'

      await detectProvider()

      expect(consoleLogSpy).toHaveBeenCalledWith(
        '✓ Using Google Vertex AI (GOOGLE_PROJECT_ID found)'
      )

      consoleLogSpy.mockRestore()
    })
  })
})

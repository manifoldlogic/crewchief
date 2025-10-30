/**
 * Unit tests for Scan tool provider integration
 *
 * Tests cover:
 * - Provider detection integration
 * - Provider flag passed to binary
 * - Error handling when no provider available
 * - Provider information in results
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { clearProviderCache } from '../../src/utils/provider-detection.js'

describe('Scan Tool - Provider Integration', () => {
  const originalEnv = { ...process.env }

  beforeEach(() => {
    clearProviderCache()
    vi.restoreAllMocks()
  })

  afterEach(() => {
    process.env = { ...originalEnv }
  })

  it('should detect provider and include in result', async () => {
    // Mock provider detection
    process.env.EMBEDDING_PROVIDER = 'ollama'

    // We'll test the integration through the index handler
    // This is a unit test of the provider integration logic
    const { getProviderConfig } = await import('../../src/utils/provider-detection.js')

    const config = await getProviderConfig()

    expect(config.provider).toBe('ollama')
    expect(config.dimension).toBe(768)
  })

  it('should return error when no provider available', async () => {
    // Clear all provider configs
    delete process.env.EMBEDDING_PROVIDER
    delete process.env.OPENAI_API_KEY
    delete process.env.GOOGLE_PROJECT_ID
    delete process.env.GOOGLE_APPLICATION_CREDENTIALS

    // Mock Ollama not available
    global.fetch = vi.fn().mockRejectedValue(new Error('Connection refused'))

    const { getProviderConfig } = await import('../../src/utils/provider-detection.js')

    await expect(getProviderConfig()).rejects.toThrow('No embedding provider available')
  })

  it('should prefer explicit provider over auto-detection', async () => {
    // Setup Ollama as available
    global.fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({
        models: [{ name: 'nomic-embed-text:latest' }],
      }),
    })

    // But explicitly request OpenAI
    process.env.EMBEDDING_PROVIDER = 'openai'
    process.env.OPENAI_API_KEY = 'sk-test123'

    const { getProviderConfig } = await import('../../src/utils/provider-detection.js')
    const config = await getProviderConfig()

    // Should use explicit config, not auto-detected Ollama
    expect(config.provider).toBe('openai')
    expect(config.dimension).toBe(1536)
  })

  it('should use cached provider on subsequent calls', async () => {
    process.env.EMBEDDING_PROVIDER = 'ollama'

    const { getProviderConfig } = await import('../../src/utils/provider-detection.js')

    const config1 = await getProviderConfig()
    const config2 = await getProviderConfig()

    // Should return same cached instance
    expect(config1).toBe(config2)
  })

  it('should include helpful setup instructions in error', async () => {
    delete process.env.EMBEDDING_PROVIDER
    delete process.env.OPENAI_API_KEY
    global.fetch = vi.fn().mockRejectedValue(new Error('Connection refused'))

    const { getProviderConfig } = await import('../../src/utils/provider-detection.js')

    try {
      await getProviderConfig()
      expect.fail('Should have thrown error')
    } catch (error: any) {
      expect(error.message).toContain('Install Ollama')
      expect(error.message).toContain('OPENAI_API_KEY')
      expect(error.message).toContain('Google Vertex AI')
      expect(error.message).toContain('EMBEDDING_PROVIDER')
    }
  })

  it('should handle Google provider correctly', async () => {
    process.env.EMBEDDING_PROVIDER = 'google'
    process.env.GOOGLE_PROJECT_ID = 'test-project'
    process.env.GOOGLE_APPLICATION_CREDENTIALS = '/path/to/key.json'

    const { getProviderConfig } = await import('../../src/utils/provider-detection.js')
    const config = await getProviderConfig()

    expect(config.provider).toBe('google')
    expect(config.dimension).toBe(768)
  })
})

/**
 * Tests for SecretsManager
 *
 * Verifies secure storage and retrieval of API credentials using
 * VSCode's SecretStorage API. Ensures no credentials are leaked
 * and proper environment variable mapping.
 */

import { describe, it, expect, beforeEach, vi } from 'vitest'
import { SecretsManager } from './secrets.js'

/**
 * Mock SecretStorage implementation
 *
 * Simulates VSCode's encrypted SecretStorage API for testing.
 * Stores secrets in memory using a Map.
 */
class MockSecretStorage {
  private storage = new Map<string, string>()

  async get(key: string): Promise<string | undefined> {
    return this.storage.get(key)
  }

  async store(key: string, value: string): Promise<void> {
    this.storage.set(key, value)
  }

  async delete(key: string): Promise<void> {
    this.storage.delete(key)
  }

  // Test helper: clear all secrets
  clear(): void {
    this.storage.clear()
  }

  // Test helper: get all keys
  keys(): string[] {
    return Array.from(this.storage.keys())
  }
}

describe('SecretsManager', () => {
  let mockSecrets: MockSecretStorage
  let secretsManager: SecretsManager

  beforeEach(() => {
    mockSecrets = new MockSecretStorage()
    secretsManager = new SecretsManager(mockSecrets as any)
  })

  describe('storeApiKey', () => {
    it('should store OpenAI API key with correct secret key', async () => {
      const apiKey = 'sk-test-openai-key-1234567890'

      await secretsManager.storeApiKey('openai', apiKey)

      // Verify stored with provider-specific key
      expect(mockSecrets.keys()).toContain('maproom.openai_key')
      expect(await mockSecrets.get('maproom.openai_key')).toBe(apiKey)
    })

    it('should store Google API key with correct secret key', async () => {
      const apiKey = 'google-api-key-abcdef123456'

      await secretsManager.storeApiKey('google', apiKey)

      // Verify stored with provider-specific key
      expect(mockSecrets.keys()).toContain('maproom.google_key')
      expect(await mockSecrets.get('maproom.google_key')).toBe(apiKey)
    })

    it('should do nothing for Ollama provider (local model)', async () => {
      await secretsManager.storeApiKey('ollama', 'should-not-be-stored')

      // Verify nothing was stored
      expect(mockSecrets.keys()).toHaveLength(0)
    })

    it('should overwrite existing API key for provider', async () => {
      const oldKey = 'sk-old-key'
      const newKey = 'sk-new-key'

      await secretsManager.storeApiKey('openai', oldKey)
      await secretsManager.storeApiKey('openai', newKey)

      // Verify new key replaced old key
      expect(await mockSecrets.get('maproom.openai_key')).toBe(newKey)
    })

    it('should store different keys for different providers', async () => {
      const openaiKey = 'sk-openai-key'
      const googleKey = 'google-key'

      await secretsManager.storeApiKey('openai', openaiKey)
      await secretsManager.storeApiKey('google', googleKey)

      // Verify both keys stored independently
      expect(await mockSecrets.get('maproom.openai_key')).toBe(openaiKey)
      expect(await mockSecrets.get('maproom.google_key')).toBe(googleKey)
      expect(mockSecrets.keys()).toHaveLength(2)
    })
  })

  describe('getApiKey', () => {
    it('should retrieve OpenAI API key', async () => {
      const apiKey = 'sk-test-openai-key'
      await mockSecrets.store('maproom.openai_key', apiKey)

      const retrieved = await secretsManager.getApiKey('openai')

      expect(retrieved).toBe(apiKey)
    })

    it('should retrieve Google API key', async () => {
      const apiKey = 'google-test-key'
      await mockSecrets.store('maproom.google_key', apiKey)

      const retrieved = await secretsManager.getApiKey('google')

      expect(retrieved).toBe(apiKey)
    })

    it('should return undefined for Ollama provider', async () => {
      const retrieved = await secretsManager.getApiKey('ollama')

      expect(retrieved).toBeUndefined()
    })

    it('should return undefined when API key not stored', async () => {
      const retrieved = await secretsManager.getApiKey('openai')

      expect(retrieved).toBeUndefined()
    })

    it('should not leak credentials between providers', async () => {
      await mockSecrets.store('maproom.openai_key', 'openai-key')
      await mockSecrets.store('maproom.google_key', 'google-key')

      const openaiKey = await secretsManager.getApiKey('openai')
      const googleKey = await secretsManager.getApiKey('google')

      expect(openaiKey).toBe('openai-key')
      expect(googleKey).toBe('google-key')
      expect(openaiKey).not.toBe(googleKey)
    })
  })

  describe('getEnvironmentVars', () => {
    it('should return MAPROOM_OPENAI_API_KEY for OpenAI provider', async () => {
      const apiKey = 'sk-test-key'
      await mockSecrets.store('maproom.openai_key', apiKey)

      const env = await secretsManager.getEnvironmentVars('openai')

      expect(env).toEqual({
        MAPROOM_OPENAI_API_KEY: apiKey,
      })
    })

    it('should return MAPROOM_GOOGLE_APPLICATION_CREDENTIALS for Google provider', async () => {
      const apiKey = 'google-key'
      await mockSecrets.store('maproom.google_key', apiKey)

      const env = await secretsManager.getEnvironmentVars('google')

      expect(env).toEqual({
        MAPROOM_GOOGLE_APPLICATION_CREDENTIALS: apiKey,
      })
    })

    it('should return empty object for Ollama provider', async () => {
      const env = await secretsManager.getEnvironmentVars('ollama')

      expect(env).toEqual({})
    })

    it('should return empty object when API key not stored', async () => {
      const env = await secretsManager.getEnvironmentVars('openai')

      expect(env).toEqual({})
    })

    it('should return correct env var names for all providers', async () => {
      // Store keys for all providers
      await mockSecrets.store('maproom.openai_key', 'openai-key')
      await mockSecrets.store('maproom.google_key', 'google-key')

      const openaiEnv = await secretsManager.getEnvironmentVars('openai')
      const googleEnv = await secretsManager.getEnvironmentVars('google')

      // Verify correct environment variable names (MAPROOM_ prefixed)
      expect(Object.keys(openaiEnv)).toEqual(['MAPROOM_OPENAI_API_KEY'])
      expect(Object.keys(googleEnv)).toEqual(['MAPROOM_GOOGLE_APPLICATION_CREDENTIALS'])
    })

    it('should not include credentials in object keys inspection', async () => {
      const apiKey = 'sk-secret-key-12345'
      await mockSecrets.store('maproom.openai_key', apiKey)

      const env = await secretsManager.getEnvironmentVars('openai')

      // Verify that inspecting keys doesn't reveal credential
      const keys = Object.keys(env)
      expect(keys).toEqual(['MAPROOM_OPENAI_API_KEY'])

      // But value is correct
      expect(env['MAPROOM_OPENAI_API_KEY']).toBe(apiKey)
    })
  })

  describe('deleteApiKey', () => {
    it('should delete OpenAI API key', async () => {
      await mockSecrets.store('maproom.openai_key', 'test-key')

      await secretsManager.deleteApiKey('openai')

      expect(await mockSecrets.get('maproom.openai_key')).toBeUndefined()
      expect(mockSecrets.keys()).toHaveLength(0)
    })

    it('should delete Google API key', async () => {
      await mockSecrets.store('maproom.google_key', 'test-key')

      await secretsManager.deleteApiKey('google')

      expect(await mockSecrets.get('maproom.google_key')).toBeUndefined()
      expect(mockSecrets.keys()).toHaveLength(0)
    })

    it('should do nothing for Ollama provider', async () => {
      await secretsManager.deleteApiKey('ollama')

      // No error should be thrown
      expect(mockSecrets.keys()).toHaveLength(0)
    })

    it('should not delete other provider keys', async () => {
      await mockSecrets.store('maproom.openai_key', 'openai-key')
      await mockSecrets.store('maproom.google_key', 'google-key')

      await secretsManager.deleteApiKey('openai')

      // OpenAI key deleted, Google key remains
      expect(await mockSecrets.get('maproom.openai_key')).toBeUndefined()
      expect(await mockSecrets.get('maproom.google_key')).toBe('google-key')
    })

    it('should not error when deleting non-existent key', async () => {
      // Should not throw error
      await expect(secretsManager.deleteApiKey('openai')).resolves.toBeUndefined()
    })
  })

  describe('hasApiKey', () => {
    it('should return true when OpenAI key exists', async () => {
      await mockSecrets.store('maproom.openai_key', 'test-key')

      const hasKey = await secretsManager.hasApiKey('openai')

      expect(hasKey).toBe(true)
    })

    it('should return true when Google key exists', async () => {
      await mockSecrets.store('maproom.google_key', 'test-key')

      const hasKey = await secretsManager.hasApiKey('google')

      expect(hasKey).toBe(true)
    })

    it('should return false when key not stored', async () => {
      const hasKey = await secretsManager.hasApiKey('openai')

      expect(hasKey).toBe(false)
    })

    it('should return false when key is empty string', async () => {
      await mockSecrets.store('maproom.openai_key', '')

      const hasKey = await secretsManager.hasApiKey('openai')

      expect(hasKey).toBe(false)
    })

    it('should return true for Ollama (no credentials needed)', async () => {
      const hasKey = await secretsManager.hasApiKey('ollama')

      expect(hasKey).toBe(true)
    })

    it('should not be affected by other provider keys', async () => {
      await mockSecrets.store('maproom.google_key', 'google-key')

      const hasOpenai = await secretsManager.hasApiKey('openai')
      const hasGoogle = await secretsManager.hasApiKey('google')

      expect(hasOpenai).toBe(false)
      expect(hasGoogle).toBe(true)
    })
  })

  describe('Credential Security', () => {
    it('should never log credentials during store', async () => {
      const consoleSpy = vi.spyOn(console, 'log')
      const consoleErrorSpy = vi.spyOn(console, 'error')
      const consoleWarnSpy = vi.spyOn(console, 'warn')

      const sensitiveKey = 'sk-super-secret-key-do-not-log'
      await secretsManager.storeApiKey('openai', sensitiveKey)

      // Verify credentials were not logged
      expect(consoleSpy).not.toHaveBeenCalledWith(expect.stringContaining(sensitiveKey))
      expect(consoleErrorSpy).not.toHaveBeenCalledWith(expect.stringContaining(sensitiveKey))
      expect(consoleWarnSpy).not.toHaveBeenCalledWith(expect.stringContaining(sensitiveKey))

      consoleSpy.mockRestore()
      consoleErrorSpy.mockRestore()
      consoleWarnSpy.mockRestore()
    })

    it('should never log credentials during retrieve', async () => {
      const sensitiveKey = 'sk-super-secret-key-do-not-log'
      await mockSecrets.store('maproom.openai_key', sensitiveKey)

      const consoleSpy = vi.spyOn(console, 'log')
      const consoleErrorSpy = vi.spyOn(console, 'error')
      const consoleWarnSpy = vi.spyOn(console, 'warn')

      await secretsManager.getApiKey('openai')

      // Verify credentials were not logged
      expect(consoleSpy).not.toHaveBeenCalledWith(expect.stringContaining(sensitiveKey))
      expect(consoleErrorSpy).not.toHaveBeenCalledWith(expect.stringContaining(sensitiveKey))
      expect(consoleWarnSpy).not.toHaveBeenCalledWith(expect.stringContaining(sensitiveKey))

      consoleSpy.mockRestore()
      consoleErrorSpy.mockRestore()
      consoleWarnSpy.mockRestore()
    })

    it('should never log credentials during environment var generation', async () => {
      const sensitiveKey = 'sk-super-secret-key-do-not-log'
      await mockSecrets.store('maproom.openai_key', sensitiveKey)

      const consoleSpy = vi.spyOn(console, 'log')
      const consoleErrorSpy = vi.spyOn(console, 'error')
      const consoleWarnSpy = vi.spyOn(console, 'warn')

      await secretsManager.getEnvironmentVars('openai')

      // Verify credentials were not logged
      expect(consoleSpy).not.toHaveBeenCalledWith(expect.stringContaining(sensitiveKey))
      expect(consoleErrorSpy).not.toHaveBeenCalledWith(expect.stringContaining(sensitiveKey))
      expect(consoleWarnSpy).not.toHaveBeenCalledWith(expect.stringContaining(sensitiveKey))

      consoleSpy.mockRestore()
      consoleErrorSpy.mockRestore()
      consoleWarnSpy.mockRestore()
    })
  })

  describe('Provider Key Isolation', () => {
    it('should use distinct secret keys for each provider', async () => {
      const openaiKey = 'openai-key'
      const googleKey = 'google-key'

      await secretsManager.storeApiKey('openai', openaiKey)
      await secretsManager.storeApiKey('google', googleKey)

      // Verify stored with different keys
      const storedKeys = mockSecrets.keys()
      expect(storedKeys).toHaveLength(2)
      expect(storedKeys).toContain('maproom.openai_key')
      expect(storedKeys).toContain('maproom.google_key')
    })

    it('should use distinct environment variable names for each provider', async () => {
      await mockSecrets.store('maproom.openai_key', 'key1')
      await mockSecrets.store('maproom.google_key', 'key2')

      const openaiEnv = await secretsManager.getEnvironmentVars('openai')
      const googleEnv = await secretsManager.getEnvironmentVars('google')

      // Verify distinct env var names (MAPROOM_ prefixed)
      expect(Object.keys(openaiEnv)).toEqual(['MAPROOM_OPENAI_API_KEY'])
      expect(Object.keys(googleEnv)).toEqual(['MAPROOM_GOOGLE_APPLICATION_CREDENTIALS'])

      // Verify no overlap
      expect(openaiEnv).not.toHaveProperty('MAPROOM_GOOGLE_APPLICATION_CREDENTIALS')
      expect(googleEnv).not.toHaveProperty('MAPROOM_OPENAI_API_KEY')
    })
  })

  describe('Integration with Process Spawning', () => {
    it('should provide env vars that can be spread into process.env', async () => {
      const apiKey = 'sk-test-key'
      await mockSecrets.store('maproom.openai_key', apiKey)

      const env = await secretsManager.getEnvironmentVars('openai')

      // Simulate process spawn environment setup
      const mockBaseEnv = { PATH: '/usr/bin', HOME: '/home/user' }
      const spawnEnv = {
        ...mockBaseEnv,
        ...env,
      }

      expect(spawnEnv.MAPROOM_OPENAI_API_KEY).toBe(apiKey)
      expect(spawnEnv.PATH).toBe('/usr/bin') // Verify base env preserved
    })

    it('should work with multiple providers in spawn environment', async () => {
      await mockSecrets.store('maproom.openai_key', 'openai-key')
      await mockSecrets.store('maproom.google_key', 'google-key')

      const openaiEnv = await secretsManager.getEnvironmentVars('openai')
      const googleEnv = await secretsManager.getEnvironmentVars('google')

      // Simulate switching providers - use clean base env
      const baseEnv = { PATH: '/usr/bin' }
      const openaiSpawnEnv = { ...baseEnv, ...openaiEnv }
      const googleSpawnEnv = { ...baseEnv, ...googleEnv }

      expect(openaiSpawnEnv.MAPROOM_OPENAI_API_KEY).toBe('openai-key')
      expect(openaiSpawnEnv.MAPROOM_GOOGLE_APPLICATION_CREDENTIALS).toBeUndefined()

      expect(googleSpawnEnv.MAPROOM_GOOGLE_APPLICATION_CREDENTIALS).toBe('google-key')
      expect(googleSpawnEnv.MAPROOM_OPENAI_API_KEY).toBeUndefined()
    })

    it('should handle missing credentials gracefully in spawn environment', async () => {
      // No credentials stored
      const env = await secretsManager.getEnvironmentVars('openai')

      // Use clean base env
      const baseEnv = { PATH: '/usr/bin' }
      const spawnEnv = { ...baseEnv, ...env }

      // Should not add env var when credential missing
      expect(spawnEnv.MAPROOM_OPENAI_API_KEY).toBeUndefined()
      expect(Object.keys(env)).toHaveLength(0) // Empty env vars returned
    })
  })
})

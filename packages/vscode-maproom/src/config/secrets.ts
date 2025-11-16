/**
 * Secrets Manager for secure API credential storage
 *
 * Wraps VSCode's SecretStorage API to securely store and retrieve
 * embedding provider API credentials. Credentials are encrypted at rest
 * and never logged to output channels or console.
 *
 * Security considerations:
 * - All credentials stored via VSCode SecretStorage (encrypted)
 * - No credentials logged anywhere in the codebase
 * - Password-masked input during collection
 * - Clear credential variables after use
 *
 * Supported providers:
 * - ollama: No credentials needed (local model)
 * - openai: Requires OPENAI_API_KEY
 * - google: Requires GOOGLE_APPLICATION_CREDENTIALS (API key)
 */

import type * as vscode from 'vscode'

/**
 * Supported embedding providers
 */
export type EmbeddingProvider = 'ollama' | 'openai' | 'google'

/**
 * Secret storage keys for each provider
 */
const SECRET_KEYS = {
  openai: 'maproom.openai_key',
  google: 'maproom.google_key',
} as const

/**
 * Environment variable names expected by Rust binary
 *
 * Using MAPROOM_ prefixed versions as specified in ticket VSMAP-2002.
 * The Rust binary accepts both prefixed and standard names for flexibility.
 */
const ENV_VAR_NAMES = {
  openai: 'MAPROOM_OPENAI_API_KEY',
  google: 'MAPROOM_GOOGLE_APPLICATION_CREDENTIALS',
} as const

/**
 * Secrets Manager for API credentials
 *
 * Provides secure storage and retrieval of embedding provider API keys
 * using VSCode's encrypted SecretStorage API.
 *
 * Example usage:
 * ```typescript
 * const secretsManager = new SecretsManager(context.secrets);
 *
 * // Store API key after user input
 * await secretsManager.storeApiKey('openai', apiKey);
 *
 * // Retrieve API key for process spawn
 * const key = await secretsManager.getApiKey('openai');
 *
 * // Get environment variables for Rust binary
 * const env = await secretsManager.getEnvironmentVars('openai');
 * // Returns: { OPENAI_API_KEY: 'sk-...' }
 * ```
 */
export class SecretsManager {
  /**
   * Create a new SecretsManager
   *
   * @param secrets - VSCode SecretStorage instance from ExtensionContext
   */
  constructor(private readonly secrets: vscode.SecretStorage) {}

  /**
   * Store API key for a provider
   *
   * Saves the API key to VSCode's encrypted SecretStorage.
   * For Ollama (local model), this is a no-op since no credentials are needed.
   *
   * SECURITY: The key is stored encrypted and never logged.
   *
   * @param provider - Embedding provider
   * @param key - API key to store
   * @returns Promise that resolves when key is stored
   */
  async storeApiKey(provider: EmbeddingProvider, key: string): Promise<void> {
    // Ollama doesn't need credentials (local model)
    if (provider === 'ollama') {
      return
    }

    // Validate provider is supported
    if (provider !== 'openai' && provider !== 'google') {
      throw new Error(`Unsupported provider: ${provider}`)
    }

    // Store encrypted credential
    const secretKey = SECRET_KEYS[provider]
    await this.secrets.store(secretKey, key)
  }

  /**
   * Retrieve API key for a provider
   *
   * Fetches the stored API key from VSCode's encrypted SecretStorage.
   * For Ollama (local model), returns undefined since no credentials are needed.
   *
   * @param provider - Embedding provider
   * @returns Promise resolving to API key, or undefined if not set or not needed
   */
  async getApiKey(provider: EmbeddingProvider): Promise<string | undefined> {
    // Ollama doesn't need credentials (local model)
    if (provider === 'ollama') {
      return undefined
    }

    // Validate provider is supported
    if (provider !== 'openai' && provider !== 'google') {
      throw new Error(`Unsupported provider: ${provider}`)
    }

    // Retrieve encrypted credential
    const secretKey = SECRET_KEYS[provider]
    return await this.secrets.get(secretKey)
  }

  /**
   * Get environment variables for spawning Rust binary
   *
   * Returns an object with the appropriate environment variable names
   * and values for the specified provider. This can be spread into the
   * process spawn environment.
   *
   * For Ollama (local model), returns an empty object since no credentials needed.
   *
   * Example:
   * ```typescript
   * const env = {
   *   ...process.env,
   *   ...await secretsManager.getEnvironmentVars(provider)
   * };
   * spawn(binaryPath, args, { env });
   * ```
   *
   * @param provider - Embedding provider
   * @returns Promise resolving to environment variables object
   */
  async getEnvironmentVars(
    provider: EmbeddingProvider
  ): Promise<Record<string, string>> {
    // Ollama doesn't need credentials (local model)
    if (provider === 'ollama') {
      return {}
    }

    // Validate provider is supported
    if (provider !== 'openai' && provider !== 'google') {
      throw new Error(`Unsupported provider: ${provider}`)
    }

    // Retrieve API key
    const apiKey = await this.getApiKey(provider)

    // If no key stored, return empty object
    if (!apiKey) {
      return {}
    }

    // Return environment variable with provider-specific name
    const envVarName = ENV_VAR_NAMES[provider]
    return {
      [envVarName]: apiKey,
    }
  }

  /**
   * Delete API key for a provider
   *
   * Removes the stored API key from VSCode's encrypted SecretStorage.
   * Useful for sign-out or provider switching.
   *
   * @param provider - Embedding provider
   * @returns Promise that resolves when key is deleted
   */
  async deleteApiKey(provider: EmbeddingProvider): Promise<void> {
    // Ollama doesn't have credentials to delete
    if (provider === 'ollama') {
      return
    }

    // Validate provider is supported
    if (provider !== 'openai' && provider !== 'google') {
      throw new Error(`Unsupported provider: ${provider}`)
    }

    // Delete encrypted credential
    const secretKey = SECRET_KEYS[provider]
    await this.secrets.delete(secretKey)
  }

  /**
   * Check if API key is stored for a provider
   *
   * Returns true if a credential is stored for the provider.
   * For Ollama, always returns true since no credentials are needed.
   *
   * @param provider - Embedding provider
   * @returns Promise resolving to true if credential exists or not needed
   */
  async hasApiKey(provider: EmbeddingProvider): Promise<boolean> {
    // Ollama doesn't need credentials
    if (provider === 'ollama') {
      return true
    }

    // Check if credential exists
    const apiKey = await this.getApiKey(provider)
    return apiKey !== undefined && apiKey.length > 0
  }
}

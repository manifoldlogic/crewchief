/**
 * Embedding provider auto-detection and configuration
 *
 * Detects available embedding providers with the following priority:
 * 1. MAPROOM_EMBEDDING_PROVIDER env var (explicit override)
 * 2. Ollama (if running on localhost:11434)
 * 3. OpenAI (if OPENAI_API_KEY set)
 * 4. Google Vertex AI (if GOOGLE_PROJECT_ID and GOOGLE_APPLICATION_CREDENTIALS set)
 */

export interface ProviderConfig {
  provider: string // "ollama" | "openai" | "google"
  dimension: number // 768 or 1536
  available: boolean
}

/**
 * Detect available embedding provider
 *
 * Priority:
 * 1. MAPROOM_EMBEDDING_PROVIDER env var (explicit override)
 * 2. Ollama (if running on localhost:11434)
 * 3. OpenAI (if OPENAI_API_KEY set)
 * 4. Google (if GOOGLE_PROJECT_ID and GOOGLE_APPLICATION_CREDENTIALS set)
 *
 * @returns Provider configuration
 * @throws Error if no provider available
 */
export async function detectProvider(): Promise<ProviderConfig> {
  // 1. Check explicit override
  const explicitProvider = process.env.MAPROOM_EMBEDDING_PROVIDER?.toLowerCase()
  if (explicitProvider) {
    console.log(`Using explicit provider: ${explicitProvider}`)
    return validateExplicitProvider(explicitProvider)
  }

  // 2. Try Ollama auto-detection
  console.log('Auto-detecting embedding provider...')
  if (await isOllamaAvailable()) {
    const endpoint = getOllamaEndpoint() || 'localhost:11434'
    console.log(`✓ Ollama detected at ${endpoint}`)
    return {
      provider: 'ollama',
      dimension: 768,
      available: true,
    }
  }

  // 3. Try OpenAI
  if (process.env.OPENAI_API_KEY) {
    console.log('✓ Using OpenAI (OPENAI_API_KEY found)')
    return {
      provider: 'openai',
      dimension: 1536,
      available: true,
    }
  }

  // 4. Try Google
  if (process.env.GOOGLE_PROJECT_ID && process.env.GOOGLE_APPLICATION_CREDENTIALS) {
    console.log('✓ Using Google Vertex AI (GOOGLE_PROJECT_ID found)')
    return {
      provider: 'google',
      dimension: 768,
      available: true,
    }
  }

  // No provider available
  throw new Error(
    'No embedding provider available. Options:\n' +
    '  1. Install Ollama: https://ollama.ai (zero-config)\n' +
    '  2. Set OPENAI_API_KEY environment variable\n' +
    '  3. Configure Google Vertex AI (see docs/providers/google-vertex-ai-setup.md)\n' +
    '  4. Set MAPROOM_EMBEDDING_PROVIDER explicitly (ollama|openai|google)'
  )
}

/**
 * Detected Ollama endpoint (cached after successful detection)
 */
let detectedOllamaEndpoint: string | null = null

/**
 * Get the detected Ollama endpoint URL
 *
 * @returns The Ollama base URL (e.g., "http://localhost:11434" or "http://host.docker.internal:11434")
 */
export function getOllamaEndpoint(): string | null {
  return detectedOllamaEndpoint
}

/**
 * Check if Ollama is running and has the mxbai-embed-large model
 *
 * Checks multiple endpoints in priority order:
 * 1. localhost:11434 (native development)
 * 2. host.docker.internal:11434 (Docker/DevContainer)
 *
 * @returns True if Ollama is available and properly configured
 */
export async function isOllamaAvailable(): Promise<boolean> {
  // Endpoints to try in priority order
  const endpoints = [
    'http://localhost:11434',
    'http://host.docker.internal:11434',
  ]

  for (const endpoint of endpoints) {
    try {
      const controller = new AbortController()
      const timeout = setTimeout(() => controller.abort(), 2000) // 2s timeout

      const response = await fetch(`${endpoint}/api/tags`, {
        method: 'GET',
        signal: controller.signal,
      })

      clearTimeout(timeout)

      if (response.ok) {
        const data = await response.json()
        // Verify mxbai-embed-large model is available
        const models = data.models || []
        const hasEmbedModel = models.some(
          (m: any) => m.name.includes('mxbai-embed-large')
        )

        if (!hasEmbedModel) {
          console.warn(
            `⚠ Ollama is running at ${endpoint} but mxbai-embed-large model not found. ` +
            'Run: ollama pull mxbai-embed-large'
          )
          return false
        }

        // Cache the detected endpoint for use by daemon
        detectedOllamaEndpoint = endpoint
        return true
      }
    } catch (error) {
      // Connection refused, timeout, or network error - try next endpoint
      continue
    }
  }

  return false
}

/**
 * Validate and return explicit provider configuration
 *
 * @param provider - Provider name from MAPROOM_EMBEDDING_PROVIDER env var
 * @returns Provider configuration
 * @throws Error if provider is invalid or required env vars missing
 */
export function validateExplicitProvider(provider: string): ProviderConfig {
  switch (provider) {
    case 'ollama':
      // Note: We don't validate Ollama availability here for explicit config
      // User explicitly requested it, so trust them
      return { provider: 'ollama', dimension: 768, available: true }

    case 'openai':
      if (!process.env.OPENAI_API_KEY) {
        throw new Error(
          'MAPROOM_EMBEDDING_PROVIDER set to "openai" but OPENAI_API_KEY not found. ' +
          'Set OPENAI_API_KEY or use a different provider.'
        )
      }
      return { provider: 'openai', dimension: 1536, available: true }

    case 'google':
      if (!process.env.GOOGLE_PROJECT_ID) {
        throw new Error(
          'MAPROOM_EMBEDDING_PROVIDER set to "google" but GOOGLE_PROJECT_ID not found. ' +
          'See docs/providers/google-vertex-ai-setup.md for setup instructions.'
        )
      }
      if (!process.env.GOOGLE_APPLICATION_CREDENTIALS) {
        throw new Error(
          'MAPROOM_EMBEDDING_PROVIDER set to "google" but GOOGLE_APPLICATION_CREDENTIALS not found. ' +
          'See docs/providers/google-vertex-ai-setup.md for setup instructions.'
        )
      }
      return { provider: 'google', dimension: 768, available: true }

    default:
      throw new Error(
        `Unknown provider: "${provider}". Supported: ollama, openai, google`
      )
  }
}

/**
 * Cached provider configuration (per MCP session)
 */
let cachedProvider: ProviderConfig | null = null

/**
 * Get provider configuration (cached per session)
 *
 * This function caches the provider detection result to avoid
 * re-detecting on every tool call. The cache persists for the
 * lifetime of the MCP session.
 *
 * @returns Provider configuration
 * @throws Error if no provider available
 */
export async function getProviderConfig(): Promise<ProviderConfig> {
  if (!cachedProvider) {
    cachedProvider = await detectProvider()
  }
  return cachedProvider
}

/**
 * Clear provider cache (for testing)
 *
 * This should only be used in tests to reset the cache between
 * test cases. In production, the cache persists for the session.
 */
export function clearProviderCache(): void {
  cachedProvider = null
}

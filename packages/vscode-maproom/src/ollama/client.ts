/**
 * HTTP client for Ollama API operations
 *
 * Provides methods to check if Ollama is running, verify model availability,
 * and pull models with progress streaming. Used by the extension to ensure
 * the required embedding model is available before starting the watch process.
 *
 * SECURITY: Base URL is validated to only allow localhost, 127.0.0.1, or
 * host.docker.internal to prevent SSRF attacks.
 */

/** Timeout for health check requests in milliseconds */
const HEALTH_CHECK_TIMEOUT_MS = 2000

/** Regex pattern for validating model names (security measure) */
const MODEL_NAME_PATTERN = /^[a-z0-9][a-z0-9._-]*(?::[a-z0-9._-]+)?$/i

/** Allowed hostnames for Ollama endpoint (security measure) */
const ALLOWED_HOSTS = ['localhost', '127.0.0.1', 'host.docker.internal']

/** Default Ollama endpoint */
const DEFAULT_ENDPOINT = 'http://127.0.0.1:11434'

/** Fallback endpoint for devcontainers (host machine) */
const DOCKER_HOST_ENDPOINT = 'http://host.docker.internal:11434'

/** Error thrown when model name validation fails */
export class InvalidModelNameError extends Error {
  constructor(modelName: string) {
    super(`Invalid model name format: ${modelName}`)
    this.name = 'InvalidModelNameError'
  }
}

/** Error thrown when Ollama API request fails */
export class OllamaApiError extends Error {
  constructor(
    message: string,
    public readonly statusCode?: number
  ) {
    super(message)
    this.name = 'OllamaApiError'
  }
}

/** Error thrown when endpoint URL is invalid or not allowed */
export class InvalidEndpointError extends Error {
  constructor(endpoint: string) {
    super(
      `Invalid Ollama endpoint: ${endpoint}. Only localhost, 127.0.0.1, and host.docker.internal are allowed.`
    )
    this.name = 'InvalidEndpointError'
  }
}

/** Model information from Ollama API */
export interface OllamaModel {
  name: string
  modified_at?: string
  size?: number
}

/** Response from /api/tags endpoint */
export interface OllamaTagsResponse {
  models?: OllamaModel[]
}

/** Progress event from model pull */
export interface PullProgress {
  status: string
  digest?: string
  total?: number
  completed?: number
}

/**
 * HTTP client for interacting with the Ollama API
 *
 * @example
 * ```typescript
 * // Default endpoint (localhost)
 * const client = new OllamaClient()
 *
 * // Custom endpoint (e.g., for devcontainers)
 * const dockerClient = new OllamaClient('http://host.docker.internal:11434')
 *
 * if (await client.isRunning()) {
 *   if (!await client.hasModel('nomic-embed-text')) {
 *     await client.pullModel('nomic-embed-text', (progress) => {
 *       console.log(progress.status)
 *     })
 *   }
 * }
 * ```
 */
export class OllamaClient {
  /**
   * Base URL for Ollama API
   * SECURITY: Validated to only allow safe hostnames
   */
  private readonly baseUrl: string

  /**
   * Create a new OllamaClient
   *
   * @param endpoint - Ollama API base URL (default: http://127.0.0.1:11434)
   * @throws InvalidEndpointError if endpoint hostname is not in allowed list
   */
  constructor(endpoint: string = DEFAULT_ENDPOINT) {
    this.baseUrl = OllamaClient.validateEndpoint(endpoint)
  }

  /**
   * Validate and normalize an Ollama endpoint URL
   *
   * SECURITY: Only allows localhost, 127.0.0.1, and host.docker.internal
   * to prevent SSRF attacks through user-controlled URLs.
   *
   * @param endpoint - Endpoint URL to validate
   * @returns Normalized endpoint URL (without trailing slash)
   * @throws InvalidEndpointError if hostname is not allowed
   */
  static validateEndpoint(endpoint: string): string {
    try {
      const url = new URL(endpoint)
      if (!ALLOWED_HOSTS.includes(url.hostname)) {
        throw new InvalidEndpointError(endpoint)
      }
      // Return normalized URL without trailing slash
      return `${url.protocol}//${url.host}`
    } catch (e) {
      if (e instanceof InvalidEndpointError) {
        throw e
      }
      throw new InvalidEndpointError(endpoint)
    }
  }

  /**
   * Check if Ollama is running and accessible
   *
   * Performs a health check by calling the /api/tags endpoint with a 2-second timeout.
   * Returns true if Ollama responds, false if connection fails or times out.
   *
   * @returns Promise resolving to true if Ollama is running, false otherwise
   */
  async isRunning(): Promise<boolean> {
    try {
      const response = await fetch(`${this.baseUrl}/api/tags`, {
        signal: AbortSignal.timeout(HEALTH_CHECK_TIMEOUT_MS),
      })
      return response.ok
    } catch {
      // Connection error, timeout, or other failure
      return false
    }
  }

  /**
   * Check if a specific model is available locally
   *
   * Queries the /api/tags endpoint to get the list of installed models,
   * then checks if the requested model (with or without :latest tag) is present.
   *
   * @param name - Model name to check (e.g., 'nomic-embed-text' or 'nomic-embed-text:latest')
   * @returns Promise resolving to true if model exists, false otherwise
   * @throws OllamaApiError if the API request fails
   */
  async hasModel(name: string): Promise<boolean> {
    try {
      const response = await fetch(`${this.baseUrl}/api/tags`)

      if (!response.ok) {
        throw new OllamaApiError(
          `Failed to fetch models: ${response.statusText}`,
          response.status
        )
      }

      const data = (await response.json()) as OllamaTagsResponse
      const models = data.models || []

      // Check both exact match and with :latest suffix
      return models.some(
        (m) => m.name === name || m.name === `${name}:latest`
      )
    } catch (error) {
      if (error instanceof OllamaApiError) {
        throw error
      }
      throw new OllamaApiError(
        `Failed to check model: ${error instanceof Error ? error.message : 'Unknown error'}`
      )
    }
  }

  /**
   * Pull a model from Ollama registry
   *
   * Streams the download progress through the onProgress callback.
   * The callback receives status updates as NDJSON events are received.
   *
   * @param name - Model name to pull (e.g., 'nomic-embed-text')
   * @param onProgress - Optional callback for progress updates
   * @throws InvalidModelNameError if model name doesn't match security pattern
   * @throws OllamaApiError if the pull request fails
   */
  async pullModel(
    name: string,
    onProgress?: (progress: PullProgress) => void
  ): Promise<void> {
    // Validate model name format (SECURITY: prevent injection attacks)
    if (!MODEL_NAME_PATTERN.test(name)) {
      throw new InvalidModelNameError(name)
    }

    let response: Response
    try {
      response = await fetch(`${this.baseUrl}/api/pull`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name }),
      })
    } catch (error) {
      throw new OllamaApiError(
        `Failed to pull model: ${error instanceof Error ? error.message : 'Network error'}`
      )
    }

    if (!response.ok) {
      throw new OllamaApiError(
        `Failed to pull model: ${response.statusText}`,
        response.status
      )
    }

    if (!response.body) {
      throw new OllamaApiError('No response body from pull request')
    }

    // Stream NDJSON progress events
    const reader = response.body.getReader()
    const decoder = new TextDecoder()
    let buffer = ''

    try {
      while (true) {
        const { done, value } = await reader.read()
        if (done) break

        buffer += decoder.decode(value, { stream: true })

        // Process complete lines
        const lines = buffer.split('\n')
        buffer = lines.pop() || '' // Keep incomplete line in buffer

        for (const line of lines) {
          if (!line.trim()) continue

          try {
            const event: PullProgress = JSON.parse(line)
            onProgress?.({
              status: event.status || 'Downloading...',
              digest: event.digest,
              total: event.total,
              completed: event.completed,
            })
          } catch {
            // Ignore malformed JSON lines - they happen during streaming
          }
        }
      }

      // Process any remaining data in buffer
      if (buffer.trim()) {
        try {
          const event: PullProgress = JSON.parse(buffer)
          onProgress?.({
            status: event.status || 'Complete',
            digest: event.digest,
            total: event.total,
            completed: event.completed,
          })
        } catch {
          // Ignore final malformed line
        }
      }
    } finally {
      reader.releaseLock()
    }
  }

  /**
   * Validate a model name against the security pattern
   *
   * Model names must:
   * - Start with a lowercase letter or number
   * - Contain only letters, numbers, dots, underscores, and hyphens
   * - Optionally have a tag suffix after a colon
   *
   * @param name - Model name to validate
   * @returns true if valid, false otherwise
   */
  static isValidModelName(name: string): boolean {
    return MODEL_NAME_PATTERN.test(name)
  }
}

/**
 * Get the configured Ollama endpoint from VS Code settings
 *
 * @returns Ollama endpoint URL from settings, or default if not configured
 */
export function getOllamaEndpoint(): string {
  // Dynamic import to avoid issues in test environments
  try {
    // eslint-disable-next-line @typescript-eslint/no-var-requires
    const vscode = require('vscode') as typeof import('vscode')
    const config = vscode.workspace.getConfiguration('maproom')
    return (config.get('ollama.endpoint') as string | undefined) || DEFAULT_ENDPOINT
  } catch {
    // In test environment where vscode is not available
    return DEFAULT_ENDPOINT
  }
}

/**
 * Create an OllamaClient using the configured endpoint from VS Code settings
 *
 * @returns OllamaClient configured with the user's endpoint setting
 */
export function createOllamaClient(): OllamaClient {
  return new OllamaClient(getOllamaEndpoint())
}

/**
 * Get list of Ollama endpoints to try, in priority order
 *
 * This is the SINGLE SOURCE OF TRUTH for Ollama endpoint fallback logic.
 * All endpoint detection should use this function to ensure consistency.
 *
 * Priority order:
 * 1. User-configured endpoint (from VS Code settings maproom.ollama.endpoint)
 * 2. localhost:11434 (default)
 * 3. host.docker.internal:11434 (devcontainer/docker fallback)
 *
 * IMPORTANT: If you need to modify the fallback logic, this is the ONLY place
 * to change. The following functions use this list:
 * - createOllamaClientWithFallback() - returns working client or undefined
 *
 * @returns Array of endpoint URLs to try in order
 */
export function getOllamaEndpointFallbackList(): string[] {
  const configuredEndpoint = getOllamaEndpoint()
  const endpoints = [configuredEndpoint]

  // Add fallback endpoints if not already in the list
  if (configuredEndpoint !== DEFAULT_ENDPOINT) {
    endpoints.push(DEFAULT_ENDPOINT)
  }
  if (!endpoints.includes(DOCKER_HOST_ENDPOINT)) {
    endpoints.push(DOCKER_HOST_ENDPOINT)
  }

  return endpoints
}

/**
 * Result of Ollama detection with fallback
 */
export interface OllamaDetectionResult {
  /** OllamaClient instance if found */
  client: OllamaClient
  /** The endpoint URL that worked (e.g., "http://host.docker.internal:11434") */
  endpoint: string
}

/**
 * Cached detected Ollama endpoint from last successful detection
 * Used by getDetectedOllamaEndpoint() to retrieve without re-probing
 */
let cachedDetectedEndpoint: string | null = null

/**
 * Create an OllamaClient with automatic endpoint detection
 *
 * Uses getOllamaEndpointFallbackList() to try multiple endpoints in order
 * until one responds. This enables zero-config for devcontainers where
 * Ollama runs on the host machine.
 *
 * IMPORTANT: When Ollama is found, the endpoint is cached and can be retrieved
 * via getDetectedOllamaEndpoint() for passing to the Rust binary.
 *
 * @returns Promise resolving to OllamaClient with working endpoint, or undefined if none found
 */
export async function createOllamaClientWithFallback(): Promise<OllamaClient | undefined> {
  const result = await detectOllamaWithFallback()
  return result?.client
}

/**
 * Detect Ollama with fallback and return both client and endpoint
 *
 * Uses getOllamaEndpointFallbackList() to try multiple endpoints in order
 * until one responds. Caches the detected endpoint for later retrieval.
 *
 * @returns Promise resolving to detection result, or undefined if not found
 */
export async function detectOllamaWithFallback(): Promise<OllamaDetectionResult | undefined> {
  const endpoints = getOllamaEndpointFallbackList()

  // Try each endpoint until one works
  for (const endpoint of endpoints) {
    try {
      const client = new OllamaClient(endpoint)
      if (await client.isRunning()) {
        // Cache the detected endpoint
        cachedDetectedEndpoint = endpoint
        return { client, endpoint }
      }
    } catch {
      // Invalid endpoint or connection failed, try next
    }
  }

  return undefined
}

/**
 * Get the last detected Ollama endpoint
 *
 * Returns the endpoint URL from the last successful Ollama detection.
 * This should be passed to the Rust binary via MAPROOM_EMBEDDING_API_ENDPOINT
 * when Ollama is the configured provider.
 *
 * IMPORTANT: This returns the cached endpoint from the last call to
 * createOllamaClientWithFallback() or detectOllamaWithFallback().
 * If Ollama hasn't been detected yet, returns null.
 *
 * @returns The detected endpoint URL, or null if not detected
 */
export function getDetectedOllamaEndpoint(): string | null {
  return cachedDetectedEndpoint
}

/**
 * Clear the cached Ollama endpoint (for testing)
 */
export function clearDetectedOllamaEndpoint(): void {
  cachedDetectedEndpoint = null
}

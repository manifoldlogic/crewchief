/**
 * Embedding provider auto-detection and configuration
 *
 * Detects available embedding providers with the following priority:
 * 1. MAPROOM_EMBEDDING_PROVIDER env var (explicit override)
 * 2. Ollama (if running on localhost:11434)
 * 3. OpenAI (if OPENAI_API_KEY set)
 * 4. Google Vertex AI (if GOOGLE_PROJECT_ID and GOOGLE_APPLICATION_CREDENTIALS set)
 *
 * AWS Bedrock is supported but never auto-selected. Its credentials come from
 * the ambient AWS chain (instance roles, SSO, EKS service accounts), so
 * "credentials are present" is true on a great many machines that did not
 * intend to spend money embedding — it must be requested explicitly via
 * MAPROOM_EMBEDDING_PROVIDER. Detected AWS credentials are surfaced as a hint
 * in the no-provider error instead.
 */

export interface ProviderConfig {
  provider: string // "ollama" | "openai" | "google" | "bedrock"
  dimension: number // 768, 1024, or 1536
  available: boolean
}

/**
 * Bedrock model id -> embedding dimension.
 *
 * Mirrors `infer_bedrock_dimension` in crates/maproom/src/embedding/bedrock.rs.
 * Rust is the source of truth; keep these in sync.
 */
const BEDROCK_MODEL_DIMENSIONS: Record<string, number> = {
  'amazon.titan-embed-text-v2': 1024,
  'amazon.titan-embed-text-v1': 1536,
  'amazon.titan-embed-g1-text': 1536,
  'cohere.embed-english-v3': 1024,
  'cohere.embed-multilingual-v3': 1024,
}

/**
 * Default Bedrock model, matching BedrockProvider::DEFAULT_MODEL in Rust.
 */
const BEDROCK_DEFAULT_MODEL = 'amazon.titan-embed-text-v2:0'

/**
 * Sanity ceiling for a manually supplied embedding dimension.
 *
 * Far above any shipping model (Titan tops out at 1536, OpenAI at 3072) and
 * far below the point where Number() loses integer precision. maproom's own
 * vector storage only supports 768, 1024, and 1536; this bound exists to
 * reject nonsense early with a clear message rather than to define the
 * supported set.
 */
const MAX_EMBEDDING_DIMENSION = 65_536

/**
 * Resolve the embedding dimension for a Bedrock model id.
 *
 * Strips cross-region inference-profile prefixes (`us.`, `eu.`, …) and ARN
 * resource paths so all spellings of the same model resolve alike.
 *
 * @param model - Bedrock model id, inference profile id, or ARN
 * @returns The dimension, or null if the model is not recognized
 */
export function inferBedrockDimension(model: string): number | null {
  const withoutArn = model.split('/').pop() ?? model
  const normalized = withoutArn.replace(
    /^(us|eu|apac|us-gov|ca|sa|jp|au)\./,
    ''
  )
  for (const [prefix, dimension] of Object.entries(BEDROCK_MODEL_DIMENSIONS)) {
    if (matchesModelPrefix(normalized, prefix)) {
      return dimension
    }
  }
  return null
}

/**
 * Whether `model` is `prefix`, or `prefix` followed by a version delimiter.
 *
 * A bare `startsWith` is too loose: it accepts `amazon.titan-embed-text-v20` as
 * Titan v2 and returns 1024 for a model id that does not exist. An
 * unrecognized id is deliberately a hard error, because a wrong dimension
 * builds an index that succeeds and then returns nothing — so a typo must not
 * slip through as a near-match.
 *
 * `:` and `-` still continue the match so the real suffixed ids AWS ships
 * (`amazon.titan-embed-text-v2:0`, `amazon.titan-embed-g1-text-02`) resolve.
 *
 * Mirrors `matches_model_prefix` in crates/maproom/src/embedding/bedrock.rs.
 */
function matchesModelPrefix(model: string, prefix: string): boolean {
  if (!model.startsWith(prefix)) return false
  const rest = model.slice(prefix.length)
  return rest === '' || rest.startsWith(':') || rest.startsWith('-')
}

/**
 * Describe the AWS credential source visible in the environment, if any.
 *
 * Deliberately env-var only: the full chain makes network calls (IMDS, STS)
 * and this is used on an error path where a two-second timeout is not welcome.
 *
 * @returns The variable that indicates AWS credentials, or null
 */
export function detectedAwsEnvironment(): string | null {
  if (process.env.AWS_ACCESS_KEY_ID) return 'AWS_ACCESS_KEY_ID'
  if (process.env.AWS_PROFILE) return 'AWS_PROFILE'
  if (process.env.AWS_WEB_IDENTITY_TOKEN_FILE) return 'AWS_WEB_IDENTITY_TOKEN_FILE'
  if (
    process.env.AWS_CONTAINER_CREDENTIALS_RELATIVE_URI ||
    process.env.AWS_CONTAINER_CREDENTIALS_FULL_URI
  ) {
    return 'container credentials endpoint'
  }
  return null
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
  const awsSource = detectedAwsEnvironment()
  const awsHint = awsSource
    ? `\n\nAWS credentials detected (${awsSource}). To use Amazon Bedrock:\n` +
      '  export MAPROOM_EMBEDDING_PROVIDER=bedrock'
    : ''
  throw new Error(
    'No embedding provider available. Options:\n' +
    '  1. Install Ollama: https://ollama.ai (zero-config)\n' +
    '  2. Set MAPROOM_EMBEDDING_PROVIDER=bedrock for AWS Bedrock (uses the standard AWS credential chain)\n' +
    '  3. Set OPENAI_API_KEY environment variable\n' +
    '  4. Configure Google Vertex AI (see docs/providers/google-vertex-ai-setup.md)\n' +
    '  5. Set MAPROOM_EMBEDDING_PROVIDER explicitly (ollama|openai|google|bedrock)' +
    awsHint
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

    case 'bedrock':
    case 'aws':
    case 'aws-bedrock': {
      // No credential check here. Bedrock resolves credentials through the
      // full AWS chain — instance roles and EKS service accounts leave no
      // environment variable to test for — so probing env vars would reject
      // working setups. The Rust provider reports precisely what it tried if
      // resolution actually fails.
      const model = process.env.MAPROOM_EMBEDDING_MODEL || BEDROCK_DEFAULT_MODEL
      const explicitDimension = process.env.MAPROOM_EMBEDDING_DIMENSION

      if (explicitDimension !== undefined) {
        const trimmed = explicitDimension.trim()
        // Number.parseInt('1024junk') is 1024, so it would accept a malformed
        // value and silently index at a plausible-looking width. Require the
        // whole string to be a positive integer instead. This also rejects
        // scientific notation ('1e300'), decimals, and signs.
        //
        // The digit check alone is not enough: '9007199254740993' is all
        // digits but Number() rounds it to ...992, and a long-enough digit
        // string becomes 1e+30. Bound it to a safe integer within a range no
        // real embedding model approaches — maproom itself stores only 768,
        // 1024, and 1536, and the Rust provider enforces the per-model width.
        const parsed = Number(trimmed)
        if (
          !/^[1-9]\d*$/.test(trimmed) ||
          !Number.isSafeInteger(parsed) ||
          parsed > MAX_EMBEDDING_DIMENSION
        ) {
          throw new Error(
            `MAPROOM_EMBEDDING_DIMENSION must be a positive integer no greater ` +
            `than ${MAX_EMBEDDING_DIMENSION}, got "${explicitDimension}". ` +
            'See docs/providers/aws-bedrock-setup.md.'
          )
        }
        return { provider: 'bedrock', dimension: parsed, available: true }
      }

      const dimension = inferBedrockDimension(model)
      if (dimension === null) {
        throw new Error(
          `MAPROOM_EMBEDDING_PROVIDER set to "bedrock" but the dimension for model ` +
          `"${model}" could not be inferred. Set MAPROOM_EMBEDDING_DIMENSION to the ` +
          'model\'s output width. See docs/providers/aws-bedrock-setup.md.'
        )
      }
      return { provider: 'bedrock', dimension, available: true }
    }

    default:
      throw new Error(
        `Unknown provider: "${provider}". Supported: ollama, openai, google, bedrock`
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

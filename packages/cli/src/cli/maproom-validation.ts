import { logger } from '../utils/logger.js'

/**
 * Result of environment validation
 */
export interface ValidationResult {
  valid: boolean
  errors: string[]
  warnings: string[]
}

/**
 * Valid embedding providers
 */
const VALID_PROVIDERS = ['ollama', 'openai', 'google'] as const

/**
 * Validate maproom environment configuration
 * Checks database URL and embedding provider settings
 * Fast validation (<10ms) using only environment variable checks
 *
 * @returns ValidationResult with errors and warnings
 */
export function validateMaproomEnvironment(): ValidationResult {
  const errors: string[] = []
  const warnings: string[] = []

  // Database URL validation - maproom uses ~/.maproom/maproom.db by default
  // MAPROOM_DATABASE_URL can optionally override this location
  // No validation needed - the default is always available

  // Embedding provider validation
  const provider = process.env.MAPROOM_EMBEDDING_PROVIDER

  if (!provider) {
    warnings.push(
      'MAPROOM_EMBEDDING_PROVIDER not set. Embeddings will not be generated during indexing.\n' +
        'Set to "ollama", "openai", or "google" to enable semantic search.',
    )
  } else {
    // Validate provider value
    if (!VALID_PROVIDERS.includes(provider as (typeof VALID_PROVIDERS)[number])) {
      errors.push(`Invalid embedding provider: "${provider}". Must be one of: ${VALID_PROVIDERS.join(', ')}`)
    } else {
      // Provider-specific validation
      if (provider === 'openai') {
        const hasOpenAIKey = !!process.env.OPENAI_API_KEY || !!process.env.MAPROOM_OPENAI_API_KEY

        if (!hasOpenAIKey) {
          errors.push(
            'OpenAI provider requires OPENAI_API_KEY or MAPROOM_OPENAI_API_KEY environment variable.\n' +
              'Get your API key from: https://platform.openai.com/api-keys',
          )
        }
      } else if (provider === 'google') {
        const hasGoogleProject = !!process.env.GOOGLE_PROJECT_ID || !!process.env.MAPROOM_GOOGLE_PROJECT_ID

        if (!hasGoogleProject) {
          errors.push(
            'Google provider requires GOOGLE_PROJECT_ID or MAPROOM_GOOGLE_PROJECT_ID environment variable.\n' +
              'See: https://cloud.google.com/vertex-ai/docs/start/cloud-environment',
          )
        }
      }
      // Ollama provider requires no additional configuration
    }
  }

  return {
    valid: errors.length === 0,
    errors,
    warnings,
  }
}

/**
 * Display validation result to user with formatted output
 * Shows errors with ❌ emoji and warnings with ⚠️ emoji
 * Never displays credential values (security requirement)
 *
 * @param result - ValidationResult from validateMaproomEnvironment()
 */
export function displayValidationResult(result: ValidationResult): void {
  if (result.errors.length > 0) {
    logger.error('❌ Environment validation failed:\n')
    result.errors.forEach((error) => {
      logger.error(`  ${error}\n`)
    })
  }

  if (result.warnings.length > 0) {
    result.warnings.forEach((warning) => {
      logger.warn(`⚠️  ${warning}\n`)
    })
  }

  if (!result.valid) {
    logger.error('\n💡 Fix the errors above and try again.')
  }
}

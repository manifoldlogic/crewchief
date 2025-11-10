/**
 * Sensitive data sanitization for competition framework
 * Protects against credential exposure in logs and reports
 */

export function sanitizeDbUrl(url: string): string {
  // postgresql://user:password@host:port/db
  //            ^^^^^^^^^^^^^ redact this part
  // Match everything between :// and the last @ (greedy match)
  // This handles passwords with @ symbols
  return url.replace(/:\/\/.*@/, '://***:***@')
}

export function sanitizeEnvironment(env: Record<string, string | undefined>): Record<string, string> {
  const sanitized: Record<string, string> = {}

  // Redact sensitive variables
  const sensitiveKeys = [
    'MAPROOM_DATABASE_URL',
    'DATABASE_URL',
    'ANTHROPIC_API_KEY',
    'OPENAI_API_KEY',
    'PASSWORD',
    'SECRET',
  ]

  for (const [key, value] of Object.entries(env)) {
    if (value === undefined) {
      continue
    }

    // Check if key matches any sensitive pattern
    const isSensitive = sensitiveKeys.some((pattern) => key.includes(pattern))

    if (isSensitive) {
      if (key.includes('URL')) {
        sanitized[key] = sanitizeDbUrl(value)
      } else {
        sanitized[key] = '***'
      }
    } else {
      sanitized[key] = value
    }
  }

  return sanitized
}

export interface ParticipantResult {
  variantId: string
  success: boolean
  environment?: Record<string, string>
  [key: string]: unknown
}

export function sanitizeAgentResult(result: ParticipantResult): ParticipantResult {
  return {
    ...result,
    environment: result.environment ? sanitizeEnvironment(result.environment) : undefined,
  }
}

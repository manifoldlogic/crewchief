/**
 * Resource limits for competition framework
 * Protects against resource exhaustion attacks
 */

export const SECURITY_LIMITS = {
  MAX_VARIANTS: 50,
  MAX_PARALLEL_AGENTS: 10,
  MAX_TIMEOUT: 600_000, // 10 minutes
  MIN_TIMEOUT: 30_000, // 30 seconds
  MAX_COMPETITION_AGE: 86400_000, // 24 hours
} as const

export interface CompetitionConfig {
  variants: string[]
  parallelAgents?: number
  timeout?: number
  [key: string]: unknown
}

export function validateCompetitionConfig(config: CompetitionConfig): void {
  // Limit number of variants
  if (config.variants.length > SECURITY_LIMITS.MAX_VARIANTS) {
    throw new Error(`Too many variants: ${config.variants.length} exceeds maximum of ${SECURITY_LIMITS.MAX_VARIANTS}`)
  }

  // Limit parallel execution
  if (config.parallelAgents && config.parallelAgents > SECURITY_LIMITS.MAX_PARALLEL_AGENTS) {
    throw new Error(
      `Too many parallel agents: ${config.parallelAgents} exceeds maximum of ${SECURITY_LIMITS.MAX_PARALLEL_AGENTS}`,
    )
  }

  // Validate timeout range
  if (config.timeout) {
    if (config.timeout < SECURITY_LIMITS.MIN_TIMEOUT) {
      throw new Error(`Timeout too short: minimum ${SECURITY_LIMITS.MIN_TIMEOUT}ms`)
    }
    if (config.timeout > SECURITY_LIMITS.MAX_TIMEOUT) {
      throw new Error(`Timeout too long: maximum ${SECURITY_LIMITS.MAX_TIMEOUT}ms`)
    }
  }
}

export interface VariantEnvironment {
  variant: { id: string; name: string }
  worktreePath: string
  worktreeName: string
  [key: string]: unknown
}

export interface ParticipantResult {
  variantId: string
  success: boolean
  [key: string]: unknown
}

export async function runAgentsInParallel<T extends VariantEnvironment, R extends ParticipantResult>(
  envs: T[],
  runTask: (env: T) => Promise<R>,
): Promise<R[]> {
  const results: R[] = []

  // Process in batches to respect MAX_PARALLEL_AGENTS
  for (let i = 0; i < envs.length; i += SECURITY_LIMITS.MAX_PARALLEL_AGENTS) {
    const batch = envs.slice(i, i + SECURITY_LIMITS.MAX_PARALLEL_AGENTS)

    console.log(
      `Running batch ${Math.floor(i / SECURITY_LIMITS.MAX_PARALLEL_AGENTS) + 1} of ${Math.ceil(envs.length / SECURITY_LIMITS.MAX_PARALLEL_AGENTS)}`,
    )

    const batchResults = await Promise.all(batch.map((env) => runTask(env)))

    results.push(...batchResults)
  }

  return results
}

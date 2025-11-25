/**
 * Daemon Singleton Management
 *
 * Provides singleton daemon client for MCP server, ensuring one long-running
 * daemon instance serves all search requests rather than spawning a new process
 * for each operation.
 *
 * Key features:
 * - Lazy initialization (daemon starts on first request)
 * - Graceful shutdown on SIGTERM
 * - Auto-restart on crash (exponential backoff, circuit breaker)
 * - Environment variable whitelisting for security
 *
 * Architecture:
 * - Module-level variable ensures single daemon per MCP server process
 * - Binary discovery via existing findMaproomBinary() function
 * - Configuration inherits from existing MCP server environment
 */

import { DaemonClient } from '@crewchief/daemon-client'
import { findMaproomBinary } from './utils/process.js'

/**
 * Singleton daemon client instance
 * @internal
 */
let daemonClient: DaemonClient | null = null

/**
 * Get or create daemon client singleton
 *
 * Lazy initialization - daemon only starts on first call.
 * Subsequent calls return the same instance.
 *
 * @returns DaemonClient instance
 * @throws Error if binary not found or MAPROOM_DATABASE_URL missing
 *
 * @example
 * ```typescript
 * const daemon = getDaemonClient()
 * const results = await daemon.search({ query: 'test', repo: 'crewchief' })
 * ```
 */
export function getDaemonClient(): DaemonClient {
  if (!daemonClient) {
    // Discover binary location using existing logic
    const binaryPath = findMaproomBinary()
    if (!binaryPath) {
      throw new Error(
        'Maproom binary not found. Please ensure crewchief-maproom is installed or built.'
      )
    }

    // Validate required environment variables
    if (!process.env.MAPROOM_DATABASE_URL) {
      throw new Error(
        'MAPROOM_DATABASE_URL environment variable is required for daemon operation'
      )
    }

    // Create daemon client with configuration
    // Note: args are hardcoded to ['serve'] in DaemonLifecycle
    daemonClient = new DaemonClient({
      binaryPath,
      env: {
        // Required: Database connection
        MAPROOM_DATABASE_URL: process.env.MAPROOM_DATABASE_URL,

        // Optional: Embedding provider credentials
        OPENAI_API_KEY: process.env.OPENAI_API_KEY,
        ANTHROPIC_API_KEY: process.env.ANTHROPIC_API_KEY,
        OLLAMA_BASE_URL: process.env.OLLAMA_BASE_URL,

        // Optional: Logging configuration
        RUST_LOG: process.env.RUST_LOG || 'info',
      },

      // Timeouts
      timeout: 30000, // 30s request timeout (matches old spawning)
      startTimeout: 5000, // 5s daemon start timeout
      shutdownTimeout: 5000, // 5s graceful shutdown timeout

      // Auto-restart configuration
      autoRestart: true, // Enable auto-restart on crash
      maxRestartAttempts: 5, // Circuit breaker after 5 failures
      restartBackoffMs: 1000, // Base delay: 1s, 2s, 4s, 8s, 16s
    })
  }

  return daemonClient
}

/**
 * Close daemon client and cleanup resources
 *
 * Gracefully stops the daemon, waiting for in-flight requests to complete
 * (up to shutdownTimeout). Sets singleton to null for potential restart.
 *
 * Safe to call multiple times (idempotent).
 *
 * @returns Promise that resolves when daemon is stopped
 *
 * @example
 * ```typescript
 * await closeDaemonClient()
 * ```
 */
export async function closeDaemonClient(): Promise<void> {
  if (daemonClient) {
    await daemonClient.stop()
    daemonClient = null
  }
}

/**
 * Graceful shutdown handler for SIGTERM
 *
 * Ensures daemon stops cleanly before process exit.
 * Registered automatically on module load.
 */
process.on('SIGTERM', async () => {
  console.log('Received SIGTERM, shutting down daemon...')
  try {
    await closeDaemonClient()
    console.log('Daemon shutdown complete')
    process.exit(0)
  } catch (error) {
    console.error('Error during daemon shutdown:', error)
    process.exit(1)
  }
})

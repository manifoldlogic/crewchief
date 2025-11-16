/**
 * Process crash recovery with exponential backoff and circuit breaker
 *
 * Implements intelligent restart logic for crashed watch processes:
 * - Exponential backoff: 1s, 2s, 4s, 8s, 16s (max)
 * - Circuit breaker with states: CLOSED, OPEN, HALF_OPEN
 * - Automatic reset after 60s successful runtime
 * - Maximum 5 restart attempts before giving up
 *
 * Circuit breaker states:
 * - CLOSED: Normal operation, allowing restarts with backoff
 * - OPEN: Max attempts exceeded, no restarts allowed
 * - HALF_OPEN: After manual reset, testing if process is stable
 */

/**
 * Circuit breaker state
 */
export type CircuitState = 'CLOSED' | 'OPEN' | 'HALF_OPEN'

/**
 * Crash recovery configuration
 */
export interface RecoveryConfig {
  /** Maximum number of restart attempts */
  maxAttempts?: number
  /** Maximum backoff delay in milliseconds */
  maxBackoffMs?: number
  /** Time in milliseconds of successful runtime before resetting attempt counter */
  successResetMs?: number
}

/**
 * Default recovery configuration
 */
const DEFAULT_CONFIG: Required<RecoveryConfig> = {
  maxAttempts: 5,
  maxBackoffMs: 16000,
  successResetMs: 60000,
}

/**
 * Crash recovery class with circuit breaker pattern
 *
 * Manages automatic restart attempts with exponential backoff and prevents
 * infinite restart loops through circuit breaker pattern.
 */
export class CrashRecovery {
  private attemptCount = 0
  private lastRestartTime: number | null = null
  private resetTimer: NodeJS.Timeout | null = null
  private state: CircuitState = 'CLOSED'
  private readonly config: Required<RecoveryConfig>

  /**
   * Create a new crash recovery instance
   *
   * @param config - Optional recovery configuration
   */
  constructor(config: RecoveryConfig = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config }
  }

  /**
   * Handle a process crash and attempt recovery
   *
   * Implements exponential backoff and circuit breaker logic. Returns true if
   * restart was attempted, false if circuit is open or max attempts exceeded.
   *
   * @param processName - Name of crashed process (for logging)
   * @param exitCode - Process exit code
   * @param signal - Signal that caused exit (if any)
   * @param restartFn - Async function to restart the process
   * @returns Promise resolving to true if restart attempted, false if blocked
   */
  async handleCrash(
    processName: string,
    exitCode: number,
    signal: string | null,
    restartFn: () => Promise<void>
  ): Promise<boolean> {
    // Check circuit breaker state
    if (this.state === 'OPEN') {
      console.log(
        `[CrashRecovery] Circuit OPEN for ${processName}, restart blocked (${this.attemptCount}/${this.config.maxAttempts} attempts)`
      )
      return false
    }

    // Increment attempt counter
    this.attemptCount++

    console.log(
      `[CrashRecovery] ${processName} crashed (exit: ${exitCode}${signal ? `, signal: ${signal}` : ''}) - attempt ${this.attemptCount}/${this.config.maxAttempts}`
    )

    // Check if max attempts exceeded
    if (this.attemptCount >= this.config.maxAttempts) {
      console.log(
        `[CrashRecovery] Max attempts (${this.config.maxAttempts}) exceeded for ${processName}, opening circuit`
      )
      this.state = 'OPEN'
      return false
    }

    // Calculate backoff delay
    const backoffMs = this.calculateBackoff(this.attemptCount)
    console.log(`[CrashRecovery] Waiting ${backoffMs}ms before restart...`)

    // Wait for backoff period
    await this.sleep(backoffMs)

    // Attempt restart
    try {
      console.log(`[CrashRecovery] Attempting to restart ${processName}...`)
      this.lastRestartTime = Date.now()

      await restartFn()

      console.log(`[CrashRecovery] ${processName} restarted successfully`)

      // Schedule reset timer if this is first successful restart
      this.scheduleResetTimer()

      // Transition to HALF_OPEN after first successful restart
      if (this.state === 'CLOSED' && this.attemptCount > 0) {
        this.state = 'HALF_OPEN'
        console.log(`[CrashRecovery] Circuit transitioned to HALF_OPEN`)
      }

      return true
    } catch (error: any) {
      console.error(
        `[CrashRecovery] Failed to restart ${processName}: ${error.message}`
      )
      // Don't throw - let circuit breaker handle retry logic
      return false
    }
  }

  /**
   * Calculate exponential backoff delay
   *
   * Implements backoff sequence: 1s, 2s, 4s, 8s, 16s (capped at maxBackoffMs)
   *
   * @param attempt - Current attempt number (1-indexed)
   * @returns Backoff delay in milliseconds
   */
  private calculateBackoff(attempt: number): number {
    // Exponential: 2^(attempt-1) * 1000ms
    // attempt 1: 2^0 * 1000 = 1000ms
    // attempt 2: 2^1 * 1000 = 2000ms
    // attempt 3: 2^2 * 1000 = 4000ms
    // attempt 4: 2^3 * 1000 = 8000ms
    // attempt 5: 2^4 * 1000 = 16000ms
    const exponentialMs = Math.pow(2, attempt - 1) * 1000

    // Cap at maxBackoffMs
    return Math.min(exponentialMs, this.config.maxBackoffMs)
  }

  /**
   * Schedule reset timer
   *
   * If process runs successfully for successResetMs, reset attempt counter
   * and transition circuit back to CLOSED state.
   */
  private scheduleResetTimer(): void {
    // Clear existing timer
    if (this.resetTimer) {
      clearTimeout(this.resetTimer)
    }

    // Schedule new reset timer
    this.resetTimer = setTimeout(() => {
      if (this.lastRestartTime) {
        const runtimeMs = Date.now() - this.lastRestartTime
        if (runtimeMs >= this.config.successResetMs) {
          console.log(
            `[CrashRecovery] Process stable for ${runtimeMs}ms, resetting attempt counter`
          )
          this.attemptCount = 0
          this.state = 'CLOSED'
          this.resetTimer = null
        }
      }
    }, this.config.successResetMs)
  }

  /**
   * Manually reset the recovery state
   *
   * Clears attempt counter and transitions circuit to CLOSED state.
   * Used when user manually restarts the process.
   */
  reset(): void {
    console.log('[CrashRecovery] Manual reset requested')

    // Clear reset timer
    if (this.resetTimer) {
      clearTimeout(this.resetTimer)
      this.resetTimer = null
    }

    // Reset state
    this.attemptCount = 0
    this.lastRestartTime = null
    this.state = 'CLOSED'

    console.log('[CrashRecovery] Recovery state reset to CLOSED')
  }

  /**
   * Get current circuit breaker state
   *
   * @returns Current circuit state
   */
  getState(): CircuitState {
    return this.state
  }

  /**
   * Get current attempt count
   *
   * @returns Number of restart attempts
   */
  getAttemptCount(): number {
    return this.attemptCount
  }

  /**
   * Check if recovery is blocked
   *
   * @returns true if circuit is OPEN (max attempts exceeded)
   */
  isBlocked(): boolean {
    return this.state === 'OPEN'
  }

  /**
   * Dispose resources
   *
   * Clears reset timer and resets state.
   */
  dispose(): void {
    if (this.resetTimer) {
      clearTimeout(this.resetTimer)
      this.resetTimer = null
    }

    this.attemptCount = 0
    this.lastRestartTime = null
    this.state = 'CLOSED'
  }

  /**
   * Sleep for specified milliseconds
   *
   * @param ms - Milliseconds to sleep
   */
  private sleep(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms))
  }
}

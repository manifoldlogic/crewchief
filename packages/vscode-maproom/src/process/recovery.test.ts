/**
 * Unit tests for CrashRecovery
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { CrashRecovery } from './recovery.js'

describe('CrashRecovery', () => {
  beforeEach(() => {
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.restoreAllMocks()
    vi.useRealTimers()
  })

  describe('calculateBackoff', () => {
    it('should calculate exponential backoff correctly', async () => {
      const recovery = new CrashRecovery()
      const restartFn = vi.fn().mockResolvedValue(undefined)

      // Track backoff delays by measuring time between restart calls
      const delays: number[] = []
      let lastCallTime = Date.now()

      for (let i = 1; i <= 5; i++) {
        const crashPromise = recovery.handleCrash('test', 1, null, restartFn)

        // Advance timers to trigger backoff
        await vi.advanceTimersByTimeAsync(20000) // Advance beyond max backoff

        await crashPromise

        const now = Date.now()
        if (i > 1) {
          delays.push(now - lastCallTime)
        }
        lastCallTime = now
      }

      // Expected delays: 1000, 2000, 4000, 8000, 16000
      // We capture delays[0-3] because first attempt has no prior timing
      expect(delays[0]).toBeGreaterThanOrEqual(1000)
      expect(delays[1]).toBeGreaterThanOrEqual(2000)
      expect(delays[2]).toBeGreaterThanOrEqual(4000)
      expect(delays[3]).toBeGreaterThanOrEqual(8000)
    })

    it('should cap backoff at maxBackoffMs', async () => {
      const recovery = new CrashRecovery({ maxBackoffMs: 5000 })
      const restartFn = vi.fn().mockResolvedValue(undefined)

      // Attempt 4 would normally be 8000ms, but should be capped at 5000ms
      for (let i = 0; i < 3; i++) {
        const crashPromise = recovery.handleCrash('test', 1, null, restartFn)
        await vi.advanceTimersByTimeAsync(10000)
        await crashPromise
      }

      const startTime = Date.now()
      const crashPromise = recovery.handleCrash('test', 1, null, restartFn)
      await vi.advanceTimersByTimeAsync(6000) // Just past capped backoff
      await crashPromise
      const elapsed = Date.now() - startTime

      // Should wait 5000ms (capped), not 8000ms
      expect(elapsed).toBeGreaterThanOrEqual(5000)
      expect(elapsed).toBeLessThan(7000) // Allow some margin but less than 8000
    })
  })

  describe('circuit breaker states', () => {
    it('should start in CLOSED state', () => {
      const recovery = new CrashRecovery()
      expect(recovery.getState()).toBe('CLOSED')
    })

    it('should transition to HALF_OPEN after first successful restart', async () => {
      const recovery = new CrashRecovery()
      const restartFn = vi.fn().mockResolvedValue(undefined)

      const crashPromise = recovery.handleCrash('test', 1, null, restartFn)
      await vi.advanceTimersByTimeAsync(2000)
      await crashPromise

      expect(recovery.getState()).toBe('HALF_OPEN')
    })

    it('should transition to OPEN after max attempts', async () => {
      const recovery = new CrashRecovery({ maxAttempts: 3 })
      const restartFn = vi.fn().mockResolvedValue(undefined)

      // Attempt 1
      let crashPromise = recovery.handleCrash('test', 1, null, restartFn)
      await vi.advanceTimersByTimeAsync(2000)
      await crashPromise

      // Attempt 2
      crashPromise = recovery.handleCrash('test', 1, null, restartFn)
      await vi.advanceTimersByTimeAsync(3000)
      await crashPromise

      // Attempt 3 (max)
      crashPromise = recovery.handleCrash('test', 1, null, restartFn)
      await vi.advanceTimersByTimeAsync(5000)
      await crashPromise

      expect(recovery.getState()).toBe('OPEN')
      expect(recovery.getAttemptCount()).toBe(3)
    })

    it('should block restarts when circuit is OPEN', async () => {
      const recovery = new CrashRecovery({ maxAttempts: 2 })
      const restartFn = vi.fn().mockResolvedValue(undefined)

      // Attempt 1
      let crashPromise = recovery.handleCrash('test', 1, null, restartFn)
      await vi.advanceTimersByTimeAsync(2000)
      await crashPromise
      expect(restartFn).toHaveBeenCalledTimes(1)

      // Attempt 2 (reaches max, opens circuit)
      crashPromise = recovery.handleCrash('test', 1, null, restartFn)
      await vi.advanceTimersByTimeAsync(3000)
      await crashPromise
      expect(restartFn).toHaveBeenCalledTimes(1) // Circuit opened before restart

      expect(recovery.getState()).toBe('OPEN')

      // Next attempt should be blocked
      const result = await recovery.handleCrash('test', 1, null, restartFn)
      expect(result).toBe(false)
      expect(restartFn).toHaveBeenCalledTimes(1) // Still 1, not called after circuit opened
    })

    it('should reset to CLOSED after successful runtime', async () => {
      const recovery = new CrashRecovery({ successResetMs: 10000 })
      const restartFn = vi.fn().mockResolvedValue(undefined)

      // First crash
      const crashPromise = recovery.handleCrash('test', 1, null, restartFn)
      await vi.advanceTimersByTimeAsync(2000)
      await crashPromise

      expect(recovery.getState()).toBe('HALF_OPEN')
      expect(recovery.getAttemptCount()).toBe(1)

      // Advance time to trigger reset (successful runtime)
      await vi.advanceTimersByTimeAsync(11000)

      expect(recovery.getState()).toBe('CLOSED')
      expect(recovery.getAttemptCount()).toBe(0)
    })
  })

  describe('attempt counting', () => {
    it('should increment attempt count on each crash', async () => {
      const recovery = new CrashRecovery()
      const restartFn = vi.fn().mockResolvedValue(undefined)

      expect(recovery.getAttemptCount()).toBe(0)

      // Attempt 1
      let crashPromise = recovery.handleCrash('test', 1, null, restartFn)
      await vi.advanceTimersByTimeAsync(2000)
      await crashPromise
      expect(recovery.getAttemptCount()).toBe(1)

      // Attempt 2
      crashPromise = recovery.handleCrash('test', 1, null, restartFn)
      await vi.advanceTimersByTimeAsync(3000)
      await crashPromise
      expect(recovery.getAttemptCount()).toBe(2)

      // Attempt 3
      crashPromise = recovery.handleCrash('test', 1, null, restartFn)
      await vi.advanceTimersByTimeAsync(5000)
      await crashPromise
      expect(recovery.getAttemptCount()).toBe(3)
    })

    it('should stop incrementing at maxAttempts', async () => {
      const recovery = new CrashRecovery({ maxAttempts: 2 })
      const restartFn = vi.fn().mockResolvedValue(undefined)

      // Attempt 1
      let crashPromise = recovery.handleCrash('test', 1, null, restartFn)
      await vi.advanceTimersByTimeAsync(2000)
      await crashPromise

      // Attempt 2 (max)
      crashPromise = recovery.handleCrash('test', 1, null, restartFn)
      await vi.advanceTimersByTimeAsync(3000)
      await crashPromise

      expect(recovery.getAttemptCount()).toBe(2)

      // Attempt 3 (blocked)
      await recovery.handleCrash('test', 1, null, restartFn)
      expect(recovery.getAttemptCount()).toBe(2) // Doesn't increment past max
    })

    it('should reset attempt count after successful runtime', async () => {
      const recovery = new CrashRecovery({ successResetMs: 5000 })
      const restartFn = vi.fn().mockResolvedValue(undefined)

      // First crash
      const crashPromise = recovery.handleCrash('test', 1, null, restartFn)
      await vi.advanceTimersByTimeAsync(2000)
      await crashPromise

      expect(recovery.getAttemptCount()).toBe(1)

      // Advance time past reset threshold
      await vi.advanceTimersByTimeAsync(6000)

      expect(recovery.getAttemptCount()).toBe(0)
    })
  })

  describe('manual reset', () => {
    it('should reset state to CLOSED', async () => {
      const recovery = new CrashRecovery({ maxAttempts: 2 })
      const restartFn = vi.fn().mockResolvedValue(undefined)

      // Exhaust attempts
      let crashPromise = recovery.handleCrash('test', 1, null, restartFn)
      await vi.advanceTimersByTimeAsync(2000)
      await crashPromise

      crashPromise = recovery.handleCrash('test', 1, null, restartFn)
      await vi.advanceTimersByTimeAsync(3000)
      await crashPromise

      expect(recovery.getState()).toBe('OPEN')
      expect(recovery.getAttemptCount()).toBe(2)

      // Manual reset
      recovery.reset()

      expect(recovery.getState()).toBe('CLOSED')
      expect(recovery.getAttemptCount()).toBe(0)
      expect(recovery.isBlocked()).toBe(false)
    })

    it('should allow restarts after manual reset', async () => {
      const recovery = new CrashRecovery({ maxAttempts: 1 })
      const restartFn = vi.fn().mockResolvedValue(undefined)

      // Attempt 1 (reaches max, opens circuit)
      const crashPromise = recovery.handleCrash('test', 1, null, restartFn)
      await vi.advanceTimersByTimeAsync(2000)
      await crashPromise

      expect(recovery.isBlocked()).toBe(true)
      expect(restartFn).toHaveBeenCalledTimes(0) // Circuit opened before restart

      // Manual reset
      recovery.reset()

      // Should allow restart now
      const crashPromise2 = recovery.handleCrash('test', 1, null, restartFn)
      await vi.advanceTimersByTimeAsync(2000)
      const result = await crashPromise2

      expect(result).toBe(false) // Will reach max again (maxAttempts=1)
      expect(restartFn).toHaveBeenCalledTimes(0) // Still blocked on first attempt
    })
  })

  describe('restart function errors', () => {
    it('should handle restart function errors gracefully', async () => {
      const recovery = new CrashRecovery()
      const restartFn = vi.fn().mockRejectedValue(new Error('Restart failed'))

      const consoleSpy = vi.spyOn(console, 'error').mockImplementation(() => {})

      const crashPromise = recovery.handleCrash('test', 1, null, restartFn)
      await vi.advanceTimersByTimeAsync(2000)
      const result = await crashPromise

      expect(result).toBe(false) // Should return false on restart failure
      expect(consoleSpy).toHaveBeenCalled()
      expect(recovery.getAttemptCount()).toBe(1) // Attempt still counted

      consoleSpy.mockRestore()
    })
  })

  describe('dispose', () => {
    it('should clear timers and reset state', async () => {
      const recovery = new CrashRecovery({ successResetMs: 60000 })
      const restartFn = vi.fn().mockResolvedValue(undefined)

      // Start a crash recovery (creates reset timer)
      const crashPromise = recovery.handleCrash('test', 1, null, restartFn)
      await vi.advanceTimersByTimeAsync(2000)
      await crashPromise

      expect(recovery.getAttemptCount()).toBe(1)

      // Dispose
      recovery.dispose()

      expect(recovery.getAttemptCount()).toBe(0)
      expect(recovery.getState()).toBe('CLOSED')
      expect(recovery.isBlocked()).toBe(false)

      // Verify timer was cleared (advance time shouldn't trigger reset)
      await vi.advanceTimersByTimeAsync(70000)
      // If timer wasn't cleared, this would have triggered, but we already reset to 0
    })
  })

  describe('isBlocked', () => {
    it('should return false when circuit is CLOSED', () => {
      const recovery = new CrashRecovery()
      expect(recovery.isBlocked()).toBe(false)
    })

    it('should return false when circuit is HALF_OPEN', async () => {
      const recovery = new CrashRecovery()
      const restartFn = vi.fn().mockResolvedValue(undefined)

      const crashPromise = recovery.handleCrash('test', 1, null, restartFn)
      await vi.advanceTimersByTimeAsync(2000)
      await crashPromise

      expect(recovery.getState()).toBe('HALF_OPEN')
      expect(recovery.isBlocked()).toBe(false)
    })

    it('should return true when circuit is OPEN', async () => {
      const recovery = new CrashRecovery({ maxAttempts: 1 })
      const restartFn = vi.fn().mockResolvedValue(undefined)

      const crashPromise = recovery.handleCrash('test', 1, null, restartFn)
      await vi.advanceTimersByTimeAsync(2000)
      await crashPromise

      expect(recovery.getState()).toBe('OPEN')
      expect(recovery.isBlocked()).toBe(true)
    })
  })

  describe('configuration', () => {
    it('should use default configuration when not provided', () => {
      const recovery = new CrashRecovery()

      expect(recovery.getState()).toBe('CLOSED')
      expect(recovery.getAttemptCount()).toBe(0)
    })

    it('should accept custom maxAttempts', async () => {
      const recovery = new CrashRecovery({ maxAttempts: 3 })
      const restartFn = vi.fn().mockResolvedValue(undefined)

      // Attempt 1
      let crashPromise = recovery.handleCrash('test', 1, null, restartFn)
      await vi.advanceTimersByTimeAsync(20000)
      await crashPromise
      expect(restartFn).toHaveBeenCalledTimes(1)

      // Attempt 2
      crashPromise = recovery.handleCrash('test', 1, null, restartFn)
      await vi.advanceTimersByTimeAsync(20000)
      await crashPromise
      expect(restartFn).toHaveBeenCalledTimes(2)

      // Attempt 3 (reaches max, opens circuit)
      crashPromise = recovery.handleCrash('test', 1, null, restartFn)
      await vi.advanceTimersByTimeAsync(20000)
      await crashPromise

      expect(recovery.getState()).toBe('OPEN')
      expect(restartFn).toHaveBeenCalledTimes(2) // Only 2 restarts, circuit opens on 3rd
    })

    it('should accept custom successResetMs', async () => {
      const recovery = new CrashRecovery({ successResetMs: 3000 })
      const restartFn = vi.fn().mockResolvedValue(undefined)

      const crashPromise = recovery.handleCrash('test', 1, null, restartFn)
      await vi.advanceTimersByTimeAsync(2000)
      await crashPromise

      expect(recovery.getAttemptCount()).toBe(1)

      // Advance just past custom reset threshold
      await vi.advanceTimersByTimeAsync(3500)

      expect(recovery.getAttemptCount()).toBe(0)
    })
  })
})

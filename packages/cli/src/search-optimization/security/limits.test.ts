import { describe, it, expect, vi } from 'vitest'
import { validateCompetitionConfig, runAgentsInParallel, type VariantEnvironment, SECURITY_LIMITS } from './limits'

describe('Resource Limits', () => {
  describe('validateCompetitionConfig', () => {
    it('accepts valid configs', () => {
      expect(() =>
        validateCompetitionConfig({
          variants: Array(10).fill('variant-a'),
          timeout: 180000,
        }),
      ).not.toThrow()

      expect(() =>
        validateCompetitionConfig({
          variants: ['a', 'b', 'c'],
          parallelAgents: 5,
          timeout: 60000,
        }),
      ).not.toThrow()
    })

    it('rejects too many variants', () => {
      expect(() =>
        validateCompetitionConfig({
          variants: Array(51).fill('variant-a'),
        }),
      ).toThrow('Too many variants')

      expect(() =>
        validateCompetitionConfig({
          variants: Array(51).fill('variant-a'),
        }),
      ).toThrow('51 exceeds maximum of 50')
    })

    it('rejects too many parallel agents', () => {
      expect(() =>
        validateCompetitionConfig({
          variants: ['a', 'b'],
          parallelAgents: 11,
        }),
      ).toThrow('Too many parallel agents')

      expect(() =>
        validateCompetitionConfig({
          variants: ['a', 'b'],
          parallelAgents: 11,
        }),
      ).toThrow('11 exceeds maximum of 10')
    })

    it('rejects invalid timeout - too long', () => {
      expect(() =>
        validateCompetitionConfig({
          variants: ['a'],
          timeout: 1000000, // 16+ minutes
        }),
      ).toThrow('Timeout too long')

      expect(() =>
        validateCompetitionConfig({
          variants: ['a'],
          timeout: 1000000,
        }),
      ).toThrow('maximum 600000ms')
    })

    it('rejects invalid timeout - too short', () => {
      expect(() =>
        validateCompetitionConfig({
          variants: ['a'],
          timeout: 1000, // 1 second
        }),
      ).toThrow('Timeout too short')

      expect(() =>
        validateCompetitionConfig({
          variants: ['a'],
          timeout: 1000,
        }),
      ).toThrow('minimum 30000ms')
    })

    it('accepts timeout at boundaries', () => {
      expect(() =>
        validateCompetitionConfig({
          variants: ['a'],
          timeout: SECURITY_LIMITS.MIN_TIMEOUT,
        }),
      ).not.toThrow()

      expect(() =>
        validateCompetitionConfig({
          variants: ['a'],
          timeout: SECURITY_LIMITS.MAX_TIMEOUT,
        }),
      ).not.toThrow()
    })

    it('accepts config without optional fields', () => {
      expect(() =>
        validateCompetitionConfig({
          variants: ['a', 'b', 'c'],
        }),
      ).not.toThrow()
    })
  })

  describe('runAgentsInParallel', () => {
    it('processes agents in batches', async () => {
      const envs: VariantEnvironment[] = Array(25)
        .fill(null)
        .map((_, i) => ({
          variant: { id: `v${i}`, name: `Variant ${i}` },
          worktreePath: `/tmp/v${i}`,
          worktreeName: `v${i}`,
        }))

      const mockRun = vi.fn().mockImplementation(async (env: VariantEnvironment) => ({
        variantId: env.variant.id,
        success: true,
      }))

      // Should process in 3 batches (10 + 10 + 5)
      const results = await runAgentsInParallel(envs, mockRun)

      expect(mockRun).toHaveBeenCalledTimes(25)
      expect(results).toHaveLength(25)
      expect(results.every((r) => r.success)).toBe(true)
    })

    it('processes small batch without exceeding limit', async () => {
      const envs: VariantEnvironment[] = Array(5)
        .fill(null)
        .map((_, i) => ({
          variant: { id: `v${i}`, name: `Variant ${i}` },
          worktreePath: `/tmp/v${i}`,
          worktreeName: `v${i}`,
        }))

      const mockRun = vi.fn().mockImplementation(async (env: VariantEnvironment) => ({
        variantId: env.variant.id,
        success: true,
      }))

      const results = await runAgentsInParallel(envs, mockRun)

      expect(mockRun).toHaveBeenCalledTimes(5)
      expect(results).toHaveLength(5)
    })

    it('respects MAX_PARALLEL_AGENTS limit', async () => {
      // Create 30 envs to test batching
      const envs: VariantEnvironment[] = Array(30)
        .fill(null)
        .map((_, i) => ({
          variant: { id: `v${i}`, name: `Variant ${i}` },
          worktreePath: `/tmp/v${i}`,
          worktreeName: `v${i}`,
        }))

      let maxConcurrent = 0
      let currentConcurrent = 0

      const mockRun = vi.fn().mockImplementation(async (env: VariantEnvironment) => {
        currentConcurrent++
        maxConcurrent = Math.max(maxConcurrent, currentConcurrent)

        // Simulate async work
        await new Promise((resolve) => setTimeout(resolve, 10))

        currentConcurrent--

        return {
          variantId: env.variant.id,
          success: true,
        }
      })

      await runAgentsInParallel(envs, mockRun)

      // Max concurrent should never exceed MAX_PARALLEL_AGENTS
      expect(maxConcurrent).toBeLessThanOrEqual(SECURITY_LIMITS.MAX_PARALLEL_AGENTS)
    })

    it('handles task failures gracefully', async () => {
      const envs: VariantEnvironment[] = Array(5)
        .fill(null)
        .map((_, i) => ({
          variant: { id: `v${i}`, name: `Variant ${i}` },
          worktreePath: `/tmp/v${i}`,
          worktreeName: `v${i}`,
        }))

      const mockRun = vi.fn().mockImplementation(async (env: VariantEnvironment) => {
        if (env.variant.id === 'v2') {
          throw new Error('Task failed')
        }
        return {
          variantId: env.variant.id,
          success: true,
        }
      })

      // Should throw when a task fails
      await expect(runAgentsInParallel(envs, mockRun)).rejects.toThrow('Task failed')
    })
  })
})

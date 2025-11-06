/**
 * Tests for statistical utility functions
 */

import { describe, it, expect } from 'vitest'
import {
  mean,
  stdDev,
  standardError,
  welchTTest,
  cohensD,
  confidenceInterval,
  bonferroniCorrection,
  requiredSampleSize,
  estimatePower
} from './statistics.js'

describe('Basic Statistics', () => {
  describe('mean', () => {
    it('should calculate mean correctly', () => {
      expect(mean([1, 2, 3, 4, 5])).toBe(3)
      expect(mean([10, 20, 30])).toBe(20)
    })

    it('should handle empty array', () => {
      expect(mean([])).toBe(0)
    })

    it('should handle single value', () => {
      expect(mean([42])).toBe(42)
    })
  })

  describe('stdDev', () => {
    it('should calculate standard deviation', () => {
      const sd = stdDev([2, 4, 6, 8])
      expect(sd).toBeCloseTo(2.582, 2)
    })

    it('should handle small samples', () => {
      expect(stdDev([5])).toBe(0)
      expect(stdDev([])).toBe(0)
    })

    it('should handle uniform values', () => {
      expect(stdDev([5, 5, 5, 5])).toBe(0)
    })
  })

  describe('standardError', () => {
    it('should calculate standard error', () => {
      const se = standardError([2, 4, 6, 8])
      expect(se).toBeCloseTo(1.291, 2) // sd/sqrt(n) = 2.582/sqrt(4)
    })

    it('should handle empty array', () => {
      expect(standardError([])).toBe(0)
    })
  })
})

describe('Welch\'s t-test', () => {
  it('should detect significant difference between two samples', () => {
    const sample1 = [10, 12, 11, 13, 12, 11, 10, 12] // mean ≈ 11.375
    const sample2 = [15, 16, 17, 15, 16, 15, 17, 16] // mean ≈ 15.875

    const result = welchTTest(sample1, sample2)

    expect(result.t).toBeLessThan(-5) // Negative because sample1 < sample2
    expect(result.df).toBeGreaterThan(0)
    expect(result.p_value).toBeLessThan(0.01) // Highly significant
  })

  it('should not detect difference between similar samples', () => {
    const sample1 = [10, 11, 12, 11, 10, 12, 11]
    const sample2 = [11, 10, 12, 11, 12, 10, 11]

    const result = welchTTest(sample1, sample2)

    expect(result.p_value).toBeGreaterThan(0.05) // Not significant
  })

  it('should throw error for insufficient samples', () => {
    expect(() => welchTTest([1], [2, 3])).toThrow()
    expect(() => welchTTest([1, 2], [3])).toThrow()
  })

  it('should handle samples with different variances', () => {
    const sample1 = [1, 2, 3, 4, 5] // low variance
    const sample2 = [1, 10, 1, 10, 1] // high variance

    const result = welchTTest(sample1, sample2)

    expect(result.df).toBeGreaterThan(0)
    expect(result.p_value).toBeGreaterThan(0)
    expect(result.p_value).toBeLessThanOrEqual(1)
  })
})

describe('Cohen\'s d effect size', () => {
  it('should calculate small effect size', () => {
    const sample1 = [10, 11, 12, 11, 10]
    const sample2 = [11, 12, 13, 12, 11]

    const d = cohensD(sample1, sample2)

    expect(Math.abs(d)).toBeGreaterThan(0)
    expect(Math.abs(d)).toBeLessThan(0.5) // Small effect
  })

  it('should calculate large effect size', () => {
    const sample1 = [10, 11, 12, 11, 10]
    const sample2 = [20, 21, 22, 21, 20]

    const d = cohensD(sample1, sample2)

    expect(Math.abs(d)).toBeGreaterThan(2) // Large effect
  })

  it('should return zero for identical samples', () => {
    const sample = [10, 11, 12, 11, 10]

    const d = cohensD(sample, sample)

    expect(Math.abs(d)).toBeLessThan(0.01) // Essentially zero
  })
})

describe('Confidence Interval', () => {
  it('should calculate 95% CI', () => {
    const values = [10, 12, 11, 13, 12, 11, 10, 12]

    const ci = confidenceInterval(values, 0.95)

    expect(ci.lower).toBeLessThan(ci.upper)
    expect(ci.confidence).toBe(0.95)
    expect(ci.lower).toBeGreaterThan(10)
    expect(ci.upper).toBeLessThan(14)
  })

  it('should handle single value', () => {
    const ci = confidenceInterval([42], 0.95)

    expect(ci.lower).toBe(42)
    expect(ci.upper).toBe(42)
  })

  it('should handle empty array', () => {
    const ci = confidenceInterval([], 0.95)

    expect(ci.lower).toBe(0)
    expect(ci.upper).toBe(0)
  })

  it('should provide narrower CI with larger sample', () => {
    const smallSample = [10, 12, 11, 13]
    const largeSample = Array(100).fill(0).map((_, i) => 11 + (i % 3) - 1)

    const ciSmall = confidenceInterval(smallSample, 0.95)
    const ciLarge = confidenceInterval(largeSample, 0.95)

    const widthSmall = ciSmall.upper - ciSmall.lower
    const widthLarge = ciLarge.upper - ciLarge.lower

    expect(widthLarge).toBeLessThan(widthSmall) // Larger sample = narrower CI
  })
})

describe('Bonferroni Correction', () => {
  it('should adjust alpha for multiple comparisons', () => {
    const alpha = 0.05
    const numComparisons = 10

    const adjusted = bonferroniCorrection(alpha, numComparisons)

    expect(adjusted).toBe(0.005) // 0.05 / 10
  })

  it('should handle single comparison', () => {
    const alpha = 0.05
    const adjusted = bonferroniCorrection(alpha, 1)

    expect(adjusted).toBe(0.05) // No adjustment needed
  })
})

describe('Sample Size and Power', () => {
  describe('requiredSampleSize', () => {
    it('should calculate larger sample size for smaller effects', () => {
      const nSmallEffect = requiredSampleSize(0.2, 0.05, 0.8) // Small effect
      const nLargeEffect = requiredSampleSize(0.8, 0.05, 0.8) // Large effect

      expect(nSmallEffect).toBeGreaterThan(nLargeEffect)
      expect(nSmallEffect).toBeGreaterThan(200) // Small effects need large samples
    })

    it('should calculate reasonable sample size for medium effect', () => {
      const n = requiredSampleSize(0.5, 0.05, 0.8) // Medium effect

      expect(n).toBeGreaterThan(50)
      expect(n).toBeLessThan(100)
    })
  })

  describe('estimatePower', () => {
    it('should estimate higher power with larger sample size', () => {
      const effectSize = 0.5
      const powerSmall = estimatePower(30, effectSize)
      const powerLarge = estimatePower(100, effectSize)

      expect(powerLarge).toBeGreaterThan(powerSmall)
    })

    it('should estimate higher power with larger effect size', () => {
      const sampleSize = 50
      const powerSmall = estimatePower(sampleSize, 0.2)
      const powerLarge = estimatePower(sampleSize, 0.8)

      expect(powerLarge).toBeGreaterThan(powerSmall)
    })

    it('should estimate power around 0.8 for well-designed study', () => {
      const n = requiredSampleSize(0.5, 0.05, 0.8)
      const power = estimatePower(n, 0.5)

      expect(power).toBeGreaterThan(0.75)
      expect(power).toBeLessThan(0.85)
    })
  })
})

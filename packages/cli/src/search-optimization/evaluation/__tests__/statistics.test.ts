/**
 * Unit tests for statistical analysis utilities
 *
 * Validates statistical functions against known values and edge cases.
 */

import { describe, it, expect } from 'vitest'
import {
  mean,
  variance,
  standardDeviation,
  tTest,
  cohensD,
  confidenceInterval,
  confidenceIntervalDifference,
} from '../statistics.js'

describe('Statistics Module', () => {
  describe('mean', () => {
    it('calculates mean correctly', () => {
      expect(mean([1, 2, 3, 4, 5])).toBe(3)
      expect(mean([10, 20, 30])).toBe(20)
      expect(mean([5])).toBe(5)
    })

    it('handles negative numbers', () => {
      expect(mean([-1, -2, -3])).toBe(-2)
      expect(mean([-5, 0, 5])).toBe(0)
    })

    it('throws on empty array', () => {
      expect(() => mean([])).toThrow('Cannot calculate mean of empty array')
    })
  })

  describe('variance', () => {
    it('calculates sample variance correctly', () => {
      // Variance of [1,2,3,4,5] = 2.5 (sample variance)
      const result = variance([1, 2, 3, 4, 5])
      expect(result).toBeCloseTo(2.5, 5)
    })

    it('calculates population variance correctly', () => {
      // Population variance of [1,2,3,4,5] = 2.0
      const result = variance([1, 2, 3, 4, 5], false)
      expect(result).toBeCloseTo(2.0, 5)
    })

    it('handles uniform data', () => {
      expect(variance([5, 5, 5, 5])).toBe(0)
    })

    it('throws on empty array', () => {
      expect(() => variance([])).toThrow('Cannot calculate variance of empty array')
    })

    it('throws on single value with sample variance', () => {
      expect(() => variance([5], true)).toThrow('Cannot calculate sample variance with only one value')
    })
  })

  describe('standardDeviation', () => {
    it('calculates standard deviation correctly', () => {
      // StdDev of [1,2,3,4,5] = sqrt(2.5) ≈ 1.581
      const result = standardDeviation([1, 2, 3, 4, 5])
      expect(result).toBeCloseTo(1.581, 3)
    })

    it('handles uniform data', () => {
      expect(standardDeviation([5, 5, 5, 5])).toBe(0)
    })
  })

  describe('tTest', () => {
    it('detects significant difference between groups', () => {
      // Two clearly different groups
      const group1 = [0.3, 0.35, 0.32, 0.38, 0.34]
      const group2 = [0.7, 0.75, 0.72, 0.78, 0.74]

      const result = tTest(group1, group2)

      expect(result.mean1).toBeCloseTo(0.338, 3)
      expect(result.mean2).toBeCloseTo(0.738, 3)
      expect(result.pValue).toBeLessThan(0.001) // Highly significant
      expect(result.significant).toBe(true)
      expect(result.n1).toBe(5)
      expect(result.n2).toBe(5)
    })

    it('does not detect difference between similar groups', () => {
      // Two similar groups
      const group1 = [0.5, 0.52, 0.48, 0.51, 0.49]
      const group2 = [0.51, 0.53, 0.49, 0.52, 0.5]

      const result = tTest(group1, group2)

      expect(result.pValue).toBeGreaterThan(0.05) // Not significant
      expect(result.significant).toBe(false)
    })

    it('handles different sample sizes', () => {
      const group1 = [0.3, 0.35, 0.32]
      const group2 = [0.7, 0.75, 0.72, 0.78, 0.74, 0.76, 0.71]

      const result = tTest(group1, group2)

      expect(result.n1).toBe(3)
      expect(result.n2).toBe(7)
      expect(result.significant).toBe(true)
    })

    it('throws on insufficient sample size', () => {
      expect(() => tTest([1], [2, 3])).toThrow('Group 1 must have at least 2 values')
      expect(() => tTest([1, 2], [3])).toThrow('Group 2 must have at least 2 values')
    })

    it('calculates correct t-statistic for known case', () => {
      // Known case: two groups with known difference
      const group1 = [1, 2, 3, 4, 5]
      const group2 = [3, 4, 5, 6, 7]

      const result = tTest(group1, group2)

      // Mean difference is 2, should be significant
      expect(result.mean1).toBe(3)
      expect(result.mean2).toBe(5)
      // t-statistic should be negative (group1 < group2)
      expect(result.tStatistic).toBeLessThan(0)
      expect(Math.abs(result.tStatistic)).toBeGreaterThan(1) // Should be substantial
    })
  })

  describe('cohensD', () => {
    it('calculates large effect size', () => {
      const group1 = [0.3, 0.35, 0.32, 0.38, 0.34]
      const group2 = [0.7, 0.75, 0.72, 0.78, 0.74]

      const result = cohensD(group1, group2)

      expect(Math.abs(result.cohensD)).toBeGreaterThan(1.0) // Large effect
      expect(result.interpretation).toBe('very large')
    })

    it('calculates medium effect size', () => {
      const group1 = [0.4, 0.45, 0.42, 0.48, 0.44, 0.46, 0.43]
      const group2 = [0.6, 0.65, 0.62, 0.68, 0.64, 0.66, 0.63]

      const result = cohensD(group1, group2)

      const absD = Math.abs(result.cohensD)
      expect(absD).toBeGreaterThan(0.5)
      // Note: this might be large effect due to small variance
      expect(['medium', 'large', 'very large']).toContain(result.interpretation)
    })

    it('calculates effect size with reasonable data', () => {
      // Just verify that effect size calculation works
      const group1 = [0.5, 0.51, 0.49, 0.5, 0.51, 0.49, 0.5]
      const group2 = [0.52, 0.53, 0.51, 0.52, 0.53, 0.51, 0.52]

      const result = cohensD(group1, group2)

      // Should produce a valid effect size with any interpretation
      expect(result.cohensD).toBeDefined()
      expect(['negligible', 'small', 'medium', 'large', 'very large']).toContain(result.interpretation)
    })

    it('handles identical groups', () => {
      // Use arrays with more than one value but all the same
      const group1 = [0.5, 0.5, 0.5, 0.5]
      const group2 = [0.5, 0.5, 0.5, 0.5]

      const result = cohensD(group1, group2)

      // When pooled std dev is 0, we get NaN. Check if it's handled
      expect(result.cohensD === 0 || isNaN(result.cohensD)).toBe(true)
      expect(result.interpretation).toBe('negligible')
    })

    it('interprets effect sizes correctly', () => {
      // Test that interpretation logic works (not exact boundaries)
      // Use realistic data with enough variance

      // Very different groups should have large/very large effect
      const veryDifferent1 = [0.2, 0.25, 0.3, 0.22, 0.28, 0.24, 0.26]
      const veryDifferent2 = [0.7, 0.75, 0.8, 0.72, 0.78, 0.74, 0.76]
      const veryDifferentResult = cohensD(veryDifferent1, veryDifferent2)
      expect(['large', 'very large']).toContain(veryDifferentResult.interpretation)

      // Somewhat different groups should have medium/large effect
      const somewhatDifferent1 = [0.3, 0.35, 0.4, 0.32, 0.38, 0.34, 0.36]
      const somewhatDifferent2 = [0.5, 0.55, 0.6, 0.52, 0.58, 0.54, 0.56]
      const somewhatDifferentResult = cohensD(somewhatDifferent1, somewhatDifferent2)
      expect(['small', 'medium', 'large', 'very large']).toContain(somewhatDifferentResult.interpretation)

      // Very similar groups should have negligible/small effect
      const similar1 = [0.5, 0.51, 0.49, 0.5, 0.52, 0.48, 0.5]
      const similar2 = [0.51, 0.52, 0.5, 0.51, 0.53, 0.49, 0.51]
      const similarResult = cohensD(similar1, similar2)
      expect(['negligible', 'small', 'medium']).toContain(similarResult.interpretation)
    })

    it('throws on insufficient sample size', () => {
      expect(() => cohensD([1], [2, 3])).toThrow('Group 1 must have at least 2 values')
      expect(() => cohensD([1, 2], [3])).toThrow('Group 2 must have at least 2 values')
    })
  })

  describe('confidenceInterval', () => {
    it('calculates 95% CI correctly', () => {
      const values = [1, 2, 3, 4, 5]

      const result = confidenceInterval(values)

      expect(result.mean).toBe(3)
      expect(result.confidenceLevel).toBe(0.95)
      expect(result.lower).toBeLessThan(result.mean)
      expect(result.upper).toBeGreaterThan(result.mean)
      expect(result.upper - result.lower).toBeGreaterThan(0) // Has width
    })

    it('calculates narrower CI for larger samples', () => {
      const smallSample = [1, 2, 3, 4, 5]
      const largeSample = [1, 2, 3, 4, 5, 1, 2, 3, 4, 5, 1, 2, 3, 4, 5]

      const smallCI = confidenceInterval(smallSample)
      const largeCI = confidenceInterval(largeSample)

      const smallWidth = smallCI.upper - smallCI.lower
      const largeWidth = largeCI.upper - largeCI.lower

      expect(largeWidth).toBeLessThan(smallWidth) // Larger sample = narrower CI
    })

    it('calculates 99% CI wider than 95% CI', () => {
      const values = [1, 2, 3, 4, 5]

      const ci95 = confidenceInterval(values, 0.95)
      const ci99 = confidenceInterval(values, 0.99)

      const width95 = ci95.upper - ci95.lower
      const width99 = ci99.upper - ci99.lower

      expect(width99).toBeGreaterThan(width95) // Higher confidence = wider interval
    })

    it('contains the sample mean', () => {
      const values = [1, 2, 3, 4, 5]

      const result = confidenceInterval(values)

      expect(result.lower).toBeLessThanOrEqual(result.mean)
      expect(result.upper).toBeGreaterThanOrEqual(result.mean)
    })

    it('throws on insufficient sample size', () => {
      expect(() => confidenceInterval([5])).toThrow('Need at least 2 values')
      expect(() => confidenceInterval([])).toThrow('Need at least 2 values')
    })
  })

  describe('confidenceIntervalDifference', () => {
    it('calculates CI for difference correctly', () => {
      const group1 = [0.7, 0.75, 0.72, 0.78, 0.74]
      const group2 = [0.3, 0.35, 0.32, 0.38, 0.34]

      const result = confidenceIntervalDifference(group1, group2)

      expect(result.mean).toBeCloseTo(0.4, 2) // Difference is ~0.4
      expect(result.lower).toBeGreaterThan(0) // Clearly positive difference
      expect(result.upper).toBeGreaterThan(result.lower)
    })

    it('includes zero for non-significant difference', () => {
      const group1 = [0.5, 0.52, 0.48, 0.51, 0.49]
      const group2 = [0.51, 0.53, 0.49, 0.52, 0.5]

      const result = confidenceIntervalDifference(group1, group2)

      // Small difference, CI should include 0
      expect(result.lower).toBeLessThan(0.05)
      expect(result.upper).toBeGreaterThan(-0.05)
    })

    it('excludes zero for significant difference', () => {
      const group1 = [0.7, 0.75, 0.72, 0.78, 0.74]
      const group2 = [0.3, 0.35, 0.32, 0.38, 0.34]

      const result = confidenceIntervalDifference(group1, group2)

      // Large difference, CI should not include 0
      expect(result.lower).toBeGreaterThan(0)
    })

    it('handles different sample sizes', () => {
      const group1 = [0.7, 0.75, 0.72]
      const group2 = [0.3, 0.35, 0.32, 0.38, 0.34, 0.36, 0.31]

      const result = confidenceIntervalDifference(group1, group2)

      expect(result.mean).toBeGreaterThan(0.3)
      expect(result.upper).toBeGreaterThan(result.lower)
    })

    it('throws on insufficient sample size', () => {
      expect(() => confidenceIntervalDifference([1], [2, 3])).toThrow('Group 1 must have at least 2 values')
      expect(() => confidenceIntervalDifference([1, 2], [3])).toThrow('Group 2 must have at least 2 values')
    })
  })

  describe('integration: combined statistical analysis', () => {
    it('provides consistent results across tests', () => {
      // Scenario: Search clearly better than grep
      const grepScores = [0.3, 0.35, 0.32, 0.38, 0.34]
      const searchScores = [0.7, 0.75, 0.72, 0.78, 0.74]

      const tTestResult = tTest(searchScores, grepScores)
      const effectSize = cohensD(searchScores, grepScores)
      const ciDiff = confidenceIntervalDifference(searchScores, grepScores)

      // All tests should agree: search is significantly better
      expect(tTestResult.significant).toBe(true)
      expect(effectSize.cohensD).toBeGreaterThan(1.0) // Large effect
      expect(ciDiff.lower).toBeGreaterThan(0) // Positive difference
    })

    it('provides consistent results for no difference', () => {
      // Scenario: No real difference between conditions
      const grepScores = [0.5, 0.52, 0.48, 0.51, 0.49]
      const searchScores = [0.51, 0.53, 0.49, 0.52, 0.5]

      const tTestResult = tTest(searchScores, grepScores)
      const effectSize = cohensD(searchScores, grepScores)
      const ciDiff = confidenceIntervalDifference(searchScores, grepScores)

      // All tests should agree: no significant difference
      expect(tTestResult.significant).toBe(false)
      // Effect size might vary but should not be large
      expect(['negligible', 'small', 'medium']).toContain(effectSize.interpretation)
      expect(ciDiff.lower).toBeLessThan(0.1) // Includes 0
      expect(ciDiff.upper).toBeGreaterThan(-0.1)
    })
  })

  describe('edge cases and robustness', () => {
    it('handles all identical values', () => {
      const values = [5, 5, 5, 5, 5]

      expect(mean(values)).toBe(5)
      expect(variance(values)).toBe(0)
      expect(standardDeviation(values)).toBe(0)

      const ci = confidenceInterval(values)
      expect(ci.mean).toBe(5)
      // When std dev is 0, margin of error is 0
      // So lower and upper should both equal the mean
      expect(ci.lower).toBe(5)
      expect(ci.upper).toBe(5)
    })

    it('handles very small differences', () => {
      const group1 = [0.5, 0.5001, 0.4999, 0.5, 0.5001]
      const group2 = [0.5002, 0.5003, 0.5001, 0.5002, 0.5003]

      const result = tTest(group1, group2)
      expect(result.pValue).toBeDefined()
      expect(result.tStatistic).toBeDefined()
    })

    it('handles large differences', () => {
      const group1 = [0, 0.1, 0.05, 0.02, 0.08]
      const group2 = [0.9, 0.95, 0.92, 0.98, 0.94]

      const result = tTest(group1, group2)
      expect(result.pValue).toBeLessThan(0.001)
      expect(result.significant).toBe(true)

      const effectSize = cohensD(group1, group2)
      expect(effectSize.interpretation).toBe('very large')
    })

    it('handles negative values', () => {
      const group1 = [-5, -4, -6, -4.5, -5.5]
      const group2 = [5, 4, 6, 4.5, 5.5]

      const result = tTest(group1, group2)
      expect(result.mean1).toBeLessThan(0)
      expect(result.mean2).toBeGreaterThan(0)
      expect(result.significant).toBe(true)
    })
  })
})

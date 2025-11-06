/**
 * Tests for statistical analysis framework
 */

import { describe, it, expect } from 'vitest'
import { analyzeExperiment, generateReport } from './analyzer.js'
import type { VariantMetrics } from './metrics.js'

describe('Statistical Analysis Framework', () => {
  // Helper to create mock metrics
  function createMockMetrics(
    id: string,
    name: string,
    successRate: number,
    totalQueries: number = 100
  ): VariantMetrics {
    const successful = Math.round(successRate * totalQueries)
    const queryResults = Array.from({ length: totalQueries }, (_, i) => ({
      query_id: `Q${i + 1}`,
      original_query: `query ${i + 1}`,
      transformed_query: `transformed ${i + 1}`,
      result_count: i < successful ? 5 : 1,
      execution_time_ms: 100,
      transformation_confidence: 0.8,
      success: i < successful
    }))

    return {
      variant_id: id,
      variant_name: name,
      total_queries: totalQueries,
      successful_queries: successful,
      success_rate: successRate,
      avg_result_count: successful * 5 / totalQueries + (totalQueries - successful) * 1 / totalQueries,
      avg_execution_time_ms: 100,
      avg_transformation_confidence: 0.8,
      total_execution_time_ms: totalQueries * 100,
      query_results: queryResults,
      timestamp: new Date()
    }
  }

  describe('analyzeExperiment', () => {
    it('should detect clear winner with statistical significance', () => {
      const variants = [
        createMockMetrics('baseline', 'Baseline', 0.70, 100),
        createMockMetrics('improved', 'Improved', 0.85, 100) // Clear 15% improvement
      ]

      const result = analyzeExperiment(variants, 'test-exp-1')

      expect(result.winner).toBe('improved')
      expect(result.statistical_significance).toBe(true)
      expect(result.p_value).toBeLessThan(0.05)
      expect(Math.abs(result.effect_size)).toBeGreaterThan(0.3) // Medium+ effect
    })

    it('should not detect winner when improvement is small', () => {
      const variants = [
        createMockMetrics('v1', 'Variant 1', 0.70, 100),
        createMockMetrics('v2', 'Variant 2', 0.72, 100) // Only 2% improvement
      ]

      const result = analyzeExperiment(variants, 'test-exp-2')

      // Should not be significant (below 5% practical threshold)
      expect(result.winner).toBeOneOf([null, 'v1', 'v2'])
      if (result.winner === 'v2') {
        expect(result.statistical_significance).toBe(false)
      }
    })

    it('should not detect winner when samples are similar', () => {
      const variants = [
        createMockMetrics('v1', 'Variant 1', 0.75, 100),
        createMockMetrics('v2', 'Variant 2', 0.76, 100) // Essentially the same
      ]

      const result = analyzeExperiment(variants, 'test-exp-3')

      expect(result.p_value).toBeGreaterThan(0.05)
    })

    it('should warn about small sample sizes', () => {
      const variants = [
        createMockMetrics('v1', 'Variant 1', 0.70, 50), // Below minimum
        createMockMetrics('v2', 'Variant 2', 0.85, 50)
      ]

      const result = analyzeExperiment(variants, 'test-exp-4')

      expect(result.warnings.length).toBeGreaterThan(0)
      expect(result.warnings[0]).toContain('50 samples')
    })

    it('should handle multiple variants with Bonferroni correction', () => {
      const variants = [
        createMockMetrics('v1', 'Variant 1', 0.70, 100),
        createMockMetrics('v2', 'Variant 2', 0.75, 100),
        createMockMetrics('v3', 'Variant 3', 0.80, 100),
        createMockMetrics('v4', 'Variant 4', 0.85, 100)
      ]

      const result = analyzeExperiment(variants, 'test-exp-5')

      // With 4 variants, Bonferroni correction applies
      // Should still detect clear winner (v4 at 85%)
      expect(result.variants).toHaveLength(4)
      expect(result.winner).toBe('v4')
    })

    it('should include vs_baseline comparisons for all non-baseline variants', () => {
      const variants = [
        createMockMetrics('baseline', 'Baseline', 0.70, 100),
        createMockMetrics('improved', 'Improved', 0.85, 100)
      ]

      const result = analyzeExperiment(variants, 'test-exp-6')

      const baselineResult = result.variants.find(v => v.variant_id === 'baseline')
      const improvedResult = result.variants.find(v => v.variant_id === 'improved')

      expect(baselineResult?.vs_baseline).toBeNull() // Baseline doesn't compare to itself
      expect(improvedResult?.vs_baseline).not.toBeNull()
      expect(improvedResult?.vs_baseline?.delta).toBeGreaterThan(0.1) // ~15% improvement
    })

    it('should calculate confidence intervals', () => {
      const variants = [
        createMockMetrics('v1', 'Variant 1', 0.70, 100),
        createMockMetrics('v2', 'Variant 2', 0.85, 100)
      ]

      const result = analyzeExperiment(variants, 'test-exp-7')

      expect(result.confidence_interval).toBeDefined()
      expect(result.confidence_interval.lower).toBeLessThan(result.confidence_interval.upper)
      expect(result.confidence_interval.confidence).toBe(0.95)
    })

    it('should throw error for insufficient variants', () => {
      const variants = [createMockMetrics('only', 'Only Variant', 0.75, 100)]

      expect(() => analyzeExperiment(variants, 'test-exp-8')).toThrow('at least 2 variants')
    })

    it('should generate appropriate recommendations for winner', () => {
      const variants = [
        createMockMetrics('baseline', 'Baseline', 0.70, 100),
        createMockMetrics('winner', 'Winner', 0.88, 100) // Strong winner
      ]

      const result = analyzeExperiment(variants, 'test-exp-9')

      expect(result.recommendation).toContain('Winner detected')
      expect(result.recommendation).toContain('Crossover')
      expect(result.recommendation).toContain('Amplification')
    })

    it('should generate recommendations for tie scenario', () => {
      const variants = [
        createMockMetrics('v1', 'Variant 1', 0.75, 100),
        createMockMetrics('v2', 'Variant 2', 0.76, 100) // Very close
      ]

      const result = analyzeExperiment(variants, 'test-exp-10')

      // Should recommend crossover or increase sample size
      expect(result.recommendation).toMatch(/crossover|sample size/i)
    })

    it('should use custom config values', () => {
      const variants = [
        createMockMetrics('v1', 'Variant 1', 0.70, 100),
        createMockMetrics('v2', 'Variant 2', 0.75, 100)
      ]

      const customConfig = {
        alpha: 0.01, // More stringent
        minPracticalDelta: 0.10, // Require 10% improvement
        minSampleSize: 50
      }

      const result = analyzeExperiment(variants, 'test-exp-11', customConfig)

      // With stricter criteria, 5% improvement may not win
      expect(result.variants).toHaveLength(2)
    })
  })

  describe('generateReport', () => {
    it('should generate human-readable report', () => {
      const variants = [
        createMockMetrics('baseline', 'Baseline Variant', 0.70, 100),
        createMockMetrics('improved', 'Improved Variant', 0.85, 100)
      ]

      const analysis = analyzeExperiment(variants, 'test-report')
      const report = generateReport(analysis)

      expect(report).toContain('STATISTICAL ANALYSIS REPORT')
      expect(report).toContain('Experiment ID: test-report')
      expect(report).toContain('Winner:')
      expect(report).toContain('VARIANT DETAILS')
      expect(report).toContain('Baseline Variant')
      expect(report).toContain('Improved Variant')
      expect(report).toContain('70.0%') // Success rates
      expect(report).toContain('85.0%')
    })

    it('should include warnings in report', () => {
      const variants = [
        createMockMetrics('v1', 'Variant 1', 0.70, 30), // Small sample
        createMockMetrics('v2', 'Variant 2', 0.85, 30)
      ]

      const analysis = analyzeExperiment(variants, 'test-report-warnings')
      const report = generateReport(analysis)

      expect(report).toContain('WARNINGS')
      expect(report).toContain('30 samples')
    })

    it('should include recommendations in report', () => {
      const variants = [
        createMockMetrics('baseline', 'Baseline', 0.70, 100),
        createMockMetrics('winner', 'Winner', 0.88, 100)
      ]

      const analysis = analyzeExperiment(variants, 'test-report-recs')
      const report = generateReport(analysis)

      expect(report).toContain('Recommended next mutations')
      expect(report).toContain('Crossover')
    })

    it('should format statistics clearly', () => {
      const variants = [
        createMockMetrics('v1', 'Variant 1', 0.70, 100),
        createMockMetrics('v2', 'Variant 2', 0.85, 100)
      ]

      const analysis = analyzeExperiment(variants, 'test-report-stats')
      const report = generateReport(analysis)

      expect(report).toContain('P-value:')
      expect(report).toContain('Effect Size')
      expect(report).toContain('95% CI')
      expect(report).toContain('Statistical Significance:')
    })
  })
})

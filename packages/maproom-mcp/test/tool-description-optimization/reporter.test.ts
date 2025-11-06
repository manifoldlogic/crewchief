/**
 * Tests for result formatter and reporter
 */

import { describe, it, expect } from 'vitest'
import {
  formatJSON,
  formatText,
  formatCategoryMetrics,
  formatComparison,
  formatLeaderboard,
  formatMarkdown
} from './reporter.js'
import type { VariantMetrics, CategoryMetrics, VariantComparison } from './metrics.js'

describe('Reporter', () => {
  const mockMetrics: VariantMetrics = {
    variant_id: 'test-variant',
    variant_name: 'Test Variant',
    total_queries: 100,
    successful_queries: 80,
    success_rate: 0.8,
    avg_result_count: 5.5,
    avg_execution_time_ms: 125,
    avg_transformation_confidence: 0.75,
    total_execution_time_ms: 12500,
    query_results: [
      {
        query_id: 'Q1',
        original_query: 'test query',
        transformed_query: 'test',
        result_count: 5,
        execution_time_ms: 100,
        transformation_confidence: 0.8,
        success: true
      },
      {
        query_id: 'Q2',
        original_query: 'failed query',
        transformed_query: 'failed',
        result_count: 1,
        execution_time_ms: 50,
        transformation_confidence: 0.5,
        success: false
      }
    ],
    timestamp: new Date('2025-01-01T00:00:00Z')
  }

  describe('formatJSON', () => {
    it('should format metrics as JSON', () => {
      const json = formatJSON(mockMetrics)

      expect(json).toContain('"variant_id": "test-variant"')
      expect(json).toContain('"success_rate": 0.8')
      expect(() => JSON.parse(json)).not.toThrow()
    })

    it('should format array of metrics as JSON', () => {
      const json = formatJSON([mockMetrics, mockMetrics])

      expect(() => JSON.parse(json)).not.toThrow()
      const parsed = JSON.parse(json)
      expect(parsed).toHaveLength(2)
    })
  })

  describe('formatText', () => {
    it('should format metrics as human-readable text', () => {
      const text = formatText(mockMetrics)

      expect(text).toContain('Test Variant')
      expect(text).toContain('Total Queries:        100')
      expect(text).toContain('Successful Queries:   80 (80.0%)')
      expect(text).toContain('Avg Results/Query:    5.50')
    })

    it('should include failed queries section', () => {
      const text = formatText(mockMetrics)

      expect(text).toContain('FAILED QUERIES')
      expect(text).toContain('Q2')
      expect(text).toContain('failed query')
    })

    it('should include top queries section', () => {
      const text = formatText(mockMetrics)

      expect(text).toContain('TOP 5 QUERIES')
      expect(text).toContain('Q1')
    })
  })

  describe('formatCategoryMetrics', () => {
    it('should format category metrics', () => {
      const categoryMetrics: CategoryMetrics[] = [
        {
          category: 'natural_language',
          total_queries: 40,
          successful_queries: 30,
          success_rate: 0.75,
          avg_result_count: 5.0
        },
        {
          category: 'simple',
          total_queries: 30,
          successful_queries: 28,
          success_rate: 0.933,
          avg_result_count: 6.5
        }
      ]

      const text = formatCategoryMetrics(categoryMetrics)

      expect(text).toContain('METRICS BY CATEGORY')
      expect(text).toContain('natural_language')
      expect(text).toContain('75.0%')
      expect(text).toContain('simple')
      expect(text).toContain('93.3%')
    })
  })

  describe('formatComparison', () => {
    it('should format variant comparison', () => {
      const comparison: VariantComparison = {
        variant_a: 'variant-a',
        variant_b: 'variant-b',
        success_rate_delta: 0.1,
        avg_result_count_delta: 1.5,
        execution_time_delta_ms: -10,
        confidence_delta: 0.05,
        winner: 'a'
      }

      const text = formatComparison(comparison)

      expect(text).toContain('VARIANT COMPARISON')
      expect(text).toContain('variant-a')
      expect(text).toContain('variant-b')
      expect(text).toContain('+10.0%') // success rate delta
      expect(text).toContain('WINNER: variant-a')
    })

    it('should handle tie scenario', () => {
      const comparison: VariantComparison = {
        variant_a: 'variant-a',
        variant_b: 'variant-b',
        success_rate_delta: 0.01,
        avg_result_count_delta: 0.1,
        execution_time_delta_ms: 5,
        confidence_delta: 0.0,
        winner: 'tie'
      }

      const text = formatComparison(comparison)

      expect(text).toContain('WINNER: TIE')
    })
  })

  describe('formatLeaderboard', () => {
    it('should format leaderboard with rankings', () => {
      const allMetrics: VariantMetrics[] = [
        { ...mockMetrics, variant_id: 'A', variant_name: 'Variant A', success_rate: 0.9 },
        { ...mockMetrics, variant_id: 'B', variant_name: 'Variant B', success_rate: 0.8 },
        { ...mockMetrics, variant_id: 'C', variant_name: 'Variant C', success_rate: 0.7 }
      ]

      const text = formatLeaderboard(allMetrics)

      expect(text).toContain('VARIANT LEADERBOARD')
      expect(text).toContain('Rank | Variant')
      expect(text).toContain('   1 | Variant A') // Highest success rate first
    })

    it('should sort by success rate primarily', () => {
      const allMetrics: VariantMetrics[] = [
        { ...mockMetrics, variant_id: 'A', success_rate: 0.7, avg_result_count: 10 },
        { ...mockMetrics, variant_id: 'B', success_rate: 0.9, avg_result_count: 4 }
      ]

      const text = formatLeaderboard(allMetrics)

      const lines = text.split('\n')
      const firstRank = lines.find(l => l.includes('   1 |'))

      expect(firstRank).toContain('B') // Higher success rate wins
    })
  })

  describe('formatMarkdown', () => {
    it('should format as markdown report', () => {
      const markdown = formatMarkdown(mockMetrics)

      expect(markdown).toContain('# Experiment Report: Test Variant')
      expect(markdown).toContain('**Variant ID:** test-variant')
      expect(markdown).toContain('| Metric | Value |')
      expect(markdown).toContain('| Total Queries | 100 |')
      expect(markdown).toContain('## Summary')
    })

    it('should include category metrics if provided', () => {
      const categoryMetrics: CategoryMetrics[] = [
        {
          category: 'natural_language',
          total_queries: 40,
          successful_queries: 30,
          success_rate: 0.75,
          avg_result_count: 5.0
        }
      ]

      const markdown = formatMarkdown(mockMetrics, categoryMetrics)

      expect(markdown).toContain('## Results by Category')
      expect(markdown).toContain('| Category | Success Rate | Avg Results |')
      expect(markdown).toContain('| natural_language |')
    })

    it('should include failed queries section', () => {
      const markdown = formatMarkdown(mockMetrics)

      expect(markdown).toContain('## Failed Queries')
      expect(markdown).toContain('- **Q2**: "failed query"')
    })
  })
})

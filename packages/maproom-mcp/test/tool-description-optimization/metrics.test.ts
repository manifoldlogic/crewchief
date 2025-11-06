/**
 * Tests for metrics collection and aggregation
 */

import { describe, it, expect, beforeEach } from 'vitest'
import { MetricsCollector, compareVariants } from './metrics.js'
import type { VariantMetrics } from './metrics.js'

describe('MetricsCollector', () => {
  let collector: MetricsCollector

  beforeEach(() => {
    collector = new MetricsCollector('test-variant', 'Test Variant')
  })

  it('should initialize with correct variant info', () => {
    const metrics = collector.getMetrics()

    expect(metrics.variant_id).toBe('test-variant')
    expect(metrics.variant_name).toBe('Test Variant')
    expect(metrics.total_queries).toBe(0)
  })

  it('should record query results', () => {
    collector.addResult('Q1', 'original query', 'transformed query', 5, 100, 0.8, 3)

    const metrics = collector.getMetrics()

    expect(metrics.total_queries).toBe(1)
    expect(metrics.successful_queries).toBe(1) // 5 >= 3
    expect(metrics.success_rate).toBe(1.0)
    expect(metrics.avg_result_count).toBe(5)
  })

  it('should calculate success rate correctly', () => {
    collector.addResult('Q1', 'query1', 'transformed1', 5, 100, 0.8, 3) // success
    collector.addResult('Q2', 'query2', 'transformed2', 2, 100, 0.7, 3) // failure
    collector.addResult('Q3', 'query3', 'transformed3', 4, 100, 0.9, 3) // success

    const metrics = collector.getMetrics()

    expect(metrics.total_queries).toBe(3)
    expect(metrics.successful_queries).toBe(2)
    expect(metrics.success_rate).toBeCloseTo(0.667, 2)
  })

  it('should calculate average metrics', () => {
    collector.addResult('Q1', 'query1', 'transformed1', 5, 100, 0.8, 3)
    collector.addResult('Q2', 'query2', 'transformed2', 3, 200, 0.6, 3)
    collector.addResult('Q3', 'query3', 'transformed3', 7, 150, 0.9, 3)

    const metrics = collector.getMetrics()

    expect(metrics.avg_result_count).toBeCloseTo(5.0, 1) // (5+3+7)/3
    expect(metrics.avg_execution_time_ms).toBeCloseTo(150.0, 1) // (100+200+150)/3
    expect(metrics.avg_transformation_confidence).toBeCloseTo(0.767, 2) // (0.8+0.6+0.9)/3
  })

  it('should get failed queries', () => {
    collector.addResult('Q1', 'query1', 'transformed1', 5, 100, 0.8, 3) // success
    collector.addResult('Q2', 'query2', 'transformed2', 1, 100, 0.7, 3) // failure
    collector.addResult('Q3', 'query3', 'transformed3', 0, 100, 0.5, 3) // failure

    const failed = collector.getFailedQueries()

    expect(failed).toHaveLength(2)
    expect(failed[0].query_id).toBe('Q2')
    expect(failed[1].query_id).toBe('Q3')
  })

  it('should get top queries by result count', () => {
    collector.addResult('Q1', 'query1', 'transformed1', 3, 100, 0.8, 3)
    collector.addResult('Q2', 'query2', 'transformed2', 10, 100, 0.9, 3)
    collector.addResult('Q3', 'query3', 'transformed3', 5, 100, 0.7, 3)

    const top = collector.getTopQueries(2)

    expect(top).toHaveLength(2)
    expect(top[0].query_id).toBe('Q2') // 10 results
    expect(top[1].query_id).toBe('Q3') // 5 results
  })

  it('should track total execution time', () => {
    collector.start()

    // Simulate some processing time
    const startTime = Date.now()
    while (Date.now() - startTime < 50) {
      // Wait 50ms
    }

    const metrics = collector.getMetrics()

    expect(metrics.total_execution_time_ms).toBeGreaterThanOrEqual(50)
  })

  it('should handle metrics by category', () => {
    collector.addResult('NL-001', 'query1', 'transformed1', 5, 100, 0.8, 3)
    collector.addResult('NL-002', 'query2', 'transformed2', 2, 100, 0.7, 3)
    collector.addResult('S-001', 'query3', 'transformed3', 4, 100, 0.9, 3)

    const categories = new Map([
      ['NL-001', 'natural_language'],
      ['NL-002', 'natural_language'],
      ['S-001', 'simple']
    ])

    const categoryMetrics = collector.getMetricsByCategory(categories)

    expect(categoryMetrics).toHaveLength(2)

    const nlMetrics = categoryMetrics.find(m => m.category === 'natural_language')
    expect(nlMetrics?.total_queries).toBe(2)
    expect(nlMetrics?.successful_queries).toBe(1)

    const simpleMetrics = categoryMetrics.find(m => m.category === 'simple')
    expect(simpleMetrics?.total_queries).toBe(1)
    expect(simpleMetrics?.successful_queries).toBe(1)
  })
})

describe('compareVariants', () => {
  it('should compare two variants and determine winner', () => {
    const metricsA: VariantMetrics = {
      variant_id: 'A',
      variant_name: 'Variant A',
      total_queries: 100,
      successful_queries: 80,
      success_rate: 0.8,
      avg_result_count: 5.0,
      avg_execution_time_ms: 100,
      avg_transformation_confidence: 0.7,
      total_execution_time_ms: 10000,
      query_results: [],
      timestamp: new Date()
    }

    const metricsB: VariantMetrics = {
      variant_id: 'B',
      variant_name: 'Variant B',
      total_queries: 100,
      successful_queries: 70,
      success_rate: 0.7,
      avg_result_count: 4.0,
      avg_execution_time_ms: 90,
      avg_transformation_confidence: 0.6,
      total_execution_time_ms: 9000,
      query_results: [],
      timestamp: new Date()
    }

    const comparison = compareVariants(metricsA, metricsB)

    expect(comparison.variant_a).toBe('A')
    expect(comparison.variant_b).toBe('B')
    expect(comparison.success_rate_delta).toBeCloseTo(0.1, 2)
    expect(comparison.winner).toBe('a') // A has higher success rate
  })

  it('should declare tie when success rates are close', () => {
    const metricsA: VariantMetrics = {
      variant_id: 'A',
      variant_name: 'Variant A',
      total_queries: 100,
      successful_queries: 75,
      success_rate: 0.75,
      avg_result_count: 5.0,
      avg_execution_time_ms: 100,
      avg_transformation_confidence: 0.7,
      total_execution_time_ms: 10000,
      query_results: [],
      timestamp: new Date()
    }

    const metricsB: VariantMetrics = {
      variant_id: 'B',
      variant_name: 'Variant B',
      total_queries: 100,
      successful_queries: 74,
      success_rate: 0.74,
      avg_result_count: 5.2,
      avg_execution_time_ms: 90,
      avg_transformation_confidence: 0.7,
      total_execution_time_ms: 9000,
      query_results: [],
      timestamp: new Date()
    }

    const comparison = compareVariants(metricsA, metricsB)

    expect(comparison.winner).toBe('tie') // Within 5% success rate, <0.5 result count delta
  })
})

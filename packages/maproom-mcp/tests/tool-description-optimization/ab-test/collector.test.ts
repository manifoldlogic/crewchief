/**
 * Tests for A/B Test Metrics Collector
 */

import { describe, it, expect, beforeEach } from 'vitest'
import { ABTestCollector, type ABTestMetric, SQL_QUERIES } from './collector.js'

describe('ABTestCollector', () => {
  let collector: ABTestCollector

  beforeEach(() => {
    collector = new ABTestCollector({
      successThreshold: 3,
      storage: 'memory'
    })
  })

  describe('log', () => {
    it('should log a query metric', () => {
      collector.log({
        user_id: 'user-1',
        session_id: 'session-1',
        variant: 'control',
        query_original: 'authentication flow',
        result_count: 5,
        execution_time_ms: 120
      })

      expect(collector.count()).toBe(1)
    })

    it('should automatically add timestamp', () => {
      const beforeLog = Date.now()
      collector.log({
        user_id: 'user-1',
        session_id: 'session-1',
        variant: 'control',
        query_original: 'test query',
        result_count: 3
      })
      const afterLog = Date.now()

      const metrics = collector.getMetrics()
      expect(metrics[0].timestamp).toBeGreaterThanOrEqual(beforeLog)
      expect(metrics[0].timestamp).toBeLessThanOrEqual(afterLog)
    })

    it('should mark as success when result_count >= threshold', () => {
      collector.log({
        user_id: 'user-1',
        session_id: 'session-1',
        variant: 'control',
        query_original: 'query with results',
        result_count: 5
      })

      const metrics = collector.getMetrics()
      expect(metrics[0].success).toBe(true)
    })

    it('should mark as failure when result_count < threshold', () => {
      collector.log({
        user_id: 'user-1',
        session_id: 'session-1',
        variant: 'control',
        query_original: 'query with few results',
        result_count: 2
      })

      const metrics = collector.getMetrics()
      expect(metrics[0].success).toBe(false)
    })

    it('should handle optional execution_time_ms', () => {
      collector.log({
        user_id: 'user-1',
        session_id: 'session-1',
        variant: 'control',
        query_original: 'test',
        result_count: 3
      })

      const metrics = collector.getMetrics()
      expect(metrics[0].execution_time_ms).toBeUndefined()
    })
  })

  describe('getMetrics', () => {
    it('should return all logged metrics', () => {
      collector.log({
        user_id: 'user-1',
        session_id: 'session-1',
        variant: 'control',
        query_original: 'query 1',
        result_count: 5
      })
      collector.log({
        user_id: 'user-2',
        session_id: 'session-2',
        variant: 'treatment',
        query_original: 'query 2',
        result_count: 3
      })

      const metrics = collector.getMetrics()
      expect(metrics).toHaveLength(2)
    })

    it('should return a copy of metrics (not reference)', () => {
      collector.log({
        user_id: 'user-1',
        session_id: 'session-1',
        variant: 'control',
        query_original: 'test',
        result_count: 3
      })

      const metrics1 = collector.getMetrics()
      const metrics2 = collector.getMetrics()

      expect(metrics1).not.toBe(metrics2)
      expect(metrics1).toEqual(metrics2)
    })
  })

  describe('getMetricsByVariant', () => {
    beforeEach(() => {
      collector.log({
        user_id: 'user-1',
        session_id: 'session-1',
        variant: 'control',
        query_original: 'query 1',
        result_count: 5
      })
      collector.log({
        user_id: 'user-2',
        session_id: 'session-2',
        variant: 'treatment',
        query_original: 'query 2',
        result_count: 3
      })
      collector.log({
        user_id: 'user-3',
        session_id: 'session-3',
        variant: 'control',
        query_original: 'query 3',
        result_count: 4
      })
    })

    it('should filter metrics by variant', () => {
      const controlMetrics = collector.getMetricsByVariant('control')
      expect(controlMetrics).toHaveLength(2)
      expect(controlMetrics.every(m => m.variant === 'control')).toBe(true)
    })

    it('should return empty array for unknown variant', () => {
      const unknownMetrics = collector.getMetricsByVariant('unknown')
      expect(unknownMetrics).toHaveLength(0)
    })
  })

  describe('getMetricsByUser', () => {
    beforeEach(() => {
      collector.log({
        user_id: 'user-1',
        session_id: 'session-1',
        variant: 'control',
        query_original: 'query 1',
        result_count: 5
      })
      collector.log({
        user_id: 'user-2',
        session_id: 'session-2',
        variant: 'treatment',
        query_original: 'query 2',
        result_count: 3
      })
      collector.log({
        user_id: 'user-1',
        session_id: 'session-3',
        variant: 'control',
        query_original: 'query 3',
        result_count: 4
      })
    })

    it('should filter metrics by user ID', () => {
      const user1Metrics = collector.getMetricsByUser('user-1')
      expect(user1Metrics).toHaveLength(2)
      expect(user1Metrics.every(m => m.user_id === 'user-1')).toBe(true)
    })

    it('should return empty array for unknown user', () => {
      const unknownMetrics = collector.getMetricsByUser('user-unknown')
      expect(unknownMetrics).toHaveLength(0)
    })
  })

  describe('getMetricsInRange', () => {
    it('should filter metrics by time range', () => {
      const now = Date.now()
      const oneHourAgo = now - 60 * 60 * 1000
      const twoHoursAgo = now - 2 * 60 * 60 * 1000

      collector.log({
        user_id: 'user-1',
        session_id: 'session-1',
        variant: 'control',
        query_original: 'old query',
        result_count: 5
      })

      // Manually adjust timestamp for testing
      const metrics = collector.getMetrics()
      ;(metrics[0] as any).timestamp = twoHoursAgo

      collector.log({
        user_id: 'user-2',
        session_id: 'session-2',
        variant: 'control',
        query_original: 'recent query',
        result_count: 3
      })

      const recentMetrics = collector.getMetricsInRange(oneHourAgo, now + 1000)
      expect(recentMetrics).toHaveLength(1)
      expect(recentMetrics[0].query_original).toBe('recent query')
    })

    it('should return empty array if no metrics in range', () => {
      const now = Date.now()
      const oneHourAgo = now - 60 * 60 * 1000
      const twoHoursAgo = now - 2 * 60 * 60 * 1000

      const metrics = collector.getMetricsInRange(twoHoursAgo, oneHourAgo)
      expect(metrics).toHaveLength(0)
    })
  })

  describe('getSuccessRate', () => {
    beforeEach(() => {
      // Control: 3 successes, 2 failures
      collector.log({
        user_id: 'user-1',
        session_id: 'session-1',
        variant: 'control',
        query_original: 'query 1',
        result_count: 5
      })
      collector.log({
        user_id: 'user-2',
        session_id: 'session-2',
        variant: 'control',
        query_original: 'query 2',
        result_count: 2
      })
      collector.log({
        user_id: 'user-3',
        session_id: 'session-3',
        variant: 'control',
        query_original: 'query 3',
        result_count: 4
      })
      collector.log({
        user_id: 'user-4',
        session_id: 'session-4',
        variant: 'control',
        query_original: 'query 4',
        result_count: 1
      })
      collector.log({
        user_id: 'user-5',
        session_id: 'session-5',
        variant: 'control',
        query_original: 'query 5',
        result_count: 3
      })

      // Treatment: 2 successes, 1 failure
      collector.log({
        user_id: 'user-6',
        session_id: 'session-6',
        variant: 'treatment',
        query_original: 'query 6',
        result_count: 5
      })
      collector.log({
        user_id: 'user-7',
        session_id: 'session-7',
        variant: 'treatment',
        query_original: 'query 7',
        result_count: 2
      })
      collector.log({
        user_id: 'user-8',
        session_id: 'session-8',
        variant: 'treatment',
        query_original: 'query 8',
        result_count: 4
      })
    })

    it('should calculate success rate for variant', () => {
      const controlRate = collector.getSuccessRate('control')
      expect(controlRate).toBeCloseTo(0.6, 2) // 3/5 = 0.6

      const treatmentRate = collector.getSuccessRate('treatment')
      expect(treatmentRate).toBeCloseTo(0.667, 2) // 2/3 ≈ 0.667
    })

    it('should return 0 for variant with no metrics', () => {
      const unknownRate = collector.getSuccessRate('unknown')
      expect(unknownRate).toBe(0)
    })
  })

  describe('getSummary', () => {
    beforeEach(() => {
      // Control variant
      collector.log({
        user_id: 'user-1',
        session_id: 'session-1',
        variant: 'control',
        query_original: 'query 1',
        result_count: 5,
        execution_time_ms: 100
      })
      collector.log({
        user_id: 'user-1',
        session_id: 'session-2',
        variant: 'control',
        query_original: 'query 2',
        result_count: 2,
        execution_time_ms: 150
      })
      collector.log({
        user_id: 'user-2',
        session_id: 'session-3',
        variant: 'control',
        query_original: 'query 3',
        result_count: 4,
        execution_time_ms: 120
      })

      // Treatment variant
      collector.log({
        user_id: 'user-3',
        session_id: 'session-4',
        variant: 'treatment',
        query_original: 'query 4',
        result_count: 6,
        execution_time_ms: 200
      })
      collector.log({
        user_id: 'user-3',
        session_id: 'session-5',
        variant: 'treatment',
        query_original: 'query 5',
        result_count: 3,
        execution_time_ms: 180
      })
    })

    it('should generate summary statistics for all variants', () => {
      const summary = collector.getSummary()

      expect(summary.size).toBe(2)
      expect(summary.has('control')).toBe(true)
      expect(summary.has('treatment')).toBe(true)
    })

    it('should calculate correct statistics for control variant', () => {
      const summary = collector.getSummary()
      const controlStats = summary.get('control')!

      expect(controlStats.variant).toBe('control')
      expect(controlStats.total_queries).toBe(3)
      expect(controlStats.successful_queries).toBe(2) // result_count >= 3
      expect(controlStats.success_rate).toBeCloseTo(0.667, 2)
      expect(controlStats.avg_result_count).toBeCloseTo(3.667, 2) // (5+2+4)/3
      expect(controlStats.avg_execution_time_ms).toBeCloseTo(123.333, 2) // (100+150+120)/3
      expect(controlStats.unique_users).toBe(2) // user-1, user-2
    })

    it('should calculate correct statistics for treatment variant', () => {
      const summary = collector.getSummary()
      const treatmentStats = summary.get('treatment')!

      expect(treatmentStats.variant).toBe('treatment')
      expect(treatmentStats.total_queries).toBe(2)
      expect(treatmentStats.successful_queries).toBe(2) // both >= 3
      expect(treatmentStats.success_rate).toBe(1.0)
      expect(treatmentStats.avg_result_count).toBeCloseTo(4.5, 2) // (6+3)/2
      expect(treatmentStats.avg_execution_time_ms).toBe(190) // (200+180)/2
      expect(treatmentStats.unique_users).toBe(1) // user-3
    })
  })

  describe('export', () => {
    it('should export metrics as JSON', () => {
      collector.log({
        user_id: 'user-1',
        session_id: 'session-1',
        variant: 'control',
        query_original: 'test query',
        result_count: 5
      })

      const exported = collector.export()
      const parsed = JSON.parse(exported)

      expect(Array.isArray(parsed)).toBe(true)
      expect(parsed).toHaveLength(1)
      expect(parsed[0].user_id).toBe('user-1')
      expect(parsed[0].variant).toBe('control')
    })
  })

  describe('clear', () => {
    it('should clear all metrics', () => {
      collector.log({
        user_id: 'user-1',
        session_id: 'session-1',
        variant: 'control',
        query_original: 'test',
        result_count: 3
      })

      expect(collector.count()).toBe(1)

      collector.clear()

      expect(collector.count()).toBe(0)
      expect(collector.getMetrics()).toHaveLength(0)
    })
  })

  describe('count', () => {
    it('should return number of metrics', () => {
      expect(collector.count()).toBe(0)

      collector.log({
        user_id: 'user-1',
        session_id: 'session-1',
        variant: 'control',
        query_original: 'test',
        result_count: 3
      })

      expect(collector.count()).toBe(1)

      collector.log({
        user_id: 'user-2',
        session_id: 'session-2',
        variant: 'treatment',
        query_original: 'test 2',
        result_count: 4
      })

      expect(collector.count()).toBe(2)
    })
  })
})

describe('SQL_QUERIES', () => {
  it('should have createTable query', () => {
    expect(SQL_QUERIES.createTable).toContain('CREATE TABLE IF NOT EXISTS ab_test_metrics')
    expect(SQL_QUERIES.createTable).toContain('timestamp BIGINT NOT NULL')
    expect(SQL_QUERIES.createTable).toContain('variant TEXT NOT NULL')
  })

  it('should have insertMetric query', () => {
    expect(SQL_QUERIES.insertMetric).toContain('INSERT INTO ab_test_metrics')
    expect(SQL_QUERIES.insertMetric).toContain('VALUES')
    expect(SQL_QUERIES.insertMetric).toContain('$1, $2, $3, $4, $5, $6, $7, $8')
  })

  it('should have successRateByVariant query', () => {
    expect(SQL_QUERIES.successRateByVariant).toContain('SELECT')
    expect(SQL_QUERIES.successRateByVariant).toContain('GROUP BY variant')
    expect(SQL_QUERIES.successRateByVariant).toContain('success_rate')
  })

  it('should have getMetricsForAnalysis query', () => {
    expect(SQL_QUERIES.getMetricsForAnalysis).toContain('SELECT')
    expect(SQL_QUERIES.getMetricsForAnalysis).toContain('variant')
    expect(SQL_QUERIES.getMetricsForAnalysis).toContain('success')
  })
})

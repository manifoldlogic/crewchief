/**
 * Tests for A/B Test Dashboard
 */

import { describe, it, expect, beforeEach } from 'vitest'
import {
  generateDashboard,
  formatDashboard,
  formatJSON,
  type DashboardData
} from './dashboard.js'
import { ABTestCollector } from './collector.js'

describe('generateDashboard', () => {
  let collector: ABTestCollector

  beforeEach(() => {
    collector = new ABTestCollector({
      successThreshold: 3,
      storage: 'memory'
    })
  })

  it('should generate dashboard data with empty collector', () => {
    const dashboard = generateDashboard(collector, 'test-exp')

    expect(dashboard.experiment_id).toBe('test-exp')
    expect(dashboard.variants).toHaveLength(0)
    expect(dashboard.winner.variant).toBeNull()
    expect(dashboard.recommendations).toHaveLength(2)
  })

  it('should generate dashboard data with metrics', () => {
    // Add control metrics
    for (let i = 0; i < 50; i++) {
      collector.log({
        user_id: `user-${i}`,
        session_id: `session-${i}`,
        variant: 'control',
        query_original: `query ${i}`,
        result_count: i % 2 === 0 ? 5 : 2 // 50% success rate
      })
    }

    // Add treatment metrics
    for (let i = 0; i < 50; i++) {
      collector.log({
        user_id: `user-${i + 50}`,
        session_id: `session-${i + 50}`,
        variant: 'treatment',
        query_original: `query ${i}`,
        result_count: i % 3 === 0 ? 5 : 2 // 33% success rate
      })
    }

    const dashboard = generateDashboard(collector, 'test-exp')

    expect(dashboard.variants).toHaveLength(2)
    expect(dashboard.variants.some(v => v.variant === 'control')).toBe(true)
    expect(dashboard.variants.some(v => v.variant === 'treatment')).toBe(true)
  })

  it('should calculate duration in hours', () => {
    const startTime = Date.now() - 2 * 60 * 60 * 1000 // 2 hours ago
    const dashboard = generateDashboard(collector, 'test-exp', startTime)

    expect(dashboard.duration_hours).toBeGreaterThan(1.9)
    expect(dashboard.duration_hours).toBeLessThan(2.1)
  })

  it('should detect winner when conditions are met', () => {
    // Control: 1000 queries, 70% success
    for (let i = 0; i < 1000; i++) {
      collector.log({
        user_id: `user-${i}`,
        session_id: `session-${i}`,
        variant: 'control',
        query_original: `query ${i}`,
        result_count: i < 700 ? 5 : 2
      })
    }

    // Treatment: 1000 queries, 80% success (>5% improvement)
    for (let i = 0; i < 1000; i++) {
      collector.log({
        user_id: `user-${i + 1000}`,
        session_id: `session-${i + 1000}`,
        variant: 'treatment',
        query_original: `query ${i}`,
        result_count: i < 800 ? 5 : 2
      })
    }

    const dashboard = generateDashboard(collector, 'test-exp')

    expect(dashboard.winner.variant).toBe('treatment')
    expect(dashboard.winner.confidence).toBeGreaterThan(0)
    expect(dashboard.recommendations[0]).toContain('Winner detected')
  })

  it('should not detect winner when sample size is too small', () => {
    // Control: 100 queries, 60% success
    for (let i = 0; i < 100; i++) {
      collector.log({
        user_id: `user-${i}`,
        session_id: `session-${i}`,
        variant: 'control',
        query_original: `query ${i}`,
        result_count: i < 60 ? 5 : 2
      })
    }

    // Treatment: 100 queries, 70% success
    for (let i = 0; i < 100; i++) {
      collector.log({
        user_id: `user-${i + 100}`,
        session_id: `session-${i + 100}`,
        variant: 'treatment',
        query_original: `query ${i}`,
        result_count: i < 70 ? 5 : 2
      })
    }

    const dashboard = generateDashboard(collector, 'test-exp')

    expect(dashboard.winner.variant).toBeNull()
    expect(dashboard.recommendations[0]).toContain('Continue collecting data')
  })

  it('should not detect winner when improvement is too small', () => {
    // Control: 1000 queries, 70% success
    for (let i = 0; i < 1000; i++) {
      collector.log({
        user_id: `user-${i}`,
        session_id: `session-${i}`,
        variant: 'control',
        query_original: `query ${i}`,
        result_count: i < 700 ? 5 : 2
      })
    }

    // Treatment: 1000 queries, 72% success (only 2% improvement, <5% threshold)
    for (let i = 0; i < 1000; i++) {
      collector.log({
        user_id: `user-${i + 1000}`,
        session_id: `session-${i + 1000}`,
        variant: 'treatment',
        query_original: `query ${i}`,
        result_count: i < 720 ? 5 : 2
      })
    }

    const dashboard = generateDashboard(collector, 'test-exp')

    expect(dashboard.winner.variant).toBeNull()
  })

  it('should include recommendations for more data when needed', () => {
    // Only 500 queries per variant
    for (let i = 0; i < 500; i++) {
      collector.log({
        user_id: `user-${i}`,
        session_id: `session-${i}`,
        variant: 'control',
        query_original: `query ${i}`,
        result_count: 5
      })
    }

    const dashboard = generateDashboard(collector, 'test-exp')

    expect(dashboard.recommendations.some(r => r.includes('Continue collecting data'))).toBe(true)
    expect(dashboard.recommendations.some(r => r.includes('500 more samples'))).toBe(true)
  })
})

describe('formatDashboard', () => {
  it('should format dashboard as text', () => {
    const mockData: DashboardData = {
      experiment_id: 'test-exp-123',
      start_time: Date.now() - 24 * 60 * 60 * 1000,
      current_time: Date.now(),
      duration_hours: 24,
      variants: [
        {
          variant: 'control',
          total_queries: 1000,
          successful_queries: 700,
          success_rate: 0.7,
          avg_result_count: 4.5,
          avg_execution_time_ms: 150,
          unique_users: 500
        },
        {
          variant: 'treatment',
          total_queries: 1000,
          successful_queries: 800,
          success_rate: 0.8,
          avg_result_count: 5.2,
          avg_execution_time_ms: 140,
          unique_users: 500
        }
      ],
      winner: {
        variant: 'treatment',
        confidence: 0.95,
        p_value: 0.01
      },
      recommendations: ['✅ Winner detected: treatment', '   Deploy treatment to 100% of traffic']
    }

    const formatted = formatDashboard(mockData)

    expect(formatted).toContain('A/B TEST DASHBOARD')
    expect(formatted).toContain('test-exp-123')
    expect(formatted).toContain('24.0 hours')
    expect(formatted).toContain('control')
    expect(formatted).toContain('treatment')
    expect(formatted).toContain('WINNER: treatment')
    expect(formatted).toContain('Confidence: 95.0%')
    expect(formatted).toContain('P-value: 0.0100')
    expect(formatted).toContain('RECOMMENDATIONS:')
    expect(formatted).toContain('✅ Winner detected: treatment')
  })

  it('should format dashboard with no winner', () => {
    const mockData: DashboardData = {
      experiment_id: 'test-exp-456',
      start_time: Date.now() - 12 * 60 * 60 * 1000,
      current_time: Date.now(),
      duration_hours: 12,
      variants: [
        {
          variant: 'control',
          total_queries: 500,
          successful_queries: 350,
          success_rate: 0.7,
          avg_result_count: 4.5,
          avg_execution_time_ms: 150,
          unique_users: 250
        }
      ],
      winner: {
        variant: null,
        confidence: 0,
        p_value: 1.0
      },
      recommendations: ['⏳ Continue collecting data', '   Need 500 more samples per variant']
    }

    const formatted = formatDashboard(mockData)

    expect(formatted).toContain('WINNER: None detected yet')
    expect(formatted).toContain('⏳ Continue collecting data')
  })

  it('should include table headers', () => {
    const mockData: DashboardData = {
      experiment_id: 'test',
      start_time: Date.now(),
      current_time: Date.now(),
      duration_hours: 1,
      variants: [],
      winner: { variant: null, confidence: 0, p_value: 1.0 },
      recommendations: []
    }

    const formatted = formatDashboard(mockData)

    expect(formatted).toContain('Variant')
    expect(formatted).toContain('Queries')
    expect(formatted).toContain('Success Rate')
    expect(formatted).toContain('Avg Results')
    expect(formatted).toContain('Users')
  })
})

describe('formatJSON', () => {
  it('should format dashboard as JSON', () => {
    const mockData: DashboardData = {
      experiment_id: 'test-exp-789',
      start_time: 1704067200000,
      current_time: 1704153600000,
      duration_hours: 24,
      variants: [
        {
          variant: 'control',
          total_queries: 100,
          successful_queries: 70,
          success_rate: 0.7,
          avg_result_count: 4.5,
          avg_execution_time_ms: 150,
          unique_users: 50
        }
      ],
      winner: {
        variant: null,
        confidence: 0,
        p_value: 1.0
      },
      recommendations: ['Continue collecting data']
    }

    const json = formatJSON(mockData)
    const parsed = JSON.parse(json)

    expect(parsed.experiment_id).toBe('test-exp-789')
    expect(parsed.variants).toHaveLength(1)
    expect(parsed.variants[0].variant).toBe('control')
    expect(parsed.winner.variant).toBeNull()
  })

  it('should format with proper indentation', () => {
    const mockData: DashboardData = {
      experiment_id: 'test',
      start_time: Date.now(),
      current_time: Date.now(),
      duration_hours: 1,
      variants: [],
      winner: { variant: null, confidence: 0, p_value: 1.0 },
      recommendations: []
    }

    const json = formatJSON(mockData)

    // Should have indentation (2 spaces)
    expect(json).toContain('  "experiment_id"')
    expect(json).toContain('  "winner"')
  })
})

describe('createDashboardHandler', () => {
  let collector: ABTestCollector

  beforeEach(() => {
    collector = new ABTestCollector({
      successThreshold: 3,
      storage: 'memory'
    })

    // Add some test data
    collector.log({
      user_id: 'user-1',
      session_id: 'session-1',
      variant: 'control',
      query_original: 'test query',
      result_count: 5
    })
  })

  it('should create handler function', () => {
    const { createDashboardHandler } = require('./dashboard.js')
    const handler = createDashboardHandler(collector)

    expect(typeof handler).toBe('function')
  })

  it('should return JSON by default', async () => {
    const { createDashboardHandler } = require('./dashboard.js')
    const handler = createDashboardHandler(collector)

    const mockReq = { query: {} }
    const mockRes = {
      json: (data: any) => {
        expect(data.experiment_id).toBeDefined()
        expect(data.variants).toBeDefined()
      },
      type: () => {},
      send: () => {},
      status: () => mockRes
    }

    await handler(mockReq, mockRes)
  })

  it('should return text when format=text', async () => {
    const { createDashboardHandler } = require('./dashboard.js')
    const handler = createDashboardHandler(collector)

    const mockReq = { query: { format: 'text' } }
    let responseText = ''
    const mockRes = {
      type: (contentType: string) => {
        expect(contentType).toBe('text/plain')
      },
      send: (text: string) => {
        responseText = text
        expect(text).toContain('A/B TEST DASHBOARD')
      },
      json: () => {},
      status: () => mockRes
    }

    await handler(mockReq, mockRes)
  })

  it('should handle errors', async () => {
    const { createDashboardHandler } = require('./dashboard.js')
    const brokenCollector = {
      getSummary: () => {
        throw new Error('Database error')
      }
    } as any

    const handler = createDashboardHandler(brokenCollector)

    const mockReq = { query: {} }
    let errorResponse: any = null
    const mockRes = {
      status: (code: number) => {
        expect(code).toBe(500)
        return mockRes
      },
      json: (data: any) => {
        errorResponse = data
        expect(data.error).toBe('Failed to generate dashboard')
        expect(data.message).toContain('Database error')
      },
      type: () => {},
      send: () => {}
    }

    await handler(mockReq, mockRes)
  })
})

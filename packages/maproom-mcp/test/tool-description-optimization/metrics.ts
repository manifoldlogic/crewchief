/**
 * Metrics Collection and Aggregation
 *
 * Collects performance metrics for variant testing:
 * - Success rate (% queries returning ≥3 results)
 * - Average result count
 * - Query transformation consistency
 * - Execution time
 */

import type { TransformationResult } from './simulator.js'

export interface QueryResult {
  query_id: string
  original_query: string
  transformed_query: string
  result_count: number
  execution_time_ms: number
  transformation_confidence: number
  success: boolean // true if result_count >= min_results
}

export interface VariantMetrics {
  variant_id: string
  variant_name: string
  total_queries: number
  successful_queries: number
  success_rate: number // 0-1
  avg_result_count: number
  avg_execution_time_ms: number
  avg_transformation_confidence: number
  total_execution_time_ms: number
  query_results: QueryResult[]
  timestamp: Date
}

export interface CategoryMetrics {
  category: string
  total_queries: number
  successful_queries: number
  success_rate: number
  avg_result_count: number
}

/**
 * Metrics Collector
 *
 * Accumulates query results and computes aggregate metrics
 */
export class MetricsCollector {
  private results: QueryResult[] = []
  private startTime?: number

  constructor(
    private variantId: string,
    private variantName: string
  ) {}

  /**
   * Start timing the experiment
   */
  start(): void {
    this.startTime = Date.now()
  }

  /**
   * Record a single query result
   */
  addResult(
    queryId: string,
    originalQuery: string,
    transformedQuery: string,
    resultCount: number,
    executionTimeMs: number,
    transformationConfidence: number,
    minResults: number = 3
  ): void {
    this.results.push({
      query_id: queryId,
      original_query: originalQuery,
      transformed_query: transformedQuery,
      result_count: resultCount,
      execution_time_ms: executionTimeMs,
      transformation_confidence: transformationConfidence,
      success: resultCount >= minResults
    })
  }

  /**
   * Compute aggregate metrics
   */
  getMetrics(): VariantMetrics {
    const totalQueries = this.results.length
    const successfulQueries = this.results.filter(r => r.success).length
    const totalExecutionTime = this.startTime ? Date.now() - this.startTime : 0

    const avgResultCount = totalQueries > 0
      ? this.results.reduce((sum, r) => sum + r.result_count, 0) / totalQueries
      : 0

    const avgExecutionTimeMs = totalQueries > 0
      ? this.results.reduce((sum, r) => sum + r.execution_time_ms, 0) / totalQueries
      : 0

    const avgTransformationConfidence = totalQueries > 0
      ? this.results.reduce((sum, r) => sum + r.transformation_confidence, 0) / totalQueries
      : 0

    return {
      variant_id: this.variantId,
      variant_name: this.variantName,
      total_queries: totalQueries,
      successful_queries: successfulQueries,
      success_rate: totalQueries > 0 ? successfulQueries / totalQueries : 0,
      avg_result_count: avgResultCount,
      avg_execution_time_ms: avgExecutionTimeMs,
      avg_transformation_confidence: avgTransformationConfidence,
      total_execution_time_ms: totalExecutionTime,
      query_results: this.results,
      timestamp: new Date()
    }
  }

  /**
   * Get metrics by category
   */
  getMetricsByCategory(categories: Map<string, string>): CategoryMetrics[] {
    const categoryMap = new Map<string, QueryResult[]>()

    // Group results by category
    for (const result of this.results) {
      const category = categories.get(result.query_id) || 'unknown'
      if (!categoryMap.has(category)) {
        categoryMap.set(category, [])
      }
      categoryMap.get(category)!.push(result)
    }

    // Compute metrics per category
    const metrics: CategoryMetrics[] = []
    for (const [category, results] of Array.from(categoryMap.entries())) {
      const total = results.length
      const successful = results.filter(r => r.success).length
      const avgResults = total > 0
        ? results.reduce((sum, r) => sum + r.result_count, 0) / total
        : 0

      metrics.push({
        category,
        total_queries: total,
        successful_queries: successful,
        success_rate: total > 0 ? successful / total : 0,
        avg_result_count: avgResults
      })
    }

    return metrics
  }

  /**
   * Get failed queries for analysis
   */
  getFailedQueries(): QueryResult[] {
    return this.results.filter(r => !r.success)
  }

  /**
   * Get top N queries by result count
   */
  getTopQueries(n: number = 10): QueryResult[] {
    return [...this.results]
      .sort((a, b) => b.result_count - a.result_count)
      .slice(0, n)
  }

  /**
   * Reset collector for new experiment
   */
  reset(): void {
    this.results = []
    this.startTime = undefined
  }
}

/**
 * Compare metrics between two variants
 */
export interface VariantComparison {
  variant_a: string
  variant_b: string
  success_rate_delta: number // positive means A is better
  avg_result_count_delta: number
  execution_time_delta_ms: number
  confidence_delta: number
  winner: 'a' | 'b' | 'tie'
}

export function compareVariants(
  metricsA: VariantMetrics,
  metricsB: VariantMetrics
): VariantComparison {
  const successRateDelta = metricsA.success_rate - metricsB.success_rate
  const avgResultCountDelta = metricsA.avg_result_count - metricsB.avg_result_count
  const executionTimeDelta = metricsA.avg_execution_time_ms - metricsB.avg_execution_time_ms
  const confidenceDelta = metricsA.avg_transformation_confidence - metricsB.avg_transformation_confidence

  // Winner determination: success rate is most important
  let winner: 'a' | 'b' | 'tie'
  if (Math.abs(successRateDelta) < 0.05) {
    // Within 5% - check avg result count
    winner = avgResultCountDelta > 0.5 ? 'a' : avgResultCountDelta < -0.5 ? 'b' : 'tie'
  } else {
    winner = successRateDelta > 0 ? 'a' : 'b'
  }

  return {
    variant_a: metricsA.variant_id,
    variant_b: metricsB.variant_id,
    success_rate_delta: successRateDelta,
    avg_result_count_delta: avgResultCountDelta,
    execution_time_delta_ms: executionTimeDelta,
    confidence_delta: confidenceDelta,
    winner
  }
}

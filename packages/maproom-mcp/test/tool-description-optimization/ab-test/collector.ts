/**
 * Metrics Collector for A/B Testing
 *
 * Logs query metrics with variant information for analysis
 */

export interface ABTestMetric {
  timestamp: number // Unix timestamp in milliseconds
  user_id: string
  session_id: string
  variant: string
  query_original: string
  result_count: number
  success: boolean // True if result_count >= threshold
  execution_time_ms?: number
}

export interface CollectorConfig {
  successThreshold: number // Minimum results for success (default: 3)
  storage: 'memory' | 'postgres' | 'file'
  flushInterval?: number // Auto-flush interval in ms (for batching)
}

/**
 * In-memory metrics collector
 *
 * For testing and development. Production should use PostgreSQL.
 */
export class ABTestCollector {
  private metrics: ABTestMetric[] = []
  private config: CollectorConfig

  constructor(config: Partial<CollectorConfig> = {}) {
    this.config = {
      successThreshold: 3,
      storage: 'memory',
      ...config
    }
  }

  /**
   * Log a query metric
   */
  log(metric: Omit<ABTestMetric, 'timestamp' | 'success'>): void {
    const fullMetric: ABTestMetric = {
      ...metric,
      timestamp: Date.now(),
      success: metric.result_count >= this.config.successThreshold
    }

    this.metrics.push(fullMetric)
  }

  /**
   * Get all collected metrics
   */
  getMetrics(): ABTestMetric[] {
    return [...this.metrics]
  }

  /**
   * Get metrics for a specific variant
   */
  getMetricsByVariant(variant: string): ABTestMetric[] {
    return this.metrics.filter(m => m.variant === variant)
  }

  /**
   * Get metrics for a specific user
   */
  getMetricsByUser(userId: string): ABTestMetric[] {
    return this.metrics.filter(m => m.user_id === userId)
  }

  /**
   * Get metrics within time range
   */
  getMetricsInRange(startMs: number, endMs: number): ABTestMetric[] {
    return this.metrics.filter(m => m.timestamp >= startMs && m.timestamp <= endMs)
  }

  /**
   * Calculate success rate for a variant
   */
  getSuccessRate(variant: string): number {
    const variantMetrics = this.getMetricsByVariant(variant)
    if (variantMetrics.length === 0) return 0

    const successful = variantMetrics.filter(m => m.success).length
    return successful / variantMetrics.length
  }

  /**
   * Get summary statistics for all variants
   */
  getSummary(): Map<string, VariantSummary> {
    const variants = new Set(this.metrics.map(m => m.variant))
    const summary = new Map<string, VariantSummary>()

    for (const variant of variants) {
      const metrics = this.getMetricsByVariant(variant)
      const successCount = metrics.filter(m => m.success).length
      const totalCount = metrics.length
      const avgResultCount = metrics.reduce((sum, m) => sum + m.result_count, 0) / totalCount
      const avgExecutionTime = metrics
        .filter(m => m.execution_time_ms !== undefined)
        .reduce((sum, m) => sum + (m.execution_time_ms || 0), 0) / totalCount

      summary.set(variant, {
        variant,
        total_queries: totalCount,
        successful_queries: successCount,
        success_rate: successCount / totalCount,
        avg_result_count: avgResultCount,
        avg_execution_time_ms: avgExecutionTime,
        unique_users: new Set(metrics.map(m => m.user_id)).size
      })
    }

    return summary
  }

  /**
   * Export metrics as JSON
   */
  export(): string {
    return JSON.stringify(this.metrics, null, 2)
  }

  /**
   * Clear all metrics
   */
  clear(): void {
    this.metrics = []
  }

  /**
   * Get metric count
   */
  count(): number {
    return this.metrics.length
  }
}

export interface VariantSummary {
  variant: string
  total_queries: number
  successful_queries: number
  success_rate: number
  avg_result_count: number
  avg_execution_time_ms: number
  unique_users: number
}

/**
 * SQL queries for PostgreSQL-based metrics collection
 */
export const SQL_QUERIES = {
  /**
   * Create ab_test_metrics table
   */
  createTable: `
    CREATE TABLE IF NOT EXISTS ab_test_metrics (
      id SERIAL PRIMARY KEY,
      timestamp BIGINT NOT NULL,
      user_id TEXT NOT NULL,
      session_id TEXT NOT NULL,
      variant TEXT NOT NULL,
      query_original TEXT NOT NULL,
      result_count INT NOT NULL,
      success BOOLEAN NOT NULL,
      execution_time_ms INT,
      created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
    );

    CREATE INDEX IF NOT EXISTS idx_ab_test_variant ON ab_test_metrics(variant);
    CREATE INDEX IF NOT EXISTS idx_ab_test_timestamp ON ab_test_metrics(timestamp);
    CREATE INDEX IF NOT EXISTS idx_ab_test_user ON ab_test_metrics(user_id);
  `,

  /**
   * Insert metric
   */
  insertMetric: `
    INSERT INTO ab_test_metrics
      (timestamp, user_id, session_id, variant, query_original, result_count, success, execution_time_ms)
    VALUES
      ($1, $2, $3, $4, $5, $6, $7, $8)
  `,

  /**
   * Get success rate by variant
   */
  successRateByVariant: `
    SELECT
      variant,
      COUNT(*) as total_queries,
      SUM(CASE WHEN success THEN 1 ELSE 0 END) as successful_queries,
      AVG(CASE WHEN success THEN 1.0 ELSE 0.0 END) as success_rate,
      AVG(result_count) as avg_result_count,
      AVG(execution_time_ms) as avg_execution_time_ms,
      COUNT(DISTINCT user_id) as unique_users
    FROM ab_test_metrics
    WHERE timestamp >= $1 AND timestamp <= $2
    GROUP BY variant
    ORDER BY success_rate DESC
  `,

  /**
   * Get metrics for statistical analysis
   */
  getMetricsForAnalysis: `
    SELECT
      variant,
      success,
      result_count
    FROM ab_test_metrics
    WHERE timestamp >= $1 AND timestamp <= $2
    ORDER BY variant, timestamp
  `
}

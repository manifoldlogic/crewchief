/**
 * Performance measurement utilities for E2E testing
 *
 * Provides functions for:
 * - Measuring response times
 * - Calculating percentiles (p50, p95, p99)
 * - Running benchmarks
 * - Generating performance reports
 */

export interface PerformanceMetrics {
  operation: string
  samples: number
  min: number
  max: number
  mean: number
  median: number
  p50: number
  p95: number
  p99: number
  stdDev: number
}

export interface BenchmarkResult {
  operation: string
  duration: number
  timestamp: number
}

/**
 * Measure execution time of an async operation
 */
export async function measureTime<T>(
  fn: () => Promise<T>
): Promise<{ result: T; duration: number }> {
  const start = performance.now()
  const result = await fn()
  const duration = performance.now() - start
  return { result, duration }
}

/**
 * Run a benchmark multiple times and collect results
 */
export async function benchmark<T>(
  operation: string,
  fn: () => Promise<T>,
  iterations: number = 100
): Promise<BenchmarkResult[]> {
  const results: BenchmarkResult[] = []

  for (let i = 0; i < iterations; i++) {
    const { duration } = await measureTime(fn)
    results.push({
      operation,
      duration,
      timestamp: Date.now(),
    })
  }

  return results
}

/**
 * Calculate percentile from sorted array
 */
function percentile(sortedValues: number[], p: number): number {
  if (sortedValues.length === 0) return 0
  const index = Math.ceil((p / 100) * sortedValues.length) - 1
  return sortedValues[Math.max(0, index)]
}

/**
 * Calculate standard deviation
 */
function standardDeviation(values: number[], mean: number): number {
  if (values.length === 0) return 0
  const squaredDiffs = values.map((v) => Math.pow(v - mean, 2))
  const variance = squaredDiffs.reduce((a, b) => a + b, 0) / values.length
  return Math.sqrt(variance)
}

/**
 * Calculate performance metrics from benchmark results
 */
export function calculateMetrics(
  operation: string,
  results: BenchmarkResult[]
): PerformanceMetrics {
  const durations = results.map((r) => r.duration).sort((a, b) => a - b)
  const sum = durations.reduce((a, b) => a + b, 0)
  const mean = sum / durations.length

  return {
    operation,
    samples: durations.length,
    min: Math.min(...durations),
    max: Math.max(...durations),
    mean,
    median: percentile(durations, 50),
    p50: percentile(durations, 50),
    p95: percentile(durations, 95),
    p99: percentile(durations, 99),
    stdDev: standardDeviation(durations, mean),
  }
}

/**
 * Format performance metrics for display
 */
export function formatMetrics(metrics: PerformanceMetrics): string {
  return `
Performance Metrics: ${metrics.operation}
  Samples: ${metrics.samples}
  Min: ${metrics.min.toFixed(2)}ms
  Max: ${metrics.max.toFixed(2)}ms
  Mean: ${metrics.mean.toFixed(2)}ms
  Median: ${metrics.median.toFixed(2)}ms
  P50: ${metrics.p50.toFixed(2)}ms
  P95: ${metrics.p95.toFixed(2)}ms
  P99: ${metrics.p99.toFixed(2)}ms
  StdDev: ${metrics.stdDev.toFixed(2)}ms
  `.trim()
}

/**
 * Assert performance target is met
 */
export function assertPerformance(
  metrics: PerformanceMetrics,
  target: { p95?: number; p99?: number; mean?: number }
): { passed: boolean; failures: string[] } {
  const failures: string[] = []

  if (target.p95 !== undefined && metrics.p95 > target.p95) {
    failures.push(
      `P95 (${metrics.p95.toFixed(2)}ms) exceeds target (${target.p95}ms)`
    )
  }

  if (target.p99 !== undefined && metrics.p99 > target.p99) {
    failures.push(
      `P99 (${metrics.p99.toFixed(2)}ms) exceeds target (${target.p99}ms)`
    )
  }

  if (target.mean !== undefined && metrics.mean > target.mean) {
    failures.push(
      `Mean (${metrics.mean.toFixed(2)}ms) exceeds target (${target.mean}ms)`
    )
  }

  return {
    passed: failures.length === 0,
    failures,
  }
}

/**
 * Run concurrent benchmarks
 */
export async function concurrentBenchmark<T>(
  operation: string,
  fn: () => Promise<T>,
  concurrency: number,
  totalRequests: number
): Promise<{
  results: BenchmarkResult[]
  metrics: PerformanceMetrics
  errors: number
  throughput: number
}> {
  const results: BenchmarkResult[] = []
  const errors: number[] = []
  const startTime = Date.now()

  // Run requests in batches of concurrent operations
  const batches = Math.ceil(totalRequests / concurrency)

  for (let batch = 0; batch < batches; batch++) {
    const batchSize = Math.min(concurrency, totalRequests - batch * concurrency)
    const promises: Promise<void>[] = []

    for (let i = 0; i < batchSize; i++) {
      promises.push(
        (async () => {
          try {
            const { duration } = await measureTime(fn)
            results.push({
              operation,
              duration,
              timestamp: Date.now(),
            })
          } catch (error) {
            errors.push(1)
          }
        })()
      )
    }

    await Promise.all(promises)
  }

  const totalTime = Date.now() - startTime
  const throughput = (results.length / totalTime) * 1000 // requests per second

  return {
    results,
    metrics: calculateMetrics(operation, results),
    errors: errors.length,
    throughput,
  }
}

/**
 * Generate performance report
 */
export function generateReport(
  metrics: PerformanceMetrics[],
  targets?: Record<string, { p95: number; p99?: number }>
): string {
  let report = '# Performance Test Report\n\n'
  report += `Generated: ${new Date().toISOString()}\n\n`

  report += '## Summary\n\n'
  report += '| Operation | Samples | Mean | P50 | P95 | P99 | Status |\n'
  report += '|-----------|---------|------|-----|-----|-----|--------|\n'

  for (const metric of metrics) {
    const target = targets?.[metric.operation]
    let status = '✓'

    if (target) {
      const assertion = assertPerformance(metric, target)
      status = assertion.passed ? '✓' : '✗'
    }

    report += `| ${metric.operation} | ${metric.samples} | ${metric.mean.toFixed(2)}ms | ${metric.p50.toFixed(2)}ms | ${metric.p95.toFixed(2)}ms | ${metric.p99.toFixed(2)}ms | ${status} |\n`
  }

  report += '\n## Detailed Metrics\n\n'
  for (const metric of metrics) {
    report += formatMetrics(metric) + '\n\n'

    if (targets?.[metric.operation]) {
      const assertion = assertPerformance(metric, targets[metric.operation])
      if (!assertion.passed) {
        report += 'FAILURES:\n'
        for (const failure of assertion.failures) {
          report += `  - ${failure}\n`
        }
        report += '\n'
      }
    }
  }

  return report
}

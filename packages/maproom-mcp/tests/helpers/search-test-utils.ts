/**
 * Search Quality Test Utilities
 *
 * Helper functions for validating search ranking behavior and performance.
 * Used by integration tests to verify semantic ranking improvements.
 */

import { Client } from 'pg'
import { expect } from 'vitest'
import { handleSearchTool } from '../../src/tools/search.js'
import { benchmark, calculateMetrics, type PerformanceMetrics } from './performance.js'
import fs from 'node:fs/promises'
import path from 'node:path'

/**
 * Search result from handleSearchTool
 */
export interface SearchResult {
  chunk_id: number
  score: number
  relpath: string
  symbol_name: string | null
  kind: string
  start_line: number
  end_line: number
}

/**
 * Search bundle returned by handleSearchTool
 */
export interface SearchBundle {
  hits: SearchResult[]
  total: number
  query: string
  mode?: string
  repo?: string
  worktree?: string
  debug?: any
}

/**
 * Baseline metric from CSV
 */
export interface BaselineMetric {
  query: string
  description: string
  latency_p50_ms: number
  latency_p95_ms: number
  latency_p99_ms: number
  top_3_kinds: string[]
  implementation_rank: number | null
  test_rank: number | null
  doc_rank: number | null
}

/**
 * Execute search query against test corpus
 */
export async function search(
  client: Client,
  query: string,
  options: {
    repo?: string
    worktree?: string
    limit?: number
    debug?: boolean
  } = {}
): Promise<SearchResult[]> {
  const params = {
    query,
    repo: options.repo || 'test-corpus',
    worktree: options.worktree || 'main',
    limit: options.limit || 20,
    debug: options.debug || false,
  }

  const bundle = await handleSearchTool(params, client)
  return bundle.hits
}

/**
 * Assert that implementation chunks rank before test/doc chunks
 *
 * Verifies the first result is:
 * - A function, class, method, or component (not heading/markdown)
 * - Not in a test file (no /test/ or .test. in path)
 */
export async function expectImplementationFirst(
  client: Client,
  query: string
): Promise<void> {
  const results = await search(client, query)

  expect(results.length).toBeGreaterThan(0)

  const first = results[0]

  // Check it's an implementation chunk kind
  const implementationKinds = ['func', 'class', 'method', 'component', 'hook']
  expect(implementationKinds).toContain(first.kind)

  // Check it's not in a test file
  expect(first.relpath).not.toMatch(/test/)
  expect(first.relpath).not.toMatch(/\.test\./)
  expect(first.relpath).not.toMatch(/\.spec\./)
}

/**
 * Assert search results match expected rank order by kind
 *
 * Checks that the first N results have the expected chunk kinds in order.
 * Useful for validating ranking improvements.
 *
 * @example
 * await expectRankOrder(client, 'authenticate', ['func', 'func', 'class'])
 */
export async function expectRankOrder(
  client: Client,
  query: string,
  expectedKinds: string[]
): Promise<void> {
  const results = await search(client, query, { limit: expectedKinds.length })

  expect(results.length).toBeGreaterThanOrEqual(expectedKinds.length)

  const actualKinds = results.slice(0, expectedKinds.length).map((r) => r.kind)

  expect(actualKinds).toEqual(expectedKinds)
}

/**
 * Measure search latency over multiple runs
 *
 * Returns percentile metrics (p50, p95, p99) for performance validation.
 * Includes warmup phase to ensure stable measurements.
 *
 * @param client - Database client
 * @param query - Search query
 * @param runs - Number of iterations (default 100)
 * @param warmup - Number of warmup iterations (default 10)
 * @returns Performance metrics with percentiles
 */
export async function measureLatency(
  client: Client,
  query: string,
  runs: number = 100,
  warmup: number = 10
): Promise<PerformanceMetrics> {
  // Warmup phase
  for (let i = 0; i < warmup; i++) {
    await search(client, query)
  }

  // Benchmark phase
  const results = await benchmark(
    query,
    async () => {
      await search(client, query)
    },
    runs
  )

  return calculateMetrics(query, results)
}

/**
 * Load baseline metrics from CSV file
 *
 * Parses the baseline-fts.csv file created in SEMRANK-1005.
 * Returns map of query -> baseline metrics for comparison.
 */
export async function loadBaseline(
  csvPath: string = path.join(process.cwd(), 'benchmarks', 'baseline-fts.csv')
): Promise<Map<string, BaselineMetric>> {
  const content = await fs.readFile(csvPath, 'utf-8')
  const lines = content.trim().split('\n')

  // Skip header
  const dataLines = lines.slice(1)

  const baseline = new Map<string, BaselineMetric>()

  for (const line of dataLines) {
    // Parse CSV line (handles quoted fields with commas)
    const fields = parseCSVLine(line)

    if (fields.length < 9) {
      continue // Skip malformed lines
    }

    const query = fields[0]
    const metric: BaselineMetric = {
      query,
      description: fields[1],
      latency_p50_ms: parseFloat(fields[2]),
      latency_p95_ms: parseFloat(fields[3]),
      latency_p99_ms: parseFloat(fields[4]),
      top_3_kinds: fields[5].split(','),
      implementation_rank: fields[6] ? parseInt(fields[6]) : null,
      test_rank: fields[7] ? parseInt(fields[7]) : null,
      doc_rank: fields[8] ? parseInt(fields[8]) : null,
    }

    baseline.set(query, metric)
  }

  return baseline
}

/**
 * Parse CSV line handling quoted fields
 */
function parseCSVLine(line: string): string[] {
  const fields: string[] = []
  let current = ''
  let inQuotes = false

  for (let i = 0; i < line.length; i++) {
    const char = line[i]

    if (char === '"') {
      inQuotes = !inQuotes
    } else if (char === ',' && !inQuotes) {
      fields.push(current.trim())
      current = ''
    } else {
      current += char
    }
  }

  // Add final field
  fields.push(current.trim())

  return fields
}

/**
 * Compare current latency against baseline
 *
 * Validates that performance hasn't regressed beyond acceptable threshold.
 * Default threshold is 10% slowdown from baseline.
 */
export function compareLatency(
  current: PerformanceMetrics,
  baseline: BaselineMetric,
  threshold: number = 0.1 // 10% acceptable slowdown
): {
  passed: boolean
  p50_diff_pct: number
  p95_diff_pct: number
  p99_diff_pct: number
  failures: string[]
} {
  const p50_diff_pct = ((current.p50 - baseline.latency_p50_ms) / baseline.latency_p50_ms) * 100
  const p95_diff_pct = ((current.p95 - baseline.latency_p95_ms) / baseline.latency_p95_ms) * 100
  const p99_diff_pct = ((current.p99 - baseline.latency_p99_ms) / baseline.latency_p99_ms) * 100

  const failures: string[] = []

  if (p50_diff_pct > threshold * 100) {
    failures.push(
      `P50 regression: ${current.p50.toFixed(2)}ms vs baseline ${baseline.latency_p50_ms.toFixed(2)}ms (+${p50_diff_pct.toFixed(1)}%)`
    )
  }

  if (p95_diff_pct > threshold * 100) {
    failures.push(
      `P95 regression: ${current.p95.toFixed(2)}ms vs baseline ${baseline.latency_p95_ms.toFixed(2)}ms (+${p95_diff_pct.toFixed(1)}%)`
    )
  }

  if (p99_diff_pct > threshold * 100) {
    failures.push(
      `P99 regression: ${current.p99.toFixed(2)}ms vs baseline ${baseline.latency_p99_ms.toFixed(2)}ms (+${p99_diff_pct.toFixed(1)}%)`
    )
  }

  return {
    passed: failures.length === 0,
    p50_diff_pct,
    p95_diff_pct,
    p99_diff_pct,
    failures,
  }
}

/**
 * Get rank of first implementation chunk in results
 *
 * Returns 1-based rank (1 = first result, 2 = second, etc.)
 * Returns null if no implementation found in results.
 */
export function getImplementationRank(results: SearchResult[]): number | null {
  const implementationKinds = ['func', 'class', 'method', 'component', 'hook']

  for (let i = 0; i < results.length; i++) {
    const result = results[i]

    // Check if it's an implementation kind and not in a test file
    if (
      implementationKinds.includes(result.kind) &&
      !result.relpath.match(/test/) &&
      !result.relpath.match(/\.test\./) &&
      !result.relpath.match(/\.spec\./)
    ) {
      return i + 1 // 1-based rank
    }
  }

  return null
}

/**
 * Get rank of first test chunk in results
 */
export function getTestRank(results: SearchResult[]): number | null {
  for (let i = 0; i < results.length; i++) {
    const result = results[i]

    if (
      result.relpath.match(/test/) ||
      result.relpath.match(/\.test\./) ||
      result.relpath.match(/\.spec\./)
    ) {
      return i + 1
    }
  }

  return null
}

/**
 * Get rank of first documentation chunk in results
 */
export function getDocRank(results: SearchResult[]): number | null {
  const docKinds = ['heading_1', 'heading_2', 'heading_3', 'markdown_section']

  for (let i = 0; i < results.length; i++) {
    const result = results[i]

    if (docKinds.includes(result.kind)) {
      return i + 1
    }
  }

  return null
}

/**
 * Assert implementation ranks before tests
 */
export async function expectImplementationBeforeTests(
  client: Client,
  query: string
): Promise<void> {
  const results = await search(client, query)

  const implRank = getImplementationRank(results)
  const testRank = getTestRank(results)

  // If both found, impl should rank higher (lower number)
  if (implRank !== null && testRank !== null) {
    expect(implRank).toBeLessThan(testRank)
  } else if (implRank === null && testRank !== null) {
    throw new Error(`No implementation found in results, but tests found at rank ${testRank}`)
  }
  // If neither found or only impl found, that's OK
}

/**
 * Assert implementation ranks before documentation
 */
export async function expectImplementationBeforeDocs(
  client: Client,
  query: string
): Promise<void> {
  const results = await search(client, query)

  const implRank = getImplementationRank(results)
  const docRank = getDocRank(results)

  // If both found, impl should rank higher (lower number)
  if (implRank !== null && docRank !== null) {
    expect(implRank).toBeLessThan(docRank)
  } else if (implRank === null && docRank !== null) {
    throw new Error(`No implementation found in results, but docs found at rank ${docRank}`)
  }
}

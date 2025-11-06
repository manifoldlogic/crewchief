/**
 * Automated Testing Harness
 *
 * Orchestrates variant testing by:
 * 1. Loading variants and test queries
 * 2. Simulating agent query transformations
 * 3. Executing searches via maproom MCP
 * 4. Collecting metrics
 * 5. Generating reports
 *
 * Supports parallel execution for fast iteration (<30min for 100 queries per variant)
 */

import { readFileSync } from 'node:fs'
import { join } from 'node:path'
import type { Variant } from './types.js'
import { simulateTransformation, type SimulationStrategy } from './simulator.js'
import { MetricsCollector, type VariantMetrics } from './metrics.js'
import { ConsoleReporter, formatJSON } from './reporter.js'

/**
 * Test query from test-queries.json
 */
export interface TestQuery {
  id: string
  category: string
  query: string
  expected_terms: string[]
  min_results: number
  gold_standard_files: string[]
  notes?: string
}

/**
 * Test query set metadata
 */
export interface TestQuerySet {
  metadata: {
    version: string
    created: string
    total_queries: number
    purpose: string
    codebase_commit: string
    query_distribution: Record<string, number>
  }
  test_queries: TestQuery[]
}

/**
 * Search result from maproom MCP (mocked for now)
 */
export interface SearchResult {
  chunk_id: string
  relpath: string
  start_line: number
  end_line: number
  score: number
}

/**
 * Tester configuration
 */
export interface TesterConfig {
  variantsDir: string
  testQueriesPath: string
  simulationStrategy: SimulationStrategy
  parallelism: number // number of concurrent queries
  subset?: number // test only first N queries (for faster iteration)
}

/**
 * Mock maproom search (replace with actual MCP call)
 */
async function mockMaproomSearch(query: string): Promise<SearchResult[]> {
  // TODO: Replace with actual mcp__maproom__search call
  // For now, return mock results based on query length
  await new Promise(resolve => setTimeout(resolve, 10 + Math.random() * 20))

  const resultCount = query.length > 3 ? Math.floor(Math.random() * 10) : 0

  return Array.from({ length: resultCount }, (_, i) => ({
    chunk_id: `chunk-${i}`,
    relpath: `src/file${i}.ts`,
    start_line: i * 10,
    end_line: i * 10 + 5,
    score: 0.9 - i * 0.1
  }))
}

/**
 * Test a single variant against all queries
 */
export async function testVariant(
  variant: Variant,
  testQueries: TestQuery[],
  simulationStrategy: SimulationStrategy = 'rule-based',
  parallelism: number = 10
): Promise<VariantMetrics> {
  const collector = new MetricsCollector(variant.id, variant.name)
  collector.start()

  // Group queries into batches for parallel execution
  const batches: TestQuery[][] = []
  for (let i = 0; i < testQueries.length; i += parallelism) {
    batches.push(testQueries.slice(i, i + parallelism))
  }

  // Process each batch in parallel
  for (const batch of batches) {
    await Promise.all(
      batch.map(async testQuery => {
        const startTime = Date.now()

        try {
          // 1. Simulate agent transformation
          const transformation = await simulateTransformation(
            testQuery.query,
            variant,
            simulationStrategy
          )

          // 2. Execute search
          const searchResults = await mockMaproomSearch(transformation.transformed_query)

          // 3. Record metrics
          const executionTime = Date.now() - startTime
          collector.addResult(
            testQuery.id,
            testQuery.query,
            transformation.transformed_query,
            searchResults.length,
            executionTime,
            transformation.confidence,
            testQuery.min_results
          )
        } catch (error) {
          // Record failure
          const executionTime = Date.now() - startTime
          collector.addResult(
            testQuery.id,
            testQuery.query,
            testQuery.query, // no transformation on error
            0, // no results
            executionTime,
            0, // no confidence
            testQuery.min_results
          )

          console.error(`Error testing query ${testQuery.id}:`, error)
        }
      })
    )
  }

  return collector.getMetrics()
}

/**
 * Test multiple variants in parallel
 */
export async function testVariants(
  variants: Variant[],
  testQueries: TestQuery[],
  simulationStrategy: SimulationStrategy = 'rule-based',
  parallelism: number = 10
): Promise<VariantMetrics[]> {
  console.log(`Testing ${variants.length} variants with ${testQueries.length} queries...`)

  // Test all variants in parallel
  const results = await Promise.all(
    variants.map(variant => testVariant(variant, testQueries, simulationStrategy, parallelism))
  )

  return results
}

/**
 * Load variant from JSON file
 */
export function loadVariant(filePath: string): Variant {
  const content = readFileSync(filePath, 'utf-8')
  return JSON.parse(content) as Variant
}

/**
 * Load test query set
 */
export function loadTestQueries(filePath: string): TestQuerySet {
  const content = readFileSync(filePath, 'utf-8')
  return JSON.parse(content) as TestQuerySet
}

/**
 * Main test harness entry point
 */
export async function runExperiment(config: TesterConfig): Promise<VariantMetrics[]> {
  console.log('='.repeat(60))
  console.log('VARIANT TESTING EXPERIMENT')
  console.log('='.repeat(60))
  console.log('')

  // Load test queries
  console.log(`Loading test queries from ${config.testQueriesPath}...`)
  const testQuerySet = loadTestQueries(config.testQueriesPath)
  let queries = testQuerySet.test_queries

  if (config.subset) {
    console.log(`Using subset of ${config.subset} queries`)
    queries = queries.slice(0, config.subset)
  }

  console.log(`Loaded ${queries.length} test queries`)
  console.log('')

  // Load variants
  console.log(`Loading variants from ${config.variantsDir}...`)
  const { readdirSync } = await import('node:fs')
  const variantFiles = readdirSync(config.variantsDir).filter(f => f.endsWith('.json'))

  const variants = variantFiles.map(f => loadVariant(join(config.variantsDir, f)))
  console.log(`Loaded ${variants.length} variants:`)
  variants.forEach(v => console.log(`  - ${v.name} (${v.id})`))
  console.log('')

  // Run tests
  console.log(`Strategy: ${config.simulationStrategy}`)
  console.log(`Parallelism: ${config.parallelism}`)
  console.log('')

  const results = await testVariants(
    variants,
    queries,
    config.simulationStrategy,
    config.parallelism
  )

  // Display results
  const reporter = new ConsoleReporter()

  for (const metrics of results) {
    reporter.report(metrics)
  }

  reporter.reportLeaderboard(results)

  return results
}

/**
 * CLI entry point (if run directly)
 */
if (import.meta.url === `file://${process.argv[1]}`) {
  const config: TesterConfig = {
    variantsDir: join(process.cwd(), 'packages/maproom-mcp/test/tool-description-optimization/variants'),
    testQueriesPath: join(process.cwd(), '.agents/projects/AGENTOPT_ai-agent-query-optimization/test-queries.json'),
    simulationStrategy: 'rule-based',
    parallelism: 10,
    subset: process.argv.includes('--quick') ? 20 : undefined
  }

  runExperiment(config)
    .then(results => {
      console.log('\nExperiment complete!')
      console.log(`Results saved for ${results.length} variants`)
      process.exit(0)
    })
    .catch(error => {
      console.error('Experiment failed:', error)
      process.exit(1)
    })
}

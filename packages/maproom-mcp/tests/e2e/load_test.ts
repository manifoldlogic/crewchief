/**
 * End-to-End Load Tests for MCP Server
 *
 * Tests concurrent scenarios:
 * - 10 concurrent clients (baseline)
 * - 50 concurrent clients (moderate load)
 * - 100 concurrent clients (high load)
 *
 * Measures:
 * - Response time degradation under load
 * - Error rates
 * - Database connection pool saturation
 * - Memory/resource leaks
 * - Request queuing behavior
 */

import { describe, it, expect, beforeAll, afterAll } from 'vitest'
import { Client } from 'pg'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import {
  createClient,
  setupTestSchema,
  cleanTestData,
  createTestRepo,
  createTestWorktree,
  createTestFile,
  createTestChunk,
} from '../helpers/database.js'
import {
  concurrentBenchmark,
  formatMetrics,
  measureTime,
} from '../helpers/performance.js'

const __dirname = path.dirname(fileURLToPath(import.meta.url))

// Import tool handlers
import { handleOpenTool } from '../../src/tools/open.js'

let testClient: Client
let testRepoId: number
let testWorktreeId: number
let testFileId: number
let testChunkIds: number[] = []
const fixturesPath = path.join(__dirname, '..', 'fixtures')

describe('E2E Load Tests', () => {
  beforeAll(async () => {
    if (!process.env.DATABASE_URL && !process.env.TEST_DATABASE_URL) {
      console.warn('No TEST_DATABASE_URL set, skipping load tests')
      return
    }

    testClient = await createClient()
    await setupTestSchema(testClient)
    await cleanTestData(testClient)

    // Create test data
    const repo = await createTestRepo(testClient, 'test-e2e-load')
    testRepoId = repo.id

    const worktree = await createTestWorktree(
      testClient,
      testRepoId,
      'main',
      fixturesPath
    )
    testWorktreeId = worktree.id

    const file = await createTestFile(
      testClient,
      testWorktreeId,
      'sample-typescript.ts'
    )
    testFileId = file.id

    // Create chunks for load testing
    for (let i = 0; i < 100; i++) {
      const chunk = await createTestChunk(testClient, testFileId, {
        symbolName: `loadTestFunction${i}`,
        kind: 'function_declaration',
        startLine: i * 10,
        endLine: i * 10 + 5,
        content: `function loadTestFunction${i}() { return ${i} }`,
        metadata: { language: 'typescript' },
      })
      testChunkIds.push(chunk.id)
    }
  })

  afterAll(async () => {
    if (testClient) {
      await cleanTestData(testClient)
      await testClient.end()
    }
  })

  describe('Baseline - 10 Concurrent Clients', () => {
    it('should handle 10 concurrent search requests', async () => {
      if (!testClient) return

      const { results, metrics, errors, throughput } = await concurrentBenchmark(
        'search_10_concurrent',
        async () => {
          const client = await createClient()
          try {
            const { rows } = await client.query(
              `SELECT c.id, f.relpath, c.symbol_name
              FROM maproom.chunks c
              JOIN maproom.files f ON f.id = c.file_id
              WHERE f.repo_id = $1 AND c.ts_doc @@ to_tsquery('simple', $2)
              LIMIT 10`,
              [testRepoId, 'loadTestFunction:*']
            )
            return rows
          } finally {
            await client.end()
          }
        },
        10, // concurrency
        100 // total requests
      )

      console.log('\n10 Concurrent Clients - Search:')
      console.log(formatMetrics(metrics))
      console.log(`Errors: ${errors}`)
      console.log(`Throughput: ${throughput.toFixed(2)} req/sec`)

      expect(errors).toBe(0)
      expect(metrics.p95).toBeLessThan(500)
      expect(throughput).toBeGreaterThan(1)
    })

    it('should handle 10 concurrent open requests', async () => {
      if (!testClient) return

      const { results, metrics, errors, throughput } = await concurrentBenchmark(
        'open_10_concurrent',
        async () => {
          const client = await createClient()
          try {
            return await handleOpenTool(
              {
                relpath: 'sample-typescript.ts',
                worktree: 'main',
              },
              client
            )
          } finally {
            await client.end()
          }
        },
        10,
        100
      )

      console.log('\n10 Concurrent Clients - Open:')
      console.log(formatMetrics(metrics))
      console.log(`Errors: ${errors}`)
      console.log(`Throughput: ${throughput.toFixed(2)} req/sec`)

      expect(errors).toBe(0)
      expect(metrics.p95).toBeLessThan(500)
      expect(throughput).toBeGreaterThan(1)
    })

    it('should handle 10 concurrent context retrievals', async () => {
      if (!testClient) return

      const { results, metrics, errors, throughput } = await concurrentBenchmark(
        'context_10_concurrent',
        async () => {
          const client = await createClient()
          try {
            const chunkId = testChunkIds[Math.floor(Math.random() * testChunkIds.length)]
            const { rows } = await client.query(
              `SELECT c.id, c.symbol_name, c.kind::text, c.start_line, c.end_line,
                f.relpath, w.name as worktree_name
              FROM maproom.chunks c
              JOIN maproom.files f ON f.id = c.file_id
              JOIN maproom.worktrees w ON w.id = f.worktree_id
              WHERE c.id = $1`,
              [chunkId]
            )
            return rows
          } finally {
            await client.end()
          }
        },
        10,
        100
      )

      console.log('\n10 Concurrent Clients - Context:')
      console.log(formatMetrics(metrics))
      console.log(`Errors: ${errors}`)
      console.log(`Throughput: ${throughput.toFixed(2)} req/sec`)

      expect(errors).toBe(0)
      expect(metrics.p95).toBeLessThan(500)
    })
  })

  describe('Moderate Load - 50 Concurrent Clients', () => {
    it('should handle 50 concurrent search requests', async () => {
      if (!testClient) return

      const { results, metrics, errors, throughput } = await concurrentBenchmark(
        'search_50_concurrent',
        async () => {
          const client = await createClient()
          try {
            const { rows } = await client.query(
              `SELECT c.id, f.relpath, c.symbol_name
              FROM maproom.chunks c
              JOIN maproom.files f ON f.id = c.file_id
              WHERE f.repo_id = $1 AND c.ts_doc @@ to_tsquery('simple', $2)
              LIMIT 10`,
              [testRepoId, 'loadTestFunction:*']
            )
            return rows
          } finally {
            await client.end()
          }
        },
        50,
        500
      )

      console.log('\n50 Concurrent Clients - Search:')
      console.log(formatMetrics(metrics))
      console.log(`Errors: ${errors}`)
      console.log(`Throughput: ${throughput.toFixed(2)} req/sec`)

      // Under load, allow higher latency but should still succeed
      expect(metrics.p95).toBeLessThan(2000)
      // Error rate should be low (< 5%)
      expect(errors).toBeLessThan(500 * 0.05)
    })

    it('should handle 50 concurrent open requests', async () => {
      if (!testClient) return

      const { results, metrics, errors, throughput } = await concurrentBenchmark(
        'open_50_concurrent',
        async () => {
          const client = await createClient()
          try {
            return await handleOpenTool(
              {
                relpath: 'sample-typescript.ts',
                worktree: 'main',
              },
              client
            )
          } finally {
            await client.end()
          }
        },
        50,
        500
      )

      console.log('\n50 Concurrent Clients - Open:')
      console.log(formatMetrics(metrics))
      console.log(`Errors: ${errors}`)
      console.log(`Throughput: ${throughput.toFixed(2)} req/sec`)

      expect(metrics.p95).toBeLessThan(2000)
      expect(errors).toBeLessThan(500 * 0.05)
    })

    it('should handle mixed workload (search + open)', async () => {
      if (!testClient) return

      let searchCount = 0
      let openCount = 0

      const { results, metrics, errors, throughput } = await concurrentBenchmark(
        'mixed_50_concurrent',
        async () => {
          const client = await createClient()
          try {
            // Alternate between search and open
            if (Math.random() > 0.5) {
              searchCount++
              const { rows } = await client.query(
                `SELECT c.id FROM maproom.chunks c
                JOIN maproom.files f ON f.id = c.file_id
                WHERE f.repo_id = $1 AND c.ts_doc @@ to_tsquery('simple', $2)
                LIMIT 5`,
                [testRepoId, 'loadTestFunction:*']
              )
              return rows
            } else {
              openCount++
              return await handleOpenTool(
                {
                  relpath: 'sample-typescript.ts',
                  worktree: 'main',
                  range: { start: 1, end: 10 },
                },
                client
              )
            }
          } finally {
            await client.end()
          }
        },
        50,
        500
      )

      console.log('\n50 Concurrent Clients - Mixed Workload:')
      console.log(formatMetrics(metrics))
      console.log(`Errors: ${errors}`)
      console.log(`Throughput: ${throughput.toFixed(2)} req/sec`)
      console.log(`Search requests: ${searchCount}`)
      console.log(`Open requests: ${openCount}`)

      expect(metrics.p95).toBeLessThan(2000)
      expect(errors).toBeLessThan(500 * 0.05)
    })
  })

  describe('High Load - 100 Concurrent Clients', () => {
    it.skip('should handle 100 concurrent search requests', async () => {
      // Skip by default as this can be resource intensive
      if (!testClient) return

      const { results, metrics, errors, throughput } = await concurrentBenchmark(
        'search_100_concurrent',
        async () => {
          const client = await createClient()
          try {
            const { rows } = await client.query(
              `SELECT c.id, f.relpath, c.symbol_name
              FROM maproom.chunks c
              JOIN maproom.files f ON f.id = c.file_id
              WHERE f.repo_id = $1 AND c.ts_doc @@ to_tsquery('simple', $2)
              LIMIT 10`,
              [testRepoId, 'loadTestFunction:*']
            )
            return rows
          } finally {
            await client.end()
          }
        },
        100,
        1000
      )

      console.log('\n100 Concurrent Clients - Search:')
      console.log(formatMetrics(metrics))
      console.log(`Errors: ${errors}`)
      console.log(`Throughput: ${throughput.toFixed(2)} req/sec`)

      // At high load, accept degraded performance
      expect(metrics.p95).toBeLessThan(5000)
      // Error rate should be acceptable (< 10%)
      expect(errors).toBeLessThan(1000 * 0.1)
    })

    it.skip('should handle 100 concurrent mixed requests', async () => {
      if (!testClient) return

      const { results, metrics, errors, throughput } = await concurrentBenchmark(
        'mixed_100_concurrent',
        async () => {
          const client = await createClient()
          try {
            if (Math.random() > 0.5) {
              const { rows } = await client.query(
                `SELECT c.id FROM maproom.chunks c
                JOIN maproom.files f ON f.id = c.file_id
                WHERE f.repo_id = $1 LIMIT 5`,
                [testRepoId]
              )
              return rows
            } else {
              return await handleOpenTool(
                {
                  relpath: 'sample-typescript.ts',
                  worktree: 'main',
                },
                client
              )
            }
          } finally {
            await client.end()
          }
        },
        100,
        1000
      )

      console.log('\n100 Concurrent Clients - Mixed:')
      console.log(formatMetrics(metrics))
      console.log(`Errors: ${errors}`)
      console.log(`Throughput: ${throughput.toFixed(2)} req/sec`)

      expect(metrics.p95).toBeLessThan(5000)
      expect(errors).toBeLessThan(1000 * 0.1)
    })
  })

  describe('Connection Pool Behavior', () => {
    it('should handle connection pool saturation gracefully', async () => {
      if (!testClient) return

      // Check active connections before load
      const { rows: before } = await testClient.query(
        "SELECT COUNT(*) as count FROM pg_stat_activity WHERE datname = current_database()"
      )

      console.log(`\nActive connections before: ${before[0].count}`)

      // Create load that exceeds typical pool size
      const { errors } = await concurrentBenchmark(
        'pool_saturation',
        async () => {
          const client = await createClient()
          try {
            await client.query('SELECT pg_sleep(0.1)')
            return true
          } finally {
            await client.end()
          }
        },
        20,
        40
      )

      // Check active connections after
      const { rows: after } = await testClient.query(
        "SELECT COUNT(*) as count FROM pg_stat_activity WHERE datname = current_database()"
      )

      console.log(`Active connections after: ${after[0].count}`)
      console.log(`Errors during pool saturation: ${errors}`)

      // Should not leak connections
      expect(parseInt(after[0].count)).toBeLessThanOrEqual(
        parseInt(before[0].count) + 5
      )
    })

    it('should measure connection acquisition time under load', async () => {
      if (!testClient) return

      const acquisitionTimes: number[] = []

      for (let i = 0; i < 20; i++) {
        const { duration } = await measureTime(async () => {
          const client = await createClient()
          await client.query('SELECT 1')
          await client.end()
        })
        acquisitionTimes.push(duration)
      }

      const avgTime =
        acquisitionTimes.reduce((a, b) => a + b, 0) / acquisitionTimes.length
      const maxTime = Math.max(...acquisitionTimes)

      console.log('\nConnection Acquisition Times:')
      console.log(`  Average: ${avgTime.toFixed(2)}ms`)
      console.log(`  Maximum: ${maxTime.toFixed(2)}ms`)

      expect(avgTime).toBeLessThan(500)
      expect(maxTime).toBeLessThan(2000)
    })

    it('should detect connection leaks', async () => {
      if (!testClient) return

      const { rows: initialConns } = await testClient.query(
        "SELECT COUNT(*) as count FROM pg_stat_activity WHERE datname = current_database()"
      )

      const initialCount = parseInt(initialConns[0].count)

      // Create and properly close many connections
      for (let i = 0; i < 20; i++) {
        const client = await createClient()
        await client.query('SELECT 1')
        await client.end()
      }

      // Wait for cleanup
      await new Promise((resolve) => setTimeout(resolve, 1000))

      const { rows: finalConns } = await testClient.query(
        "SELECT COUNT(*) as count FROM pg_stat_activity WHERE datname = current_database()"
      )

      const finalCount = parseInt(finalConns[0].count)

      console.log(`\nConnection leak test:`)
      console.log(`  Initial connections: ${initialCount}`)
      console.log(`  Final connections: ${finalCount}`)
      console.log(`  Difference: ${finalCount - initialCount}`)

      // Should not have leaked significant connections
      expect(finalCount - initialCount).toBeLessThanOrEqual(2)
    })
  })

  describe('Resource Usage Under Load', () => {
    it('should monitor memory usage during load test', async () => {
      if (!testClient) return

      const startMemory = process.memoryUsage()

      // Run load test
      await concurrentBenchmark(
        'memory_test',
        async () => {
          const client = await createClient()
          try {
            const { rows } = await client.query(
              'SELECT id FROM maproom.chunks WHERE file_id = $1 LIMIT 10',
              [testFileId]
            )
            return rows
          } finally {
            await client.end()
          }
        },
        20,
        200
      )

      const endMemory = process.memoryUsage()
      const memoryIncrease = endMemory.heapUsed - startMemory.heapUsed

      console.log('\nMemory Usage During Load:')
      console.log(
        `  Start: ${(startMemory.heapUsed / 1024 / 1024).toFixed(2)} MB`
      )
      console.log(`  End: ${(endMemory.heapUsed / 1024 / 1024).toFixed(2)} MB`)
      console.log(
        `  Increase: ${(memoryIncrease / 1024 / 1024).toFixed(2)} MB`
      )

      // Memory increase should be reasonable
      expect(memoryIncrease).toBeLessThan(100 * 1024 * 1024) // < 100MB
    })

    it('should measure query queuing behavior', async () => {
      if (!testClient) return

      const queueTimes: number[] = []

      // Create artificial queue by running slow queries
      const promises = []
      for (let i = 0; i < 30; i++) {
        promises.push(
          (async () => {
            const queueStart = Date.now()
            const client = await createClient()
            const queueTime = Date.now() - queueStart

            try {
              await client.query('SELECT pg_sleep(0.05)')
              queueTimes.push(queueTime)
            } finally {
              await client.end()
            }
          })()
        )
      }

      await Promise.all(promises)

      const avgQueueTime =
        queueTimes.reduce((a, b) => a + b, 0) / queueTimes.length
      const maxQueueTime = Math.max(...queueTimes)

      console.log('\nQuery Queuing Behavior:')
      console.log(`  Average queue time: ${avgQueueTime.toFixed(2)}ms`)
      console.log(`  Maximum queue time: ${maxQueueTime.toFixed(2)}ms`)
      console.log(`  Total queries: ${queueTimes.length}`)

      expect(maxQueueTime).toBeLessThan(5000)
    })
  })

  describe('Error Rate Under Load', () => {
    it('should maintain low error rate under sustained load', async () => {
      if (!testClient) return

      const { results, errors } = await concurrentBenchmark(
        'error_rate_test',
        async () => {
          const client = await createClient()
          try {
            const { rows } = await client.query(
              `SELECT c.id FROM maproom.chunks c
              WHERE c.file_id = $1
              LIMIT 5`,
              [testFileId]
            )
            return rows
          } finally {
            await client.end()
          }
        },
        25,
        500
      )

      const successRate = ((results.length / 500) * 100).toFixed(2)
      const errorRate = ((errors / 500) * 100).toFixed(2)

      console.log('\nError Rate Analysis:')
      console.log(`  Total requests: 500`)
      console.log(`  Successful: ${results.length} (${successRate}%)`)
      console.log(`  Failed: ${errors} (${errorRate}%)`)

      // Error rate should be < 1%
      expect(errors).toBeLessThan(5)
      expect(parseFloat(successRate)).toBeGreaterThan(99)
    })

    it('should handle rapid connection cycling', async () => {
      if (!testClient) return

      let connectionErrors = 0
      let queryErrors = 0

      for (let i = 0; i < 50; i++) {
        try {
          const client = await createClient()
          try {
            await client.query('SELECT 1')
          } catch (err) {
            queryErrors++
          } finally {
            await client.end()
          }
        } catch (err) {
          connectionErrors++
        }
      }

      console.log('\nRapid Connection Cycling:')
      console.log(`  Connection errors: ${connectionErrors}`)
      console.log(`  Query errors: ${queryErrors}`)

      expect(connectionErrors).toBe(0)
      expect(queryErrors).toBe(0)
    })
  })
})

#!/usr/bin/env node
/**
 * Performance Baseline Measurement Script
 *
 * Measures query execution time for search operations to establish
 * a baseline before implementing file_type filtering.
 */

import pg from 'pg'
const { Client } = pg

const DATABASE_URL = process.env.MAPROOM_DATABASE_URL || 'postgresql://maproom:maproom@maproom-postgres:5432/maproom'

/**
 * Execute a full-text search query and measure timing
 */
async function measureFtsSearch(client, query, repoId, k) {
  const tsParts = query
    .split(/\s+/)
    .filter(Boolean)
    .map((t) => `${t.replace(/'/g, '')}:*`)
    .join(' & ')

  const sql = `
    SELECT c.id, f.relpath, c.symbol_name, c.kind::text, c.start_line, c.end_line,
      ts_rank_cd(c.ts_doc, to_tsquery('simple', $2)) AS fts_score
    FROM maproom.chunks c
    JOIN maproom.files f ON f.id = c.file_id
    WHERE f.repo_id = $1 AND c.ts_doc @@ to_tsquery('simple', $2)
    ORDER BY fts_score DESC
    LIMIT $3
  `

  const startTime = performance.now()
  const { rows } = await client.query(sql, [repoId, tsParts, k])
  const endTime = performance.now()

  return {
    duration_ms: endTime - startTime,
    hit_count: rows.length
  }
}

/**
 * Run baseline measurements
 */
async function measureBaseline(params, runs = 10) {
  const client = new Client({ connectionString: DATABASE_URL })
  await client.connect()

  try {
    // Get repo ID
    const { rows: repoRows } = await client.query('SELECT id FROM maproom.repos WHERE name = $1', [params.repo])
    if (repoRows.length === 0) {
      throw new Error(`Repository '${params.repo}' not found`)
    }
    const repoId = repoRows[0].id

    // Get file count for documentation
    const { rows: statsRows } = await client.query(
      `SELECT COUNT(DISTINCT f.id) as file_count, COUNT(DISTINCT c.id) as chunk_count
       FROM maproom.files f
       JOIN maproom.worktrees w ON w.id = f.worktree_id
       LEFT JOIN maproom.chunks c ON c.file_id = f.id
       WHERE w.repo_id = $1`,
      [repoId]
    )

    const fileCount = parseInt(statsRows[0].file_count)
    const chunkCount = parseInt(statsRows[0].chunk_count)

    console.log(`\nRepository: ${params.repo}`)
    console.log(`Files: ${fileCount}`)
    console.log(`Chunks: ${chunkCount}`)
    console.log(`Query: "${params.query}"`)
    console.log(`Mode: ${params.mode}`)
    console.log(`k: ${params.k}`)
    console.log(`Runs: ${runs}\n`)

    const results = []

    for (let i = 0; i < runs; i++) {
      const { duration_ms, hit_count } = await measureFtsSearch(client, params.query, repoId, params.k)
      results.push({ run: i + 1, duration_ms, hit_count })
      console.log(`Run ${i + 1}: ${duration_ms.toFixed(2)}ms (${hit_count} hits)`)
    }

    return { results, fileCount, chunkCount }
  } finally {
    await client.end()
  }
}

/**
 * Calculate statistics from measurements
 */
function calculateStats(results) {
  const durations = results.map(r => r.duration_ms)
  const average = durations.reduce((sum, d) => sum + d, 0) / durations.length
  const threshold = average * 1.2

  return { average, threshold, durations }
}

/**
 * Generate markdown report
 */
function generateReport(repo, fileCount, query, mode, results, stats) {
  const date = new Date().toISOString().split('T')[0]

  let report = `# Performance Baseline - File Type Filter\n\n`
  report += `**Date:** ${date}\n`
  report += `**Repository:** ${repo}\n`
  report += `**File count:** ${fileCount} files\n`
  report += `**Query:** "${query}"\n`
  report += `**Mode:** ${mode}\n\n`

  report += `## Baseline Measurements (No Filter)\n\n`

  results.forEach(({ run, duration_ms, hit_count }) => {
    report += `Run ${run}: ${duration_ms.toFixed(2)}ms (${hit_count} hits)\n`
  })

  report += `\n**Average:** ${stats.average.toFixed(2)}ms\n`
  report += `**Threshold (baseline × 1.2):** ${stats.threshold.toFixed(2)}ms\n\n`

  report += `## Validation Criteria\n\n`
  report += `After implementing file_type filter:\n`
  report += `- Single extension (file_type: "ts"): Must be ≤ ${stats.threshold.toFixed(2)}ms\n`
  report += `- Multi extension (file_type: "ts,tsx,js"): Must be ≤ ${stats.threshold.toFixed(2)}ms\n`

  return report
}

/**
 * Main execution
 */
async function main() {
  const repo = process.argv[2] || 'crewchief'
  const query = process.argv[3] || 'authentication'
  const runs = parseInt(process.argv[4] || '10', 10)

  console.log('Starting performance baseline measurement...\n')

  const { results, fileCount, chunkCount } = await measureBaseline(
    { repo, query, mode: 'hybrid', k: 10 },
    runs
  )

  const stats = calculateStats(results)

  console.log(`\n=== Statistics ===`)
  console.log(`Average: ${stats.average.toFixed(2)}ms`)
  console.log(`Threshold (1.2x): ${stats.threshold.toFixed(2)}ms`)

  const report = generateReport(repo, fileCount, query, 'hybrid', results, stats)

  console.log('\n=== Generated Report ===\n')
  console.log(report)

  return { report, fileCount, results, stats }
}

// Run if called directly
main()
  .then(({ report }) => {
    // Write report to file
    import('fs/promises').then(async (fs) => {
      await fs.writeFile('packages/maproom-mcp/tests/performance-baseline.md', report, 'utf-8')
      console.log('\n✅ Report saved to packages/maproom-mcp/tests/performance-baseline.md')
      process.exit(0)
    })
  })
  .catch((error) => {
    console.error('Error:', error.message)
    process.exit(1)
  })

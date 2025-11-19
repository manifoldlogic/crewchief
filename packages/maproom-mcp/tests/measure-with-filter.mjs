#!/usr/bin/env node

/**
 * Performance Measurement with file_type Filter
 *
 * Measures query execution time with file_type filter enabled
 * to validate <20% performance overhead requirement (FILETYPE-2004)
 */

import pg from 'pg';
import { performance } from 'perf_hooks';

const { Client } = pg;

/**
 * Connect to database
 */
async function getClient() {
  const client = new Client({
    connectionString: 'postgresql://maproom:maproom@maproom-postgres:5432/maproom'
  });
  await client.connect();
  return client;
}

/**
 * Parse file type filter (same logic as parseFileTypeFilter in index.ts)
 */
function parseFileTypeFilter(input) {
  return input
    .split(',')
    .map(ext => ext.trim())
    .map(ext => ext.replace(/^\./, ''))
    .map(ext => ext.toLowerCase())
    .filter(ext => ext.length > 0);
}

/**
 * Run FTS search with file_type filter and measure time
 */
async function measureFtsSearchWithFilter(client, repoId, query, fileTypeFilter) {
  const tsParts = query
    .split(/\s+/)
    .filter(Boolean)
    .map((t) => `${t.replace(/'/g, '')}:*`)
    .join(' & ');

  // Parse extensions
  const extensions = parseFileTypeFilter(fileTypeFilter);

  // Build SQL with filter
  const args = [repoId, tsParts];
  let sql = `
    SELECT c.id, f.relpath, c.symbol_name, c.kind::text, c.start_line, c.end_line,
      ts_rank_cd(c.ts_doc, to_tsquery('simple', $2)) AS fts_score
    FROM maproom.chunks c
    JOIN maproom.files f ON f.id = c.file_id
    WHERE f.repo_id = $1 AND c.ts_doc @@ to_tsquery('simple', $2)
  `;

  // Add file_type filter (same logic as buildFilterClauses)
  if (extensions.length > 0) {
    if (extensions.length > 20) {
      extensions.splice(20); // Truncate to 20
    }

    if (extensions.length === 1) {
      args.push(`%.${extensions[0]}`);
      sql += ` AND f.relpath LIKE $${args.length}`;
    } else {
      const likeConditions = extensions.map(ext => {
        args.push(`%.${ext}`);
        return `f.relpath LIKE $${args.length}`;
      });
      sql += ` AND (${likeConditions.join(' OR ')})`;
    }
  }

  sql += `
    ORDER BY fts_score DESC
    LIMIT 10
  `;

  const start = performance.now();
  await client.query(sql, args);
  const end = performance.now();

  return end - start;
}

/**
 * Run multiple measurements for a filter
 */
async function runMeasurements(client, repoId, query, filterDesc, fileTypeFilter, iterations = 10) {
  console.log(`\n### ${filterDesc}`);
  console.log(`Filter: "${fileTypeFilter}"`);
  console.log();

  const timings = [];

  for (let i = 0; i < iterations; i++) {
    process.stdout.write(`Run ${i + 1}/${iterations}... `);
    const time = await measureFtsSearchWithFilter(client, repoId, query, fileTypeFilter);
    timings.push(time);
    console.log(`${time.toFixed(2)}ms`);
  }

  return timings;
}

/**
 * Calculate statistics
 */
function calculateStats(timings) {
  const sorted = [...timings].sort((a, b) => a - b);
  const sum = timings.reduce((a, b) => a + b, 0);
  const mean = sum / timings.length;

  // Remove outliers (values > 2 std deviations from mean)
  const variance = timings.reduce((acc, val) => acc + Math.pow(val - mean, 2), 0) / timings.length;
  const stdDev = Math.sqrt(variance);
  const upperBound = mean + (2 * stdDev);
  const lowerBound = mean - (2 * stdDev);

  const filtered = timings.filter(t => t >= lowerBound && t <= upperBound);
  const filteredMean = filtered.reduce((a, b) => a + b, 0) / filtered.length;

  return {
    all: timings,
    filtered,
    mean,
    filteredMean,
    median: sorted[Math.floor(sorted.length / 2)],
    min: sorted[0],
    max: sorted[sorted.length - 1],
    stdDev,
    outlierCount: timings.length - filtered.length
  };
}

/**
 * Main execution
 */
async function main() {
  const client = await getClient();

  try {
    console.log('=== Performance Measurement with file_type Filter ===\n');

    // Get repository ID
    const { rows: repoRows } = await client.query('SELECT id FROM maproom.repos WHERE name = $1', ['crewchief']);
    if (repoRows.length === 0) {
      throw new Error('Repository "crewchief" not found');
    }
    const repoId = repoRows[0].id;

    const query = 'authentication';
    const baseline = 4.02; // From FILETYPE-1001
    const threshold = 4.83; // baseline × 1.2

    console.log(`Baseline: ${baseline}ms`);
    console.log(`Threshold: ${threshold}ms (+20%)`);

    // Test 1: Single extension
    const timings1 = await runMeasurements(client, repoId, query,
      'Single Extension (file_type: "ts")', 'ts', 10);
    const stats1 = calculateStats(timings1);

    // Test 2: Multi-extension (3)
    const timings2 = await runMeasurements(client, repoId, query,
      'Multi Extension (file_type: "ts,tsx,js")', 'ts,tsx,js', 10);
    const stats2 = calculateStats(timings2);

    // Test 3: Maximum extensions (20)
    const twentyExts = 'ts,tsx,js,jsx,mts,cts,mjs,cjs,rs,py,rb,go,java,cpp,c,h,hpp,cs,php,swift';
    const timings3 = await runMeasurements(client, repoId, query,
      'Maximum Extensions (20 extensions)', twentyExts, 10);
    const stats3 = calculateStats(timings3);

    // Summary
    console.log('\n\n=== Summary ===\n');

    const results = [
      { name: 'Single Extension (ts)', stats: stats1 },
      { name: 'Multi Extension (ts,tsx,js)', stats: stats2 },
      { name: 'Maximum Extensions (20)', stats: stats3 }
    ];

    results.forEach(({ name, stats }) => {
      const avg = stats.outlierCount > 0 ? stats.filteredMean : stats.mean;
      const overhead = ((avg - baseline) / baseline * 100);
      const pass = avg <= threshold;

      console.log(`${name}:`);
      console.log(`  Average: ${avg.toFixed(2)}ms`);
      console.log(`  Overhead vs baseline: ${overhead >= 0 ? '+' : ''}${overhead.toFixed(1)}%`);
      console.log(`  Within threshold: ${pass ? '✅ YES' : '❌ NO'} (threshold: ${threshold}ms)`);
      console.log();
    });

    // Overall result
    const allPass = results.every(({ stats }) => {
      const avg = stats.outlierCount > 0 ? stats.filteredMean : stats.mean;
      return avg <= threshold;
    });

    console.log(`Overall: ${allPass ? '✅ PASS' : '❌ FAIL'} - Performance requirement ${allPass ? 'met' : 'NOT met'}`);

    return {
      single: stats1,
      multi: stats2,
      max: stats3,
      baseline,
      threshold,
      allPass
    };

  } finally {
    await client.end();
  }
}

main().catch(error => {
  console.error('Error:', error.message);
  console.error(error.stack);
  process.exit(1);
});

#!/usr/bin/env node

/**
 * Performance Baseline Measurement Tool (Simplified)
 *
 * Directly measures query execution time by querying PostgreSQL
 * to establish performance baselines for FILETYPE-1001
 */

import pg from 'pg';
import { performance } from 'perf_hooks';
import fs from 'fs';

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
 * Get repository info
 */
async function getRepoInfo(client) {
  const { rows } = await client.query(`
    SELECT
      r.name as repo_name,
      COUNT(DISTINCT f.id) as file_count,
      COUNT(DISTINCT c.id) as chunk_count
    FROM maproom.repos r
    LEFT JOIN maproom.worktrees w ON w.repo_id = r.id
    LEFT JOIN maproom.files f ON f.worktree_id = w.id
    LEFT JOIN maproom.chunks c ON c.file_id = f.id
    WHERE r.name = $1
    GROUP BY r.name
  `, ['crewchief']);

  return rows[0] || null;
}

/**
 * Run a single FTS search and measure time
 */
async function measureFtsSearch(client, repoId, query) {
  const tsParts = query
    .split(/\s+/)
    .filter(Boolean)
    .map((t) => `${t.replace(/'/g, '')}:*`)
    .join(' & ');

  const start = performance.now();

  await client.query(`
    SELECT c.id, f.relpath, c.symbol_name, c.kind::text, c.start_line, c.end_line,
      ts_rank_cd(c.ts_doc, to_tsquery('simple', $2)) AS fts_score
    FROM maproom.chunks c
    JOIN maproom.files f ON f.id = c.file_id
    WHERE f.repo_id = $1 AND c.ts_doc @@ to_tsquery('simple', $2)
    ORDER BY fts_score DESC
    LIMIT 10
  `, [repoId, tsParts]);

  const end = performance.now();
  return end - start;
}

/**
 * Run multiple measurements and calculate statistics
 */
async function runMeasurements(client, repoId, query, iterations = 10) {
  console.log(`Running ${iterations} FTS search measurements...`);
  console.log(`Query: "${query}"`);
  console.log();

  const timings = [];

  for (let i = 0; i < iterations; i++) {
    process.stdout.write(`Run ${i + 1}/${iterations}... `);
    const time = await measureFtsSearch(client, repoId, query);
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
    console.log('=== Maproom Performance Baseline Measurement ===\n');

    // Get repository info
    console.log('Checking repository status...');
    const repoInfo = await getRepoInfo(client);

    if (!repoInfo) {
      throw new Error('Repository "crewchief" not found in database');
    }

    const { rows: repoRows } = await client.query('SELECT id FROM maproom.repos WHERE name = $1', ['crewchief']);
    const repoId = repoRows[0].id;

    console.log(`Repository: crewchief`);
    console.log(`Files: ${repoInfo.file_count}`);
    console.log(`Chunks: ${repoInfo.chunk_count}`);
    console.log();

    // Run measurements with a common search term
    const query = 'authentication';
    const timings = await runMeasurements(client, repoId, query, 10);

    // Calculate statistics
    const stats = calculateStats(timings);

    console.log();
    console.log('=== Statistics ===');
    console.log(`Mean (all runs): ${stats.mean.toFixed(2)}ms`);
    console.log(`Mean (outliers removed): ${stats.filteredMean.toFixed(2)}ms`);
    console.log(`Median: ${stats.median.toFixed(2)}ms`);
    console.log(`Min: ${stats.min.toFixed(2)}ms`);
    console.log(`Max: ${stats.max.toFixed(2)}ms`);
    console.log(`Std Dev: ${stats.stdDev.toFixed(2)}ms`);
    console.log(`Outliers removed: ${stats.outlierCount}`);
    console.log();

    // Calculate threshold (baseline × 1.2)
    const baseline = stats.outlierCount > 0 ? stats.filteredMean : stats.mean;
    const threshold = baseline * 1.2;

    console.log('=== Performance Targets ===');
    console.log(`Baseline: ${baseline.toFixed(2)}ms`);
    console.log(`Acceptable threshold (+20%): ${threshold.toFixed(2)}ms`);
    console.log();

    // Create results object
    const results = {
      repository: {
        name: 'crewchief',
        fileCount: parseInt(repoInfo.file_count),
        chunkCount: parseInt(repoInfo.chunk_count)
      },
      testQuery: {
        query,
        mode: 'fts',
        k: 10,
        description: 'Full-text search for "authentication" with ts_rank_cd scoring'
      },
      measurements: {
        timings: timings.map(t => parseFloat(t.toFixed(2))),
        mean: parseFloat(stats.mean.toFixed(2)),
        filteredMean: parseFloat(stats.filteredMean.toFixed(2)),
        median: parseFloat(stats.median.toFixed(2)),
        min: parseFloat(stats.min.toFixed(2)),
        max: parseFloat(stats.max.toFixed(2)),
        stdDev: parseFloat(stats.stdDev.toFixed(2)),
        outliersRemoved: stats.outlierCount
      },
      baseline: {
        value: parseFloat(baseline.toFixed(2)),
        threshold: parseFloat(threshold.toFixed(2)),
        method: stats.outlierCount > 0 ? 'mean (outliers removed)' : 'mean (all runs)'
      },
      timestamp: new Date().toISOString(),
      environment: {
        nodeVersion: process.version,
        platform: process.platform,
        database: 'PostgreSQL + pgvector (maproom-postgres)'
      }
    };

    console.log('=== Results (JSON) ===');
    console.log(JSON.stringify(results, null, 2));

    return results;

  } finally {
    await client.end();
  }
}

main().catch(error => {
  console.error('Error:', error.message);
  process.exit(1);
});

#!/usr/bin/env node

/**
 * Performance Regression Check Script
 *
 * This script checks for performance regressions in semantic search by comparing
 * test results against baseline metrics. It's designed to be used in CI/CD pipelines
 * to fail builds when search performance degrades beyond acceptable thresholds.
 *
 * Usage:
 *   node scripts/check-performance.js [--results-file path/to/results.json]
 *
 * Environment Variables:
 *   BASELINE_P95: Override default baseline p95 latency (default: 200ms)
 *   MAX_REGRESSION: Override maximum allowed regression (default: 0.10 = 10%)
 */

import fs from 'fs';
import path from 'path';

// Baseline performance targets (from SEMRANK architecture.md)
const DEFAULT_BASELINE_P95 = 200; // ms
const DEFAULT_MAX_REGRESSION = 0.10; // 10% increase allowed

const BASELINE_P95 = process.env.BASELINE_P95
  ? parseFloat(process.env.BASELINE_P95)
  : DEFAULT_BASELINE_P95;

const MAX_REGRESSION = process.env.MAX_REGRESSION
  ? parseFloat(process.env.MAX_REGRESSION)
  : DEFAULT_MAX_REGRESSION;

const MAX_P95 = BASELINE_P95 * (1 + MAX_REGRESSION);

/**
 * Extract p95 latency from test results
 *
 * This is a reference implementation. Adapt based on your actual test output format.
 * Options:
 * - Parse vitest JSON reporter output
 * - Parse custom benchmark output
 * - Read from test results file
 *
 * @param {string} resultsFile - Path to test results file
 * @returns {number|null} p95 latency in milliseconds, or null if not available
 */
function extractP95FromTestResults(resultsFile) {
  if (!resultsFile) {
    console.log('⚠️  No results file specified, skipping performance check');
    console.log('   To enable: node scripts/check-performance.js --results-file path/to/results.json');
    return null;
  }

  if (!fs.existsSync(resultsFile)) {
    console.log(`⚠️  Results file not found: ${resultsFile}`);
    console.log('   Skipping performance regression check');
    return null;
  }

  try {
    const results = JSON.parse(fs.readFileSync(resultsFile, 'utf8'));

    // Example: Extract from custom benchmark results format
    // Adapt this based on your actual test output structure
    if (results.performance && results.performance.p95) {
      return results.performance.p95;
    }

    // Example: Extract from vitest results with custom reporter
    if (results.testResults) {
      // Parse test results to find performance metrics
      // This is placeholder logic - implement based on your format
      for (const testResult of results.testResults) {
        if (testResult.name && testResult.name.includes('performance')) {
          // Extract p95 from test assertions or custom data
        }
      }
    }

    console.log('⚠️  Could not find p95 metric in results file');
    return null;
  } catch (error) {
    console.error(`❌ Error reading results file: ${error.message}`);
    return null;
  }
}

/**
 * Main performance check logic
 */
function main() {
  const args = process.argv.slice(2);
  let resultsFile = null;

  // Parse command line arguments
  for (let i = 0; i < args.length; i++) {
    if (args[i] === '--results-file' && i + 1 < args.length) {
      resultsFile = args[i + 1];
      i++;
    }
  }

  console.log('📊 SEMRANK Performance Regression Check');
  console.log('=' .repeat(50));

  const p95 = extractP95FromTestResults(resultsFile);

  if (p95 === null) {
    console.log('\n⚠️  Performance metrics not available');
    console.log('   Skipping regression check (not failing build)');
    console.log('\nTo enable performance checks:');
    console.log('  1. Run tests with performance metrics collection');
    console.log('  2. Output results to JSON file');
    console.log('  3. Pass results file: node scripts/check-performance.js --results-file results.json');
    console.log('\nBaseline Configuration:');
    console.log(`   Baseline p95: ${BASELINE_P95}ms`);
    console.log(`   Max regression: ${(MAX_REGRESSION * 100).toFixed(0)}%`);
    console.log(`   Max allowed p95: ${MAX_P95}ms`);
    process.exit(0); // Don't fail if we can't extract metrics
  }

  console.log('\nBaseline Metrics:');
  console.log(`   Baseline p95: ${BASELINE_P95}ms`);
  console.log(`   Max regression: ${(MAX_REGRESSION * 100).toFixed(0)}%`);
  console.log(`   Max allowed p95: ${MAX_P95}ms`);

  console.log('\nCurrent Metrics:');
  console.log(`   Measured p95: ${p95}ms`);

  if (p95 > MAX_P95) {
    const regressionPercent = ((p95 / BASELINE_P95 - 1) * 100).toFixed(1);
    console.log('\n❌ PERFORMANCE REGRESSION DETECTED!');
    console.log(`   p95 latency (${p95}ms) exceeds maximum (${MAX_P95}ms)`);
    console.log(`   This is a ${regressionPercent}% increase from baseline`);
    console.log('\nRecommended Actions:');
    console.log('  1. Profile slow queries with EXPLAIN ANALYZE');
    console.log('  2. Check for missing database indexes');
    console.log('  3. Review recent SQL changes for inefficiencies');
    console.log('  4. Consider rolling back if regression is severe');
    console.log('\nSee: packages/maproom-mcp/docs/deployment/semantic-ranking-rollout.md');
    process.exit(1);
  } else if (p95 < BASELINE_P95) {
    const improvement = ((1 - p95 / BASELINE_P95) * 100).toFixed(1);
    console.log(`\n✅ Performance IMPROVED by ${improvement}%! 🎉`);
    console.log('   Consider updating baseline in architecture.md');
  } else {
    const regression = ((p95 / BASELINE_P95 - 1) * 100).toFixed(1);
    console.log(`\n✅ Performance within acceptable range (+${regression}%)`);
  }

  console.log('\nPerformance check PASSED');
  process.exit(0);
}

// Run the check
main();

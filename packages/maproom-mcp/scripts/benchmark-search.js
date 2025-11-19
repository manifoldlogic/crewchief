#!/usr/bin/env node --loader ts-node/esm
/**
 * Benchmark Script: Baseline Search Quality Metrics (SEMRANK-1005)
 *
 * Measures FTS search performance and quality metrics for 20 golden queries
 * against the test-corpus repository indexed in SEMRANK-1004.
 *
 * Output: CSV file with p50, p95, p99 latencies and ranking behavior
 *
 * Usage:
 *   npx tsx scripts/benchmark-search.ts
 */
import { spawnSync } from 'child_process';
import { writeFileSync } from 'fs';
import { Client } from 'pg';
import path from 'path';
import { fileURLToPath } from 'url';
// ES module __dirname equivalent
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
// Golden query set: 20 representative queries across languages
const GOLDEN_QUERIES = [
    // Exact function names (Rust, TypeScript, Python)
    { query: 'authenticate', description: 'Exact function name across all languages' },
    { query: 'validate_token', description: 'Exact Python/Rust function name (snake_case)' },
    { query: 'validateToken', description: 'Exact TypeScript function name (camelCase)' },
    { query: 'create_session', description: 'Exact Python/Rust function name' },
    { query: 'connect_database', description: 'Exact Rust function name' },
    // Class and method names
    { query: 'DatabaseConnection', description: 'Exact Python class name' },
    { query: 'AuthenticationError', description: 'Exact Python exception class' },
    { query: 'execute_query', description: 'Method name across classes' },
    // React hooks (TypeScript specific)
    { query: 'useAuth', description: 'React hook name' },
    { query: 'login', description: 'Function in React hook' },
    // Concept searches (multi-word)
    { query: 'user authentication', description: 'Concept search for auth functionality' },
    { query: 'database connection', description: 'Concept search for DB functionality' },
    { query: 'session management', description: 'Concept search for session handling' },
    { query: 'token validation', description: 'Concept search for validation logic' },
    // Documentation/markdown searches
    { query: 'API reference', description: 'Documentation heading search' },
    { query: 'Python Authentication', description: 'Language-specific docs' },
    // Edge cases
    { query: 'test_authenticate', description: 'Test function names (should rank lower than impl)' },
    { query: 'close', description: 'Short common word (method name)' },
    { query: '__init__', description: 'Python dunder method' },
    { query: 'SEMRANK', description: 'Project acronym in README' },
];
const REPO = 'test-corpus';
const WORKTREE = 'main';
const ITERATIONS = 100;
const WARMUP_ITERATIONS = 10;
// Database connection
const DB_URL = process.env.DATABASE_URL || 'postgresql://maproom:maproom@maproom-postgres:5432/maproom';
// Binary path
const BINARY_PATH = process.env.CREWCHIEF_MAPROOM_BIN ||
    path.resolve(__dirname, '../../cli/bin/linux-arm64/crewchief-maproom');
/**
 * Calculate percentile from sorted array
 */
function percentile(sorted, p) {
    if (sorted.length === 0)
        return 0;
    const index = Math.ceil((p / 100) * sorted.length) - 1;
    return sorted[Math.max(0, index)];
}
/**
 * Execute a single search query via Rust binary
 */
function executeSearch(query) {
    const startTime = Date.now();
    const result = spawnSync(BINARY_PATH, [
        'search',
        '--repo', REPO,
        '--worktree', WORKTREE,
        '--query', query,
        '--k', '20',
    ], {
        encoding: 'utf8',
        timeout: 5000,
    });
    const latencyMs = Date.now() - startTime;
    if (result.error) {
        throw new Error(`Failed to execute search: ${result.error.message}`);
    }
    if (result.status !== 0) {
        throw new Error(`Search failed with status ${result.status}: ${result.stderr}`);
    }
    try {
        const searchResult = JSON.parse(result.stdout);
        return { result: searchResult, latencyMs };
    }
    catch (error) {
        throw new Error(`Failed to parse search output: ${error instanceof Error ? error.message : String(error)}`);
    }
}
/**
 * Analyze search results to find implementation vs test/doc ranking
 */
function analyzeRanking(hits) {
    // Get top 3 kinds
    const top_3_kinds = hits
        .slice(0, 3)
        .map(h => h.kind)
        .join(',');
    // Find first implementation (func, class, method from src/)
    const implementation_rank = hits.findIndex(h => {
        const isImpl = ['func', 'class', 'method', 'hook', 'component'].includes(h.kind);
        const notTest = !h.file_relpath.includes('/test') && !h.file_relpath.includes('_test.');
        const notDoc = !h.file_relpath.includes('/doc');
        return isImpl && notTest && notDoc;
    });
    // Find first test (func from test/)
    const test_rank = hits.findIndex(h => {
        return h.file_relpath.includes('/test') || h.file_relpath.includes('_test.');
    });
    // Find first doc (heading, markdown_section)
    const doc_rank = hits.findIndex(h => {
        return h.kind.startsWith('heading_') || h.kind === 'markdown_section' ||
            h.file_relpath.includes('/doc') || h.file_relpath.endsWith('.md');
    });
    return {
        top_3_kinds,
        implementation_rank: implementation_rank >= 0 ? implementation_rank + 1 : null,
        test_rank: test_rank >= 0 ? test_rank + 1 : null,
        doc_rank: doc_rank >= 0 ? doc_rank + 1 : null,
    };
}
/**
 * Run benchmark for a single query
 */
async function benchmarkQuery(query, description) {
    console.log(`\nBenchmarking: "${query}" (${description})`);
    // Warmup
    console.log(`  Warming up (${WARMUP_ITERATIONS} iterations)...`);
    for (let i = 0; i < WARMUP_ITERATIONS; i++) {
        executeSearch(query);
    }
    // Measure
    console.log(`  Measuring (${ITERATIONS} iterations)...`);
    const latencies = [];
    let lastResult = null;
    for (let i = 0; i < ITERATIONS; i++) {
        const { result, latencyMs } = executeSearch(query);
        latencies.push(latencyMs);
        lastResult = result;
        // Progress indicator
        if ((i + 1) % 20 === 0) {
            process.stdout.write('.');
        }
    }
    console.log('');
    // Calculate percentiles
    const sortedLatencies = [...latencies].sort((a, b) => a - b);
    const p50 = percentile(sortedLatencies, 50);
    const p95 = percentile(sortedLatencies, 95);
    const p99 = percentile(sortedLatencies, 99);
    // Analyze ranking behavior
    const ranking = lastResult ? analyzeRanking(lastResult.hits) : {
        top_3_kinds: '',
        implementation_rank: null,
        test_rank: null,
        doc_rank: null,
    };
    console.log(`  p50: ${p50}ms, p95: ${p95}ms, p99: ${p99}ms`);
    console.log(`  Top 3 kinds: ${ranking.top_3_kinds}`);
    console.log(`  Impl rank: ${ranking.implementation_rank ?? 'N/A'}, Test rank: ${ranking.test_rank ?? 'N/A'}, Doc rank: ${ranking.doc_rank ?? 'N/A'}`);
    return {
        query,
        description,
        latency_p50_ms: p50,
        latency_p95_ms: p95,
        latency_p99_ms: p99,
        ...ranking,
    };
}
/**
 * Get EXPLAIN ANALYZE output for a query
 */
async function getQueryPlan(client, query) {
    // Convert search query to tsquery format
    const tsquery = query
        .toLowerCase()
        .split(/\s+/)
        .filter(w => w.length > 0)
        .join(' & ');
    const sql = `
    EXPLAIN (ANALYZE, BUFFERS, VERBOSE)
    WITH ranked_chunks AS (
      SELECT
        c.id,
        f.relpath,
        c.symbol_name,
        c.kind::text,
        c.start_line,
        c.end_line,
        ts_rank_cd(c.ts_doc, to_tsquery('simple', $1)) AS rank
      FROM maproom.chunks c
      JOIN maproom.files f ON f.id = c.file_id
      JOIN maproom.repos r ON r.id = f.repo_id
      WHERE r.name = $2
        AND ($3::text IS NULL OR f.worktree_id = (SELECT id FROM maproom.worktrees WHERE name = $3 AND repo_id = r.id))
        AND c.ts_doc @@ to_tsquery('simple', $1)
      ORDER BY rank DESC
      LIMIT 20
    )
    SELECT * FROM ranked_chunks;
  `;
    const result = await client.query(sql, [tsquery, REPO, WORKTREE]);
    return result.rows.map(row => row['QUERY PLAN']).join('\n');
}
/**
 * Main benchmark execution
 */
async function main() {
    console.log('=== SEMRANK-1005: Baseline Search Quality Metrics ===\n');
    console.log(`Repository: ${REPO}`);
    console.log(`Worktree: ${WORKTREE}`);
    console.log(`Iterations per query: ${ITERATIONS} (after ${WARMUP_ITERATIONS} warmup)`);
    console.log(`Binary: ${BINARY_PATH}`);
    console.log(`Queries: ${GOLDEN_QUERIES.length}\n`);
    // Verify binary exists
    const binaryCheck = spawnSync(BINARY_PATH, ['--version'], { encoding: 'utf8' });
    if (binaryCheck.error || binaryCheck.status !== 0) {
        console.error('ERROR: crewchief-maproom binary not found or not executable');
        console.error(`Path: ${BINARY_PATH}`);
        console.error('Build it with: cargo build --release --bin crewchief-maproom');
        process.exit(1);
    }
    console.log(`Binary version: ${binaryCheck.stdout.trim()}\n`);
    // Connect to database for query plans
    const client = new Client({ connectionString: DB_URL });
    await client.connect();
    console.log('Database connected\n');
    // Run benchmarks
    const results = [];
    for (const { query, description } of GOLDEN_QUERIES) {
        try {
            const result = await benchmarkQuery(query, description);
            results.push(result);
        }
        catch (error) {
            console.error(`  ERROR: ${error instanceof Error ? error.message : String(error)}`);
            results.push({
                query,
                description,
                latency_p50_ms: 0,
                latency_p95_ms: 0,
                latency_p99_ms: 0,
                top_3_kinds: 'ERROR',
                implementation_rank: null,
                test_rank: null,
                doc_rank: null,
            });
        }
    }
    // Get EXPLAIN ANALYZE for 5 representative queries
    console.log('\n=== Query Plans (EXPLAIN ANALYZE) ===\n');
    const planQueries = [
        'authenticate',
        'user authentication',
        'test_authenticate',
        'DatabaseConnection',
        'API reference',
    ];
    const queryPlans = {};
    for (const query of planQueries) {
        try {
            console.log(`Getting query plan for: "${query}"`);
            const plan = await getQueryPlan(client, query);
            queryPlans[query] = plan;
            console.log(plan);
            console.log('');
        }
        catch (error) {
            console.error(`  ERROR: ${error instanceof Error ? error.message : String(error)}`);
        }
    }
    await client.end();
    // Generate CSV output
    const csvPath = path.resolve(__dirname, '../benchmarks/baseline-fts.csv');
    const csvHeader = 'query,description,latency_p50_ms,latency_p95_ms,latency_p99_ms,top_3_kinds,implementation_rank,test_rank,doc_rank\n';
    const csvRows = results.map(r => `"${r.query}","${r.description}",${r.latency_p50_ms},${r.latency_p95_ms},${r.latency_p99_ms},"${r.top_3_kinds}",${r.implementation_rank ?? ''},${r.test_rank ?? ''},${r.doc_rank ?? ''}`).join('\n');
    writeFileSync(csvPath, csvHeader + csvRows + '\n');
    console.log(`\n=== Results Written ===`);
    console.log(`CSV: ${csvPath}\n`);
    // Summary statistics
    const avgP50 = results.reduce((sum, r) => sum + r.latency_p50_ms, 0) / results.length;
    const avgP95 = results.reduce((sum, r) => sum + r.latency_p95_ms, 0) / results.length;
    const avgP99 = results.reduce((sum, r) => sum + r.latency_p99_ms, 0) / results.length;
    const implBeforeTest = results.filter(r => r.implementation_rank !== null &&
        r.test_rank !== null &&
        r.implementation_rank < r.test_rank).length;
    const testBeforeImpl = results.filter(r => r.implementation_rank !== null &&
        r.test_rank !== null &&
        r.test_rank < r.implementation_rank).length;
    console.log('=== Summary ===');
    console.log(`Average latency: p50=${avgP50.toFixed(1)}ms, p95=${avgP95.toFixed(1)}ms, p99=${avgP99.toFixed(1)}ms`);
    console.log(`Ranking behavior:`);
    console.log(`  Implementation ranks before test: ${implBeforeTest}/${results.length}`);
    console.log(`  Test ranks before implementation: ${testBeforeImpl}/${results.length}`);
    console.log('');
    // Save query plans to file
    const plansPath = path.resolve(__dirname, '../benchmarks/baseline-query-plans.txt');
    const plansContent = Object.entries(queryPlans)
        .map(([query, plan]) => `=== Query: "${query}" ===\n\n${plan}\n\n`)
        .join('\n');
    writeFileSync(plansPath, plansContent);
    console.log(`Query plans: ${plansPath}`);
    console.log('\nBenchmark complete!');
}
main().catch(error => {
    console.error('FATAL ERROR:', error);
    process.exit(1);
});

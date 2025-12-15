// Performance validation for quality-weighted graph importance query (SRCHREL-0002)
//
// This test validates that the quality-weighted SQL query meets performance
// budget (<30ms p95 latency) on real CrewChief database.
//
// Methodology:
// 1. Connect to real database at ~/.maproom/maproom.db
// 2. Run quality-weighted query multiple times (20+ iterations)
// 3. Measure p50, p95, p99 latencies
// 4. Run EXPLAIN QUERY PLAN to verify index usage
// 5. Confirm no full table scans

use rusqlite::{params, Connection};
use std::time::{Duration, Instant};

#[derive(Debug)]
struct PerformanceStats {
    iterations: usize,
    min: Duration,
    max: Duration,
    mean: Duration,
    p50: Duration,
    p95: Duration,
    p99: Duration,
}

impl PerformanceStats {
    fn from_samples(mut samples: Vec<Duration>) -> Self {
        samples.sort();
        let count = samples.len();

        let sum: Duration = samples.iter().sum();
        let mean = sum / count as u32;

        let p50_idx = (count as f64 * 0.50) as usize;
        let p95_idx = (count as f64 * 0.95) as usize;
        let p99_idx = (count as f64 * 0.99) as usize;

        Self {
            iterations: count,
            min: samples[0],
            max: samples[count - 1],
            mean,
            p50: samples[p50_idx],
            p95: samples[p95_idx],
            p99: samples[p99_idx],
        }
    }

    fn print_report(&self) {
        println!("\n=== PERFORMANCE STATISTICS ===");
        println!("Iterations: {}", self.iterations);
        println!("Min:        {:?}", self.min);
        println!("Mean:       {:?}", self.mean);
        println!("P50:        {:?}", self.p50);
        println!("P95:        {:?}", self.p95);
        println!("P99:        {:?}", self.p99);
        println!("Max:        {:?}", self.max);
        println!("==============================\n");
    }
}

/// Quality-weighted graph importance SQL query (Phase 1: calls edges only)
/// Note: ln() is computed in Rust after fetching to avoid SQLite function compatibility issues
const QUALITY_WEIGHTED_GRAPH_SQL: &str = r#"
WITH quality_edges AS (
  SELECT
    ce.dst_chunk_id as chunk_id,
    -- Edge type weight (Phase 1: calls only)
    CASE ce.type
      WHEN 'calls' THEN 1.0
      ELSE 1.0
    END *
    -- Source code type weight (test detection)
    CASE
      WHEN src_file.relpath LIKE '%/test/%'
        OR src_file.relpath LIKE '%/tests/%'
        OR src_file.relpath LIKE '%/__tests__/%'
        OR src_file.relpath LIKE '%.test.%'
        OR src_file.relpath LIKE '%.spec.%'
        OR src_file.relpath LIKE '%_test.%'
        OR src_chunk.kind LIKE '%test%'
      THEN 0.5  -- Test code penalty
      ELSE 1.0  -- Production code baseline
    END as edge_quality
  FROM chunk_edges ce
  JOIN chunks src_chunk ON src_chunk.id = ce.src_chunk_id
  JOIN files src_file ON src_file.id = src_chunk.file_id
  WHERE ce.dst_chunk_id IN (
    SELECT c.id FROM chunks c
    JOIN files f ON f.id = c.file_id
    WHERE f.repo_id = ?1
      AND (?2 IS NULL OR f.worktree_id = ?2)
  )
),
importance_scores AS (
  SELECT
    chunk_id,
    SUM(edge_quality) as quality_weighted_sum
  FROM quality_edges
  GROUP BY chunk_id
)
SELECT
  chunk_id,
  COALESCE(quality_weighted_sum, 0.0) as quality_weighted_sum
FROM importance_scores
ORDER BY quality_weighted_sum DESC
LIMIT ?3
"#;

/// EXPLAIN QUERY PLAN for the quality-weighted query
const EXPLAIN_QUERY_PLAN_SQL: &str = r#"
EXPLAIN QUERY PLAN
WITH quality_edges AS (
  SELECT
    ce.dst_chunk_id as chunk_id,
    CASE ce.type
      WHEN 'calls' THEN 1.0
      ELSE 1.0
    END *
    CASE
      WHEN src_file.relpath LIKE '%/test/%'
        OR src_file.relpath LIKE '%/tests/%'
        OR src_file.relpath LIKE '%/__tests__/%'
        OR src_file.relpath LIKE '%.test.%'
        OR src_file.relpath LIKE '%.spec.%'
        OR src_file.relpath LIKE '%_test.%'
        OR src_chunk.kind LIKE '%test%'
      THEN 0.5
      ELSE 1.0
    END as edge_quality
  FROM chunk_edges ce
  JOIN chunks src_chunk ON src_chunk.id = ce.src_chunk_id
  JOIN files src_file ON src_file.id = src_chunk.file_id
  WHERE ce.dst_chunk_id IN (
    SELECT c.id FROM chunks c
    JOIN files f ON f.id = c.file_id
    WHERE f.repo_id = ?1
      AND (?2 IS NULL OR f.worktree_id = ?2)
  )
),
importance_scores AS (
  SELECT
    chunk_id,
    SUM(edge_quality) as quality_weighted_sum
  FROM quality_edges
  GROUP BY chunk_id
)
SELECT
  chunk_id,
  COALESCE(quality_weighted_sum, 0.0) as quality_weighted_sum
FROM importance_scores
ORDER BY quality_weighted_sum DESC
LIMIT ?3
"#;

#[test]
fn test_quality_weighted_performance_validation() {
    // Connect to real database
    let db_path = dirs::home_dir()
        .expect("Failed to get home directory")
        .join(".maproom/maproom.db");

    println!("\n=== SRCHREL-0002: SQL Performance Validation ===");
    println!("Database: {}", db_path.display());

    let conn = Connection::open(&db_path)
        .expect("Failed to open database - ensure ~/.maproom/maproom.db exists");

    // Gather database statistics
    let total_chunks: i64 = conn
        .query_row("SELECT COUNT(*) FROM chunks", [], |row| row.get(0))
        .expect("Failed to count chunks");

    let total_edges: i64 = conn
        .query_row("SELECT COUNT(*) FROM chunk_edges", [], |row| row.get(0))
        .expect("Failed to count edges");

    let calls_edges: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM chunk_edges WHERE type = 'calls'",
            [],
            |row| row.get(0),
        )
        .expect("Failed to count calls edges");

    let repo_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM repos", [], |row| row.get(0))
        .expect("Failed to count repos");

    println!("\n=== DATABASE STATISTICS ===");
    println!("Total chunks: {}", total_chunks);
    println!("Total edges: {}", total_edges);
    println!(
        "Calls edges: {} ({:.1}%)",
        calls_edges,
        (calls_edges as f64 / total_edges as f64) * 100.0
    );
    println!("Repositories: {}", repo_count);
    println!("===========================\n");

    // Get repo_id with most edges for testing
    let repo_id: i64 = conn
        .query_row(
            "SELECT f.repo_id FROM chunk_edges ce
             JOIN chunks c ON c.id = ce.dst_chunk_id
             JOIN files f ON f.id = c.file_id
             GROUP BY f.repo_id
             ORDER BY COUNT(*) DESC
             LIMIT 1",
            [],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| {
            // Fallback to first repo if no edges exist
            conn.query_row("SELECT id FROM repos LIMIT 1", [], |row| row.get(0))
                .expect("No repos found in database")
        });

    let edges_in_repo: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM chunk_edges ce
             JOIN chunks c ON c.id = ce.dst_chunk_id
             JOIN files f ON f.id = c.file_id
             WHERE f.repo_id = ?1",
            [repo_id],
            |row| row.get(0),
        )
        .unwrap_or(0);

    println!(
        "Testing with repo_id={} ({} edges), worktree_id=NULL (all worktrees), limit=10\n",
        repo_id, edges_in_repo
    );

    // === EXPLAIN QUERY PLAN Analysis ===
    println!("=== EXPLAIN QUERY PLAN ===");
    let mut stmt = conn
        .prepare(EXPLAIN_QUERY_PLAN_SQL)
        .expect("Failed to prepare EXPLAIN statement");
    let mut rows = stmt
        .query(params![repo_id, Option::<i64>::None, 10])
        .expect("Failed to execute EXPLAIN");

    let mut has_scan = false;
    let mut plan_lines = Vec::new();

    while let Some(row) = rows.next().expect("Failed to read EXPLAIN row") {
        let id: i32 = row.get(0).unwrap_or(0);
        let parent: i32 = row.get(1).unwrap_or(0);
        let notused: i32 = row.get(2).unwrap_or(0);
        let detail: String = row.get(3).unwrap_or_default();

        plan_lines.push(format!("{:2} {:2} {:2} {}", id, parent, notused, detail));

        // Check for full table scans (bad - should use indexes)
        if detail.contains("SCAN TABLE") && !detail.contains("SCAN TABLE sqlite_") {
            has_scan = true;
            println!("⚠️  WARNING: Full table scan detected: {}", detail);
        }

        // Check for index usage (good)
        if detail.contains("USING INDEX") || detail.contains("SEARCH TABLE") {
            println!("✓  Index usage: {}", detail);
        }
    }

    println!("\nFull EXPLAIN QUERY PLAN:");
    for line in &plan_lines {
        println!("  {}", line);
    }
    println!("===========================\n");

    if has_scan {
        println!("⚠️  WARNING: Query uses full table scans - may need index optimization\n");
    } else {
        println!("✓  No full table scans detected - query uses indexes efficiently\n");
    }

    // === PERFORMANCE BENCHMARK ===
    println!("=== RUNNING PERFORMANCE BENCHMARK ===");
    println!("Warming up (3 iterations)...");

    // Warm up (populate OS page cache)
    for _ in 0..3 {
        let mut stmt = conn
            .prepare(QUALITY_WEIGHTED_GRAPH_SQL)
            .expect("Failed to prepare query");
        let results: Vec<(i64, f64)> = stmt
            .query_map(params![repo_id, Option::<i64>::None, 10], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .expect("Failed to execute query")
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to collect results");
        let _: Vec<(i64, f64)> = results
            .into_iter()
            .map(|(chunk_id, sum)| (chunk_id, (2.0 + sum).ln()))
            .collect();
    }

    println!("Running timed iterations (25 iterations)...\n");

    // Benchmark iterations
    let iterations = 25;
    let mut samples = Vec::with_capacity(iterations);

    for i in 0..iterations {
        let start = Instant::now();

        let mut stmt = conn
            .prepare(QUALITY_WEIGHTED_GRAPH_SQL)
            .expect("Failed to prepare query");
        let results: Vec<(i64, f64)> = stmt
            .query_map(params![repo_id, Option::<i64>::None, 10], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .expect("Failed to execute query")
            .collect::<Result<Vec<_>, _>>()
            .expect("Failed to collect results");

        // Apply logarithmic scaling in Rust: ln(2.0 + quality_weighted_sum)
        let _scores: Vec<(i64, f64)> = results
            .into_iter()
            .map(|(chunk_id, sum)| (chunk_id, (2.0 + sum).ln()))
            .collect();

        let elapsed = start.elapsed();
        samples.push(elapsed);

        let result_count = _scores.len();

        if i < 5 || i >= iterations - 5 {
            println!(
                "  Iteration {:2}: {:?} ({} results)",
                i + 1,
                elapsed,
                result_count
            );
        } else if i == 5 {
            println!("  ... (iterations 6-{} omitted) ...", iterations - 4);
        }
    }

    // Calculate and print statistics
    let stats = PerformanceStats::from_samples(samples);
    stats.print_report();

    // === VALIDATION AGAINST TARGETS ===
    println!("=== VALIDATION AGAINST TARGETS ===");

    let p50_target_ms = 15.0;
    let p95_target_ms = 30.0;
    let p99_target_ms = 50.0;

    let p50_ms = stats.p50.as_secs_f64() * 1000.0;
    let p95_ms = stats.p95.as_secs_f64() * 1000.0;
    let p99_ms = stats.p99.as_secs_f64() * 1000.0;

    println!(
        "P50 latency: {:.2}ms (target: <{:.0}ms) {}",
        p50_ms,
        p50_target_ms,
        if p50_ms < p50_target_ms {
            "✓ PASS"
        } else {
            "✗ FAIL"
        }
    );

    println!(
        "P95 latency: {:.2}ms (target: <{:.0}ms) {}",
        p95_ms,
        p95_target_ms,
        if p95_ms < p95_target_ms {
            "✓ PASS"
        } else {
            "✗ FAIL"
        }
    );

    println!(
        "P99 latency: {:.2}ms (target: <{:.0}ms) {}",
        p99_ms,
        p99_target_ms,
        if p99_ms < p99_target_ms {
            "✓ PASS"
        } else {
            "✗ FAIL"
        }
    );

    println!(
        "No full table scans: {}",
        if !has_scan { "✓ PASS" } else { "✗ FAIL" }
    );

    println!("===================================\n");

    // Assert on critical requirements
    assert!(
        p95_ms < p95_target_ms,
        "P95 latency {:.2}ms exceeds target {:.0}ms - query optimization needed",
        p95_ms,
        p95_target_ms
    );

    assert!(
        !has_scan,
        "Query uses full table scans - index optimization required"
    );

    println!("✓ All performance validation checks passed!\n");
}

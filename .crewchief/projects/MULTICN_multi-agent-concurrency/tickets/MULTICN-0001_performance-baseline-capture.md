# Ticket: MULTICN-0001: Capture Performance Baseline

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- rust-indexer-engineer
- verify-ticket
- commit-ticket

## Summary

Establish a performance baseline before implementing multi-agent concurrency changes. Capture search latency, index time, and memory usage metrics to enable before/after comparison.

## Background

Before modifying the maproom daemon architecture to support multiple concurrent agents, we need quantifiable baseline metrics. This allows us to verify that the new architecture delivers expected improvements (memory reduction, elimination of SQLITE_BUSY errors) without regressing performance.

Reference: [plan.md](../planning/plan.md) - Phase 0: Baseline Capture

## Acceptance Criteria

- [ ] Benchmark script created or existing `crewchief-maproom bench` command used
- [ ] Search latency metrics captured (p50, p95, p99) for 100 queries
- [ ] Index time captured for 1000 files
- [ ] Memory usage captured with 1 agent and 3 agents
- [ ] Baseline data saved to `planning/performance-baseline.json`
- [ ] Baseline data committed to repository

## Technical Requirements

Create or run benchmark measuring:

### Search Latency Metrics
- Run 100 search queries across indexed repository
- Capture response times (milliseconds)
- Calculate percentiles: p50 (median), p95, p99
- Query mix: code search, hybrid search, vector search

### Index Performance
- Index 1000 files from test repository
- Measure total indexing time (seconds)
- Record memory usage during indexing

### Memory Usage
- **Single agent**: Measure resident memory (RSS) with one daemon
- **Three agents**: Spawn 3 daemon processes, measure total RSS
- Capture peak memory usage during concurrent operations

### Output Format

Save to `planning/performance-baseline.json`:

```json
{
  "timestamp": "2025-12-05T...",
  "git_commit": "...",
  "metrics": {
    "search_latency_ms": {
      "p50": 45.2,
      "p95": 112.5,
      "p99": 245.0
    },
    "index_time_seconds": 15.3,
    "memory_usage_mb": {
      "single_agent": 95,
      "three_agents": 285
    }
  },
  "environment": {
    "os": "Linux 6.12.54",
    "rust_version": "...",
    "test_repo_size": "1000 files"
  }
}
```

## Implementation Notes

### Option 1: Use Existing Bench Command

If `crewchief-maproom bench` exists:
```bash
cargo run --bin crewchief-maproom bench > planning/performance-baseline.json
```

### Option 2: Create Benchmark Script

If no bench command exists, create `scripts/baseline-benchmark.sh`:

```bash
#!/usr/bin/env bash
# Baseline performance measurement script

set -e

REPO_PATH="${1:-.}"
RESULTS_FILE="planning/performance-baseline.json"

# Index test repository
echo "Indexing repository..."
time cargo run --bin crewchief-maproom index "$REPO_PATH"

# Run search queries
echo "Running search queries..."
for i in {1..100}; do
  cargo run --bin crewchief-maproom search "function" --limit 10
done

# Measure memory (requires 3 daemon processes)
echo "Measuring memory with 3 agents..."
# Launch 3 daemons in background, measure RSS

# Format results to JSON
echo "Saving results to $RESULTS_FILE..."
```

### Memory Measurement

Use `ps` or `/proc` to measure RSS:
```bash
# Single daemon
ps -o rss= -p $PID

# Three daemons
ps -o rss= -p $PID1,$PID2,$PID3 | awk '{sum+=$1} END {print sum}'
```

### Test Environment

- Use CrewChief repository as test data (or create test repo with ~1000 files)
- Ensure clean state (no existing index)
- Run on same machine that will run final tests for consistency

## Dependencies

- None (first ticket in project)

## Risk Assessment

- **Risk**: Benchmark results vary by machine/environment
  - **Mitigation**: Document environment details in baseline file. Run on same machine for comparison tests.

- **Risk**: No existing bench command requires creating script
  - **Mitigation**: Simple bash script is acceptable. Focus on capturing numbers, not perfect benchmarking.

- **Risk**: Memory measurement complexity
  - **Mitigation**: Simple RSS measurement via `ps` is sufficient. Document measurement method.

## Files/Packages Affected

- `planning/performance-baseline.json` (NEW)
- `scripts/baseline-benchmark.sh` (NEW - optional, if bench command doesn't exist)

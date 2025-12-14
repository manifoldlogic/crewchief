#!/bin/bash
# Calculate percentile statistics from baseline results (simplified version)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RESULTS_FILE="$SCRIPT_DIR/baseline-results.jsonl"

if [ ! -f "$RESULTS_FILE" ]; then
    echo "Error: Results file not found at $RESULTS_FILE"
    echo "Run measure-baseline-simple.sh first"
    exit 1
fi

echo "=== Calculating Performance Percentiles ==="
echo ""

# Count total queries
total=$(wc -l < "$RESULTS_FILE")
echo "Total queries: $total"
echo ""

# Extract latencies and sort them
latencies=$(jq -r '.latency_ms' "$RESULTS_FILE" | sort -n)

# Calculate percentile index
calc_index() {
    local percentile=$1
    awk "BEGIN {printf \"%d\", ($total * $percentile / 100)}"
}

# Get value at index
get_value() {
    local index=$1
    echo "$latencies" | sed -n "$((index + 1))p"
}

# Calculate percentiles
p50_index=$(calc_index 50)
p95_index=$(calc_index 95)
p99_index=$(calc_index 99)

p50=$(get_value "$p50_index")
p95=$(get_value "$p95_index")
p99=$(get_value "$p99_index")

echo "### Latency Percentiles"
printf "  p50 (median): %.2fms\n" "$p50"
printf "  p95: %.2fms\n" "$p95"
printf "  p99: %.2fms\n" "$p99"
echo ""

# Calculate average, min, max
avg=$(jq -s 'map(.latency_ms) | add / length' "$RESULTS_FILE")
min=$(jq -s 'map(.latency_ms) | min' "$RESULTS_FILE")
max=$(jq -s 'map(.latency_ms) | max' "$RESULTS_FILE")

echo "### Latency Range"
printf "  Average: %.2fms\n" "$avg"
printf "  Min: %.2fms\n" "$min"
printf "  Max: %.2fms\n" "$max"
echo ""

# Export summary for documentation
cat > "$SCRIPT_DIR/percentile-summary.json" << EOF
{
  "date": "$(date -u +%Y-%m-%d)",
  "git_commit": "$(git rev-parse HEAD 2>/dev/null || echo 'unknown')",
  "total_queries": $total,
  "latency_percentiles_ms": {
    "p50": $p50,
    "p95": $p95,
    "p99": $p99
  },
  "latency_statistics_ms": {
    "average": $avg,
    "min": $min,
    "max": $max
  },
  "test_configuration": {
    "repo": "crewchief",
    "worktree": "main",
    "search_mode": "fts",
    "result_limit": 10,
    "indexed_chunks": "23232"
  }
}
EOF

echo "=== Summary exported to percentile-summary.json ==="
echo ""
cat "$SCRIPT_DIR/percentile-summary.json"

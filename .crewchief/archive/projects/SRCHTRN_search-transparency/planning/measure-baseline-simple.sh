#!/bin/bash
# Simplified Performance Baseline Measurement Script
# Uses wall-clock timing instead of parsing JSON metadata

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKLOAD_FILE="$SCRIPT_DIR/test-workload.txt"
RESULTS_FILE="$SCRIPT_DIR/baseline-results.jsonl"
BINARY="${BINARY:-./target/release/crewchief-maproom}"

echo "=== Search Performance Baseline Measurement ==="
echo "Binary: $BINARY"
echo "Workload: $WORKLOAD_FILE"
echo "Results: $RESULTS_FILE"
echo ""

# Verify binary exists
if [ ! -f "$BINARY" ]; then
    echo "Error: Binary not found at $BINARY"
    echo "Run: cargo build --release --bin crewchief-maproom"
    exit 1
fi

# Verify workload exists
if [ ! -f "$WORKLOAD_FILE" ]; then
    echo "Error: Workload file not found at $WORKLOAD_FILE"
    exit 1
fi

# Clear previous results
> "$RESULTS_FILE"

# Extract queries from workload file (ignore comments and empty lines)
queries=()
while IFS= read -r line; do
    # Skip empty lines and comments
    if [[ -z "$line" || "$line" =~ ^[[:space:]]*# ]]; then
        continue
    fi
    queries+=("$line")
done < "$WORKLOAD_FILE"

total_queries=${#queries[@]}
echo "Loaded $total_queries queries from workload file"
echo ""

# Run each query and collect timing
echo "Running queries..."
count=0
for query in "${queries[@]}"; do
    count=$((count + 1))

    # Measure wall-clock time using GNU time format or date
    start=$(date +%s%N)

    # Run search (suppress output to avoid clutter, capture exit code)
    "$BINARY" search \
        --repo crewchief \
        --query "$query" \
        --k 10 \
        > /dev/null 2>&1 || true

    end=$(date +%s%N)

    # Calculate latency in milliseconds
    latency_ns=$((end - start))
    latency_ms=$(awk "BEGIN {printf \"%.3f\", $latency_ns / 1000000}")

    echo "[$count/$total_queries] '$query' -> ${latency_ms}ms"

    # Record result as JSON
    jq -n \
        --arg query "$query" \
        --arg latency "$latency_ms" \
        '{
            query: $query,
            latency_ms: ($latency | tonumber)
        }' >> "$RESULTS_FILE"
done

echo ""
echo "=== Measurement Complete ==="
echo "Collected $total_queries measurements"
echo "Results saved to: $RESULTS_FILE"
echo ""
echo "Run calculate-percentiles-simple.sh to compute statistics"

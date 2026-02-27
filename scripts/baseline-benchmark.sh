#!/usr/bin/env bash
# Baseline performance measurement script for MULTICN-0001
# Captures search latency, index time, and memory usage metrics

set -e

# Configuration
REPO_PATH="${1:-/workspace}"
RESULTS_FILE="/workspace/.crewchief/projects/MULTICN_multi-agent-concurrency/planning/performance-baseline.json"
MAPROOM_BIN="/workspace/target/release/maproom"
TEMP_DB=$(mktemp -d)/baseline.db
export MAPROOM_DATABASE_URL="sqlite:///${TEMP_DB}"

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}Maproom Performance Baseline Capture${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Check if binary exists
if [ ! -f "$MAPROOM_BIN" ]; then
    echo -e "${YELLOW}Building maproom binary...${NC}"
    cargo build --release -p maproom
fi

# Initialize database
echo -e "${GREEN}[1/5] Initializing database...${NC}"
"$MAPROOM_BIN" db migrate > /dev/null 2>&1

# Get git info
GIT_COMMIT=$(git -C "$REPO_PATH" rev-parse HEAD)
RUST_VERSION=$(rustc --version | awk '{print $2}')
OS_VERSION=$(uname -sr)

# Count files in repository
FILE_COUNT=$(find "$REPO_PATH" -type f -name "*.rs" -o -name "*.ts" -o -name "*.js" -o -name "*.tsx" -o -name "*.jsx" | wc -l | xargs)

# === INDEX PERFORMANCE ===
echo -e "${GREEN}[2/5] Measuring index time...${NC}"
INDEX_START=$(date +%s.%N)

"$MAPROOM_BIN" scan \
    --path "$REPO_PATH" \
    --repo crewchief \
    --worktree baseline-test \
    --generate-embeddings false \
    --json > /dev/null 2>&1

INDEX_END=$(date +%s.%N)
INDEX_TIME=$(awk "BEGIN {print $INDEX_END - $INDEX_START}")
INDEX_TIME_FORMATTED=$(printf "%.2f" "$INDEX_TIME")

echo "   Index time: ${INDEX_TIME_FORMATTED}s for ${FILE_COUNT} files"

# === SEARCH LATENCY ===
echo -e "${GREEN}[3/5] Measuring search latency (100 queries)...${NC}"

# Diverse query set to simulate real usage
QUERIES=(
    "function"
    "async"
    "error"
    "database"
    "search"
    "index"
    "worktree"
    "chunk"
    "embedding"
    "maproom"
    "query"
    "store"
    "result"
    "context"
    "parse"
    "file"
    "path"
    "token"
    "vector"
    "schema"
)

# Run 100 queries (5 iterations of 20 queries)
LATENCIES=()
for iteration in {1..5}; do
    for query in "${QUERIES[@]}"; do
        START=$(date +%s.%N)

        "$MAPROOM_BIN" search \
            --repo crewchief \
            --worktree baseline-test \
            --query "$query" \
            --k 10 \
            --deduplicate false \
            > /dev/null 2>&1

        END=$(date +%s.%N)
        LATENCY=$(awk "BEGIN {printf \"%.2f\", ($END - $START) * 1000}")
        LATENCIES+=("$LATENCY")
    done
done

# Calculate percentiles (p50, p95, p99)
# Sort latencies
IFS=$'\n' SORTED_LATENCIES=($(sort -n <<<"${LATENCIES[*]}"))
unset IFS

# Calculate indices
TOTAL=${#SORTED_LATENCIES[@]}
P50_IDX=$(awk "BEGIN {print int($TOTAL * 0.50)}")
P95_IDX=$(awk "BEGIN {print int($TOTAL * 0.95)}")
P99_IDX=$(awk "BEGIN {print int($TOTAL * 0.99)}")

# Get values
P50=$(printf "%.2f" "${SORTED_LATENCIES[$P50_IDX]}")
P95=$(printf "%.2f" "${SORTED_LATENCIES[$P95_IDX]}")
P99=$(printf "%.2f" "${SORTED_LATENCIES[$P99_IDX]}")

echo "   p50: ${P50}ms"
echo "   p95: ${P95}ms"
echo "   p99: ${P99}ms"

# === MEMORY USAGE - SINGLE AGENT ===
echo -e "${GREEN}[4/5] Measuring memory usage (single agent)...${NC}"

# Start daemon with stdin held open using a FIFO
FIFO=$(mktemp -u)
mkfifo "$FIFO"
"$MAPROOM_BIN" serve < "$FIFO" > /dev/null 2>&1 &
DAEMON_PID=$!
exec 3>"$FIFO"  # Keep FIFO open for writing
sleep 2  # Let daemon initialize

# Measure RSS (Resident Set Size) in KB, convert to MB
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS: ps shows RSS in KB
    SINGLE_AGENT_RSS=$(ps -o rss= -p "$DAEMON_PID" 2>/dev/null | xargs)
else
    # Linux: ps shows RSS in KB
    SINGLE_AGENT_RSS=$(ps -o rss= -p "$DAEMON_PID" 2>/dev/null | xargs)
fi

# Handle case where ps fails or returns empty
if [ -z "$SINGLE_AGENT_RSS" ] || [ "$SINGLE_AGENT_RSS" = "" ]; then
    echo "   Warning: Could not measure single agent memory, daemon may have exited"
    SINGLE_AGENT_MB=0
else
    SINGLE_AGENT_MB=$(echo "$SINGLE_AGENT_RSS" | awk '{print int($1 / 1024)}')
fi

echo "   Single agent: ${SINGLE_AGENT_MB} MB"

# Kill daemon and cleanup
exec 3>&-  # Close FIFO write end
kill "$DAEMON_PID" 2>/dev/null || true
wait "$DAEMON_PID" 2>/dev/null || true
rm -f "$FIFO"

# === MEMORY USAGE - THREE AGENTS ===
echo -e "${GREEN}[5/5] Measuring memory usage (three agents)...${NC}"

# Start 3 daemons in background with FIFOs to keep them alive
PIDS=()
FIFOS=()
for i in {1..3}; do
    FIFO=$(mktemp -u)
    mkfifo "$FIFO"
    FIFOS+=("$FIFO")
    "$MAPROOM_BIN" serve < "$FIFO" > /dev/null 2>&1 &
    PIDS+=($!)
    # Open FIFO for writing to keep it alive
    eval "exec $((3+i))>\"$FIFO\""
done
sleep 2  # Let daemons initialize

# Measure total RSS across all daemons
TOTAL_RSS=0
for pid in "${PIDS[@]}"; do
    if [[ "$OSTYPE" == "darwin"* ]]; then
        RSS=$(ps -o rss= -p "$pid" 2>/dev/null | xargs)
    else
        RSS=$(ps -o rss= -p "$pid" 2>/dev/null | xargs)
    fi
    # Only add if we got a valid number
    if [ -n "$RSS" ] && [ "$RSS" != "" ]; then
        TOTAL_RSS=$((TOTAL_RSS + RSS))
    fi
done

if [ "$TOTAL_RSS" -eq 0 ]; then
    echo "   Warning: Could not measure three agent memory, daemons may have exited"
    THREE_AGENTS_MB=0
else
    THREE_AGENTS_MB=$(echo "$TOTAL_RSS" | awk '{print int($1 / 1024)}')
fi

echo "   Three agents: ${THREE_AGENTS_MB} MB"

# Kill all daemons and cleanup
for i in {1..3}; do
    eval "exec $((3+i))>&-"  # Close FIFO write ends
done
for pid in "${PIDS[@]}"; do
    kill "$pid" 2>/dev/null || true
done
wait "${PIDS[@]}" 2>/dev/null || true
for fifo in "${FIFOS[@]}"; do
    rm -f "$fifo"
done

# === GENERATE JSON OUTPUT ===
echo ""
echo -e "${BLUE}Generating results JSON...${NC}"

TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

# Create JSON output
cat > "$RESULTS_FILE" <<EOF
{
  "timestamp": "$TIMESTAMP",
  "git_commit": "$GIT_COMMIT",
  "metrics": {
    "search_latency_ms": {
      "p50": $P50,
      "p95": $P95,
      "p99": $P99
    },
    "index_time_seconds": $INDEX_TIME_FORMATTED,
    "memory_usage_mb": {
      "single_agent": $SINGLE_AGENT_MB,
      "three_agents": $THREE_AGENTS_MB
    }
  },
  "environment": {
    "os": "$OS_VERSION",
    "rust_version": "$RUST_VERSION",
    "test_repo_size": "$FILE_COUNT files"
  },
  "notes": [
    "Search latency measured over 100 queries (5 iterations of 20 diverse queries)",
    "Index time measured for full repository scan without embeddings",
    "Memory usage measured using RSS (Resident Set Size) via ps command",
    "Single agent: one daemon process serving requests",
    "Three agents: three concurrent daemon processes simulating multi-agent workload"
  ]
}
EOF

# Cleanup
rm -rf "$(dirname "$TEMP_DB")"

echo -e "${GREEN}✓ Baseline results saved to:${NC}"
echo "  $RESULTS_FILE"
echo ""
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}Summary${NC}"
echo -e "${BLUE}========================================${NC}"
echo "Search latency (p50/p95/p99): ${P50}ms / ${P95}ms / ${P99}ms"
echo "Index time: ${INDEX_TIME_FORMATTED}s"
echo "Memory (1 agent): ${SINGLE_AGENT_MB} MB"
echo "Memory (3 agents): ${THREE_AGENTS_MB} MB"
echo -e "${BLUE}========================================${NC}"

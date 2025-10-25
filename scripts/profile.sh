#!/usr/bin/env bash
#
# Maproom Performance Profiling Script
#
# This script runs performance benchmarks with profiling enabled
# to identify CPU hotspots and bottlenecks.
#
# Usage:
#   ./scripts/profile.sh [BENCHMARK_NAME]
#
# Examples:
#   ./scripts/profile.sh                    # Profile all benchmarks
#   ./scripts/profile.sh indexing           # Profile indexing benchmark
#   ./scripts/profile.sh search_benchmark   # Profile search benchmark
#
# Output:
#   - Criterion benchmark results in target/criterion/
#   - Profiling data (when profiling feature enabled)
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
MAPROOM_DIR="$PROJECT_ROOT/crates/maproom"

echo -e "${BLUE}=== Maproom Performance Profiling ===${NC}"
echo ""

# Check if we're in the right directory
if [ ! -f "$MAPROOM_DIR/Cargo.toml" ]; then
    echo -e "${RED}Error: Cannot find crates/maproom/Cargo.toml${NC}"
    echo "Are you running this from the project root?"
    exit 1
fi

cd "$MAPROOM_DIR"

# Parse arguments
BENCHMARK_NAME="${1:-}"
PROFILING_ENABLED="${PROFILING:-false}"

if [ "$PROFILING_ENABLED" = "true" ]; then
    echo -e "${GREEN}Profiling feature: ENABLED${NC}"
    PROFILE_FLAG="--features profiling"
else
    echo -e "${YELLOW}Profiling feature: DISABLED (set PROFILING=true to enable)${NC}"
    PROFILE_FLAG=""
fi

echo ""

# Run benchmarks
if [ -z "$BENCHMARK_NAME" ]; then
    echo -e "${BLUE}Running all benchmarks...${NC}"
    echo ""
    cargo bench $PROFILE_FLAG
else
    echo -e "${BLUE}Running benchmark: ${BENCHMARK_NAME}${NC}"
    echo ""
    cargo bench --bench "$BENCHMARK_NAME" $PROFILE_FLAG
fi

echo ""
echo -e "${GREEN}=== Profiling Complete ===${NC}"
echo ""
echo -e "${BLUE}Results:${NC}"
echo "  - Benchmark reports: file://$MAPROOM_DIR/target/criterion/report/index.html"
echo ""

if [ "$PROFILING_ENABLED" = "true" ]; then
    echo -e "${BLUE}Profiling data has been collected.${NC}"
    echo "Note: Profiling data analysis requires additional tools (puffin_viewer, etc.)"
else
    echo -e "${YELLOW}Tip: Set PROFILING=true to enable detailed profiling data collection${NC}"
    echo "      PROFILING=true ./scripts/profile.sh"
fi

echo ""
echo -e "${BLUE}Performance Targets:${NC}"
echo "  - Indexing (cold cache): ≥150 files/min"
echo "  - Indexing (warm cache): ≥500 files/min"
echo "  - Search p95: <50ms"
echo "  - Context assembly p95: <120ms"
echo "  - Memory peak: <500MB"
echo ""

# Optional: Generate flamegraph if cargo-flamegraph is installed
if command -v cargo-flamegraph &> /dev/null && [ -n "$BENCHMARK_NAME" ]; then
    echo -e "${BLUE}Generate flamegraph? (y/N)${NC}"
    read -r -n 1 -t 5 response || response="n"
    echo ""

    if [[ "$response" =~ ^[Yy]$ ]]; then
        echo -e "${BLUE}Generating flamegraph for ${BENCHMARK_NAME}...${NC}"
        echo ""
        cargo flamegraph --bench "$BENCHMARK_NAME" -o "target/flamegraph-${BENCHMARK_NAME}.svg"
        echo ""
        echo -e "${GREEN}Flamegraph saved to: target/flamegraph-${BENCHMARK_NAME}.svg${NC}"
    fi
fi

echo -e "${GREEN}Done!${NC}"

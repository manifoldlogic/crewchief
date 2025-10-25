#!/usr/bin/env bash
#
# Load Testing Script for Maproom (PERF_OPT-5002)
#
# This script runs comprehensive load tests and validates all performance targets:
# - Indexing: ≥150 files/min
# - Search p95: <50ms
# - Context p95: <120ms
# - Memory: <500MB
# - Cache hit: >60%
#
# Usage:
#   ./scripts/load-test.sh                    # Run all tests with default targets
#   ./scripts/load-test.sh --quick            # Run quick validation (skip long tests)
#   ./scripts/load-test.sh --benchmark-only   # Run only benchmarks, skip integration tests
#   ./scripts/load-test.sh --targets-only     # Run only target validation
#
# Environment Variables:
#   DATABASE_URL            - PostgreSQL connection string (required)
#   INDEXING_TARGET         - Minimum indexing throughput (default: 150 files/min)
#   SEARCH_P95_TARGET       - Maximum search p95 latency (default: 50ms)
#   CONTEXT_P95_TARGET      - Maximum context p95 latency (default: 120ms)
#   MEMORY_TARGET           - Maximum memory usage (default: 500MB)
#   CACHE_HIT_TARGET        - Minimum cache hit rate (default: 0.6)
#
# Requirements:
#   - PostgreSQL database with DATABASE_URL set
#   - Cargo and Rust toolchain
#   - At least 2GB RAM and 4 CPU cores recommended

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Parse command line arguments
QUICK_MODE=false
BENCHMARK_ONLY=false
TARGETS_ONLY=false

for arg in "$@"; do
    case $arg in
        --quick)
            QUICK_MODE=true
            shift
            ;;
        --benchmark-only)
            BENCHMARK_ONLY=true
            shift
            ;;
        --targets-only)
            TARGETS_ONLY=true
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [--quick] [--benchmark-only] [--targets-only]"
            echo ""
            echo "Options:"
            echo "  --quick           Run quick validation (skip long tests)"
            echo "  --benchmark-only  Run only benchmarks, skip integration tests"
            echo "  --targets-only    Run only target validation"
            echo "  --help, -h        Show this help message"
            exit 0
            ;;
        *)
            echo -e "${RED}Unknown option: $arg${NC}"
            exit 1
            ;;
    esac
done

# Check requirements
echo -e "${BLUE}=== Maproom Load Testing ===${NC}"
echo ""

# Check DATABASE_URL
if [ -z "${DATABASE_URL:-}" ]; then
    echo -e "${RED}ERROR: DATABASE_URL environment variable is not set${NC}"
    echo "Please set DATABASE_URL to your PostgreSQL connection string"
    echo "Example: export DATABASE_URL='postgresql://user:pass@localhost/maproom'"
    exit 1
fi

echo -e "${GREEN}✓ DATABASE_URL is set${NC}"

# Check cargo
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}ERROR: cargo command not found${NC}"
    echo "Please install Rust toolchain: https://rustup.rs/"
    exit 1
fi

echo -e "${GREEN}✓ cargo is available${NC}"
echo ""

# Display performance targets
echo -e "${BLUE}Performance Targets:${NC}"
echo "  Indexing:    ≥${INDEXING_TARGET:-150} files/min"
echo "  Search p95:  <${SEARCH_P95_TARGET:-50}ms"
echo "  Context p95: <${CONTEXT_P95_TARGET:-120}ms"
echo "  Memory:      <${MEMORY_TARGET:-500}MB"
echo "  Cache hit:   >${CACHE_HIT_TARGET:-60}%"
echo ""

# Track overall success
OVERALL_SUCCESS=true

# Function to run a test and track results
run_test() {
    local test_name="$1"
    local test_command="$2"

    echo -e "${YELLOW}Running: $test_name${NC}"

    if eval "$test_command"; then
        echo -e "${GREEN}✓ $test_name passed${NC}"
        echo ""
        return 0
    else
        echo -e "${RED}✗ $test_name failed${NC}"
        echo ""
        OVERALL_SUCCESS=false
        return 1
    fi
}

# Function to run benchmarks
run_benchmarks() {
    echo -e "${BLUE}=== Running Benchmarks ===${NC}"
    echo ""

    if [ "$QUICK_MODE" = true ]; then
        echo "Quick mode: Running abbreviated benchmarks"

        run_test "Indexing Benchmark (quick)" \
            "cargo bench --bench indexing -- --quick 2>&1 | tail -20"

        run_test "Search Benchmark (quick)" \
            "cargo bench --bench search_benchmark -- --quick 2>&1 | tail -20"

        run_test "Context Benchmark (quick)" \
            "cargo bench --bench context_assembly_bench -- --quick 2>&1 | tail -20"
    else
        echo "Full mode: Running complete benchmarks (this may take 10-15 minutes)"

        run_test "Indexing Benchmark" \
            "cargo bench --bench indexing 2>&1 | tail -30"

        run_test "Search Benchmark" \
            "cargo bench --bench search_benchmark 2>&1 | tail -30"

        run_test "Context Assembly Benchmark" \
            "cargo bench --bench context_assembly_bench 2>&1 | tail -30"

        run_test "Memory Optimization Benchmark" \
            "cargo bench --bench memory_optimization_bench 2>&1 | tail -30"

        run_test "Concurrent Operations Benchmark" \
            "cargo bench --bench concurrent_operations_bench 2>&1 | tail -30"
    fi
}

# Function to run integration tests
run_integration_tests() {
    echo -e "${BLUE}=== Running Integration Tests ===${NC}"
    echo ""

    if [ "$QUICK_MODE" = true ]; then
        echo "Quick mode: Skipping long-running load tests"

        run_test "Cache Effectiveness Test" \
            "cargo test --test cache_effectiveness -- --ignored --nocapture 2>&1 | tail -30"

        run_test "Index Usage Test" \
            "cargo test --test index_usage -- --ignored --nocapture 2>&1 | tail -30"
    else
        echo "Full mode: Running all integration tests including load tests"

        run_test "Load Test (sustained)" \
            "cargo test --test load_test test_sustained_load -- --ignored --nocapture --test-threads=1 2>&1 | tail -50"

        run_test "Load Test (burst)" \
            "cargo test --test load_test test_burst_load -- --ignored --nocapture --test-threads=1 2>&1 | tail -50"

        run_test "Cache Effectiveness Test" \
            "cargo test --test cache_effectiveness -- --ignored --nocapture 2>&1 | tail -30"

        run_test "Index Usage Test" \
            "cargo test --test index_usage -- --ignored --nocapture 2>&1 | tail -30"
    fi
}

# Function to validate performance targets
validate_targets() {
    echo -e "${BLUE}=== Validating Performance Targets ===${NC}"
    echo ""

    run_test "Performance Target Validation" \
        "cargo test --test performance_targets test_validate_all_performance_targets -- --ignored --nocapture 2>&1"
}

# Main execution flow
START_TIME=$(date +%s)

if [ "$TARGETS_ONLY" = true ]; then
    # Run only target validation
    validate_targets
elif [ "$BENCHMARK_ONLY" = true ]; then
    # Run only benchmarks
    run_benchmarks
else
    # Run full test suite
    run_benchmarks

    if [ "$QUICK_MODE" = false ]; then
        run_integration_tests
    fi

    validate_targets
fi

END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

# Print summary
echo ""
echo -e "${BLUE}=== Test Summary ===${NC}"
echo ""
echo "Duration: ${DURATION}s"
echo ""

if [ "$OVERALL_SUCCESS" = true ]; then
    echo -e "${GREEN}════════════════════════════════════════${NC}"
    echo -e "${GREEN}  ✓ ALL TESTS PASSED                   ${NC}"
    echo -e "${GREEN}  All performance targets met!         ${NC}"
    echo -e "${GREEN}════════════════════════════════════════${NC}"
    exit 0
else
    echo -e "${RED}════════════════════════════════════════${NC}"
    echo -e "${RED}  ✗ SOME TESTS FAILED                   ${NC}"
    echo -e "${RED}  Performance targets not met           ${NC}"
    echo -e "${RED}════════════════════════════════════════${NC}"
    echo ""
    echo "Next steps:"
    echo "  1. Review failed test output above"
    echo "  2. Check docs/PERFORMANCE_TUNING.md for tuning guidance"
    echo "  3. Profile bottlenecks: cargo flamegraph --bench <benchmark_name>"
    echo "  4. Adjust configuration parameters and retest"
    exit 1
fi

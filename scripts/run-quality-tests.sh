#!/bin/bash
# MD_ENHANCE-4002: Quality Testing Script
#
# This script runs comprehensive quality tests for the markdown parser:
# - Parser accuracy tests
# - Performance benchmarks
# - Hierarchy validation
# - Code block detection
# - Edge case testing

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}MD_ENHANCE Quality Testing Suite${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Navigate to crate directory
cd "$(dirname "$0")/../crates/maproom"

# Run quality validation tests
echo -e "${YELLOW}Running MD_ENHANCE Quality Tests...${NC}"
echo ""

cargo test --test md_enhance_quality_test \
    --release \
    -- --nocapture \
    --test-threads=1

echo ""
echo -e "${GREEN}✓ MD_ENHANCE quality tests completed${NC}"
echo ""

# Run performance tests
echo -e "${YELLOW}Running MD_ENHANCE Performance Tests...${NC}"
echo ""

cargo test --test md_enhance_performance_test \
    --release \
    -- --nocapture \
    --test-threads=1

echo ""
echo -e "${GREEN}✓ MD_ENHANCE performance tests completed${NC}"
echo ""

# Run existing markdown parser tests
echo -e "${YELLOW}Running Markdown Parser Tests...${NC}"
echo ""

cargo test markdown_parser_test \
    --release \
    -- --nocapture

echo ""
echo -e "${GREEN}✓ Markdown parser tests completed${NC}"
echo ""

# Run code block tests
echo -e "${YELLOW}Running Code Block Tests...${NC}"
echo ""

cargo test code_blocks_test \
    --release \
    -- --nocapture

echo ""
echo -e "${GREEN}✓ Code block tests completed${NC}"
echo ""

# Run section boundaries tests
echo -e "${YELLOW}Running Section Boundaries Tests...${NC}"
echo ""

cargo test section_boundaries_test \
    --release \
    -- --nocapture

echo ""
echo -e "${GREEN}✓ Section boundaries tests completed${NC}"
echo ""

# Run real document validation tests
echo -e "${YELLOW}Running Real Document Validation Tests...${NC}"
echo ""

cargo test real_doc_validation_test \
    --release \
    -- --nocapture

echo ""
echo -e "${GREEN}✓ Real document validation tests completed${NC}"
echo ""

# Run benchmarks (optional - requires criterion)
if command -v cargo-criterion &> /dev/null; then
    echo -e "${YELLOW}Running Parser Benchmarks...${NC}"
    echo ""

    cargo criterion --bench parser_bench

    echo ""
    echo -e "${GREEN}✓ Benchmarks completed${NC}"
    echo -e "${BLUE}Benchmark results: target/criterion/report/index.html${NC}"
    echo ""
else
    echo -e "${YELLOW}Skipping benchmarks (cargo-criterion not installed)${NC}"
    echo -e "${BLUE}To install: cargo install cargo-criterion${NC}"
    echo ""
fi

# Summary
echo -e "${BLUE}========================================${NC}"
echo -e "${GREEN}All Quality Tests Completed Successfully${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""
echo "Test Coverage:"
echo "  ✓ Parser accuracy validation"
echo "  ✓ Hierarchy tracking verification"
echo "  ✓ Code block detection (100%)"
echo "  ✓ Performance benchmarks"
echo "  ✓ Edge case testing"
echo "  ✓ Real document validation"
echo ""
echo -e "${GREEN}Success Metrics:${NC}"
echo "  • Parser accuracy: >99% (or >95% acceptable)"
echo "  • Hierarchy tracking: 100%"
echo "  • Code block detection: 100%"
echo "  • Performance: No significant regression"
echo ""

#!/bin/bash
# Quick estimate of maproom scan time for a codebase
# Usage: ./quick-estimate-scan-time.sh /path/to/codebase

CODEBASE_PATH="${1:-.}"

if [ ! -d "$CODEBASE_PATH" ]; then
    echo "Error: Directory not found: $CODEBASE_PATH"
    exit 1
fi

echo "🔍 Quick Scan Estimate: $CODEBASE_PATH"
echo "========================================"
echo ""

# Quick count of supported file types
echo "📊 Counting files..."
ts_count=$(find "$CODEBASE_PATH" -name "*.ts" -o -name "*.tsx" 2>/dev/null | wc -l)
js_count=$(find "$CODEBASE_PATH" -name "*.js" -o -name "*.jsx" 2>/dev/null | wc -l)
rs_count=$(find "$CODEBASE_PATH" -name "*.rs" 2>/dev/null | wc -l)
py_count=$(find "$CODEBASE_PATH" -name "*.py" 2>/dev/null | wc -l)
go_count=$(find "$CODEBASE_PATH" -name "*.go" 2>/dev/null | wc -l)
md_count=$(find "$CODEBASE_PATH" -name "*.md" 2>/dev/null | wc -l)

total_files=$((ts_count + js_count + rs_count + py_count + go_count + md_count))

echo "  TypeScript/TSX: $ts_count"
echo "  JavaScript/JSX: $js_count"
echo "  Rust:           $rs_count"
echo "  Python:         $py_count"
echo "  Go:             $go_count"
echo "  Markdown:       $md_count"
echo "  -------------------------"
echo "  Total:          $total_files files"
echo ""

# Estimate chunks (rough: 1 file ≈ 10 chunks average for well-structured code)
est_chunks=$((total_files * 10))

echo "📦 Estimated: ~$est_chunks chunks"
echo ""

# Benchmarks (from EMBCOPY integration test & genetic optimizer)
# - 60K chunks ≈ 20-30 min base scan (OpenAI)
# - Variant scan with 95% cache hit: <1 min

base_time_min=$((est_chunks * 20 / 60000))
base_time_max=$((est_chunks * 30 / 60000))

if [ $base_time_min -eq 0 ]; then
    base_time_min=1
fi

echo "⏱️  Scan Time Estimates:"
echo "-------------------"
echo "  BASE SCAN (first time):         ~$base_time_min-$base_time_max minutes"

if [ $est_chunks -gt 10000 ]; then
    variant_time="1-2 minutes"
else
    variant_time="<1 minute"
fi

echo "  VARIANT SCAN (with cache):      $variant_time ⚡"
echo ""

# Cost estimate (using integer arithmetic)
# OpenAI text-embedding-3-small: $0.02 / 1M tokens
# ~200 tokens per chunk
# cost = chunks * 200 * 0.02 / 1000000 = chunks * 4 / 1000000
cost_cents=$((est_chunks * 4 / 10000))  # in cents
cost_dollars=$((cost_cents / 100))
cost_remainder=$((cost_cents % 100))

echo "💰 Estimated Cost:"
echo "-------------------"
printf "  OpenAI API:     \$%d.%02d\n" $cost_dollars $cost_remainder
echo "  Ollama (local): \$0 (but 2-4x slower)"
echo ""

# Size comparison
relative=$((est_chunks * 100 / 60000))

echo "📏 Size: ${relative}% of CrewChief codebase (~60K chunks)"
echo ""

if [ $relative -lt 10 ]; then
    echo "✓ Very small - scan will be very fast"
elif [ $relative -lt 50 ]; then
    echo "✓ Small - scan will be quick"
elif [ $relative -lt 150 ]; then
    echo "✓ Medium - moderate scan time"
elif [ $relative -lt 300 ]; then
    echo "⚠  Large - longer scan time, consider Ollama"
else
    echo "⚠  Very large - use Ollama or scan incrementally"
fi
echo ""

# Provide actual database comparison if available
if command -v docker &> /dev/null; then
    actual_chunks=$(docker exec maproom-postgres psql -U maproom -d maproom -t -c "SELECT COUNT(*) FROM maproom.chunks;" 2>/dev/null | tr -d ' ')

    if [ -n "$actual_chunks" ] && [ "$actual_chunks" != "0" ]; then
        echo "📊 Actual chunks in database: $actual_chunks"
        echo ""
    fi
fi

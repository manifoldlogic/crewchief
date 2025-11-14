#!/bin/bash
# Estimate maproom scan time for a codebase
# Usage: ./estimate-scan-time.sh /path/to/codebase

set -e

CODEBASE_PATH="${1:-.}"

if [ ! -d "$CODEBASE_PATH" ]; then
    echo "Error: Directory not found: $CODEBASE_PATH"
    exit 1
fi

echo "🔍 Analyzing codebase: $CODEBASE_PATH"
echo "========================================"
echo ""

# Count files by language
echo "📊 File Statistics:"
echo "-------------------"

# Supported languages in maproom
EXTENSIONS=(
    "ts:TypeScript"
    "tsx:TypeScript"
    "js:JavaScript"
    "jsx:JavaScript"
    "rs:Rust"
    "py:Python"
    "go:Go"
    "md:Markdown"
)

total_files=0
total_lines=0

for ext_lang in "${EXTENSIONS[@]}"; do
    IFS=':' read -r ext lang <<< "$ext_lang"

    count=$(find "$CODEBASE_PATH" -type f -name "*.$ext" 2>/dev/null | wc -l | tr -d ' ')

    if [ "$count" -gt 0 ]; then
        lines=$(find "$CODEBASE_PATH" -type f -name "*.$ext" -exec wc -l {} + 2>/dev/null | tail -1 | awk '{print $1}')
        lines=${lines:-0}

        printf "  %-12s %6d files, %8d lines\n" "$lang:" "$count" "$lines"

        total_files=$((total_files + count))
        total_lines=$((total_lines + lines))
    fi
done

echo "  -------------------------------------"
printf "  %-12s %6d files, %8d lines\n" "TOTAL:" "$total_files" "$total_lines"
echo ""

# Estimate chunks (rough estimate: 1 chunk per 50-100 lines)
est_chunks_min=$((total_lines / 100))
est_chunks_max=$((total_lines / 50))
est_chunks_avg=$(( (est_chunks_min + est_chunks_max) / 2 ))

echo "📦 Estimated Chunks:"
echo "-------------------"
echo "  Conservative: ~$est_chunks_min chunks"
echo "  Average:      ~$est_chunks_avg chunks"
echo "  High:         ~$est_chunks_max chunks"
echo ""

# Known scan benchmarks (from EMBCOPY project integration test)
echo "⏱️  Scan Time Estimates:"
echo "-------------------"
echo ""

# Base scan (first time, with embedding generation)
echo "BASE SCAN (first time with embedding generation):"
# Known: ~60K chunks takes ~20-30 minutes with OpenAI API
# Rate: ~30-50 embeddings/second depending on API latency

embeddings_per_sec_slow=30
embeddings_per_sec_fast=50

base_time_slow=$((est_chunks_avg / embeddings_per_sec_slow / 60))
base_time_fast=$((est_chunks_avg / embeddings_per_sec_fast / 60))

echo "  With OpenAI API (typical):  $base_time_fast - $base_time_slow minutes"
echo "  With Ollama (local):        ~$((base_time_slow * 2)) - $((base_time_slow * 4)) minutes (slower API)"
echo ""

# Variant scan (with embedding copy from cache)
echo "VARIANT SCAN (with embedding inheritance from cache):"
# Known from integration test: 0.37 seconds for 22 chunks with 95.5% cache hit
# Extrapolate: ~0.017 sec per chunk when copying from cache

copy_time_sec=$(echo "scale=1; $est_chunks_avg * 0.017" | bc)
copy_time_min=$(echo "scale=1; $copy_time_sec / 60" | bc)

if (( $(echo "$copy_time_min < 1" | bc -l) )); then
    echo "  With cache (95%+ hit):      <1 minute (${copy_time_sec}s) ⚡"
else
    echo "  With cache (95%+ hit):      ~$copy_time_min minutes ⚡"
fi
echo ""

# Cost estimates
echo "💰 API Cost Estimates (OpenAI):"
echo "-------------------"
# text-embedding-3-small: $0.02 / 1M tokens
# Rough estimate: 200 tokens per chunk average
tokens_per_chunk=200
total_tokens=$((est_chunks_avg * tokens_per_chunk))
cost=$(echo "scale=2; $total_tokens * 0.02 / 1000000" | bc)

echo "  Base scan:        ~\$$cost (${total_tokens} tokens)"
echo "  Variant scan:     ~\$0.01 (only new/modified chunks)"
echo ""

# Comparison with known codebases
echo "📏 Size Comparison:"
echo "-------------------"

# Known reference points
crewchief_chunks=60000
crewchief_lines=150000

relative_size=$(echo "scale=1; $est_chunks_avg * 100 / $crewchief_chunks" | bc)

echo "  vs CrewChief (~60K chunks):  ${relative_size}% of size"

if (( $(echo "$relative_size < 10" | bc -l) )); then
    echo "  Category: Very Small (quick scan)"
elif (( $(echo "$relative_size < 50" | bc -l) )); then
    echo "  Category: Small (fast scan)"
elif (( $(echo "$relative_size < 150" | bc -l) )); then
    echo "  Category: Medium (moderate scan time)"
elif (( $(echo "$relative_size < 300" | bc -l) )); then
    echo "  Category: Large (longer scan time)"
else
    echo "  Category: Very Large (extended scan time)"
fi
echo ""

# Recommendations
echo "💡 Recommendations:"
echo "-------------------"

if (( $(echo "$relative_size > 200" | bc -l) )); then
    echo "  ⚠️  Large codebase - consider:"
    echo "     - Use Ollama locally to avoid API costs"
    echo "     - Scan incrementally by directory"
    echo "     - Enable watch mode for continuous updates"
elif (( $(echo "$relative_size > 100" | bc -l) )); then
    echo "  ✓ Medium-large codebase:"
    echo "     - First scan may take 30-60 minutes"
    echo "     - Subsequent variant scans will be fast (<5 min)"
    echo "     - Consider Ollama if doing frequent full rescans"
else
    echo "  ✓ Good size for fast scanning"
    echo "     - First scan should complete in under 30 minutes"
    echo "     - Variant scans will be near-instant"
fi
echo ""

# Git worktree info if available
if [ -d "$CODEBASE_PATH/.git" ]; then
    echo "🌳 Git Repository Info:"
    echo "-------------------"
    cd "$CODEBASE_PATH"

    branches=$(git branch -a 2>/dev/null | wc -l | tr -d ' ')
    commits=$(git rev-list --all --count 2>/dev/null)

    echo "  Branches: $branches"
    echo "  Commits:  $commits"
    echo ""

    if [ "$branches" -gt 10 ]; then
        echo "  💡 With embedding inheritance:"
        echo "     - Scan main branch first (~$base_time_fast min)"
        echo "     - Each additional branch: <1 min (copies from cache)"
        echo "     - Total for $branches branches: ~$((base_time_fast + branches)) minutes"
    fi
fi

echo ""
echo "✨ Summary:"
echo "========================================"
echo "First scan:    $base_time_fast-$base_time_slow minutes (with embedding generation)"
echo "Variant scans: <1 minute (with embedding inheritance)"
echo "Total cost:    ~\$$cost for full scan"
echo ""

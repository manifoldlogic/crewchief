# Maproom Scan Time Estimation Scripts

These scripts help you estimate how long it will take to scan a codebase with maproom before actually running the scan.

## Quick Estimate (Recommended)

**Fast** (~1-5 seconds) - Counts files and estimates based on file count:

```bash
./scripts/quick-estimate-scan-time.sh /path/to/codebase

# Or from current directory
./scripts/quick-estimate-scan-time.sh .
```

### Example Output

```
🔍 Quick Scan Estimate: /workspace/packages/cli
========================================

📊 Counting files...
  TypeScript/TSX: 3433
  JavaScript/JSX: 6522
  Rust:           514
  Python:         81
  Go:             0
  Markdown:       2630
  -------------------------
  Total:          13180 files

📦 Estimated: ~131800 chunks

⏱️  Scan Time Estimates:
-------------------
  BASE SCAN (first time):         ~43-65 minutes
  VARIANT SCAN (with cache):      1-2 minutes ⚡

💰 Estimated Cost:
-------------------
  OpenAI API:     $0.52
  Ollama (local): $0 (but 2-4x slower)

📏 Size: 219% of CrewChief codebase (~60K chunks)

⚠  Large - longer scan time, consider Ollama

📊 Actual chunks in database: 737070
```

## Detailed Estimate

**Slower** (~30-60 seconds for large codebases) - Counts lines for more accuracy:

```bash
./scripts/estimate-scan-time.sh /path/to/codebase
```

Provides:

- Line count analysis by language
- More precise chunk estimates
- Detailed breakdown of time/cost by scenario
- Git repository analysis (branches, commits)

## Understanding the Output

### Scan Types

**BASE SCAN** (first time):

- Generates embeddings for ALL chunks
- Slower: ~30-50 embeddings/second (OpenAI API)
- Cost: Based on total tokens (~200 tokens/chunk)
- Time: Proportional to chunk count

**VARIANT SCAN** (with embedding inheritance):

- Copies embeddings from cache for unchanged code
- FAST: 200-500× faster than base scan
- Cost: Only pays for new/modified chunks
- Time: Seconds to a few minutes (regardless of size)

### Size Categories

- **<10%** of CrewChief (~6K chunks): Very small, <10 min
- **10-50%** (6-30K chunks): Small, 10-30 min
- **50-150%** (30-90K chunks): Medium, 30-90 min
- **150-300%** (90-180K chunks): Large, 90-180 min
- **>300%** (>180K chunks): Very large, >180 min

### Cost Estimates

Based on OpenAI's `text-embedding-3-small` pricing ($0.02 / 1M tokens):

- ~200 tokens per chunk average
- Example: 60K chunks = ~12M tokens = ~$0.24

## Benchmarks (Real Data)

From EMBCOPY project integration tests and genetic optimizer runs:

| Metric            | Value     | Source                      |
| ----------------- | --------- | --------------------------- |
| Chunks per scan   | ~60K      | CrewChief codebase          |
| Base scan time    | 20-30 min | OpenAI API                  |
| Variant scan time | <1 min    | With 95%+ cache hit         |
| Cache hit rate    | 95.5%     | Variant with 1 file changed |
| Speedup           | 200-500×  | Embedding inheritance       |

## Tips

### For Large Codebases (>100K chunks)

1. **Use Ollama** (local) to avoid API costs:

   ```bash
   export MAPROOM_EMBEDDING_PROVIDER=ollama
   export MAPROOM_EMBEDDING_MODEL=nomic-embed-text
   ```

2. **Scan incrementally** by directory:

   ```bash
   crewchief-maproom scan --repo myrepo --worktree main --path /workspace/src
   crewchief-maproom scan --repo myrepo --worktree main --path /workspace/tests
   ```

3. **Use watch mode** for continuous updates:
   ```bash
   crewchief-maproom watch --repo myrepo --worktree main --path /workspace
   ```

### For Multiple Branches

With embedding inheritance, scanning multiple branches is fast:

1. Scan main branch first (~30-60 min for medium codebase)
2. Scan feature branches (<1-2 min each - copies from cache!)
3. Total for 10 branches: ~35-70 min (not 300-600 min!)

## Algorithm Details

### Estimation Formula

```
chunks ≈ total_files × 10

# Base scan time (minutes)
time_min = chunks × 20 / 60000
time_max = chunks × 30 / 60000

# Cost (USD)
cost = chunks × 200 × 0.02 / 1000000
```

### Why This Works

- Tree-sitter extracts ~5-15 chunks per file (functions, classes, etc.)
- Average: ~10 chunks per well-structured code file
- Embedding generation rate: ~30-50/sec (OpenAI API)
- 60K chunks ≈ 20-30 minutes empirically validated

## Supported Languages

- TypeScript/TSX (.ts, .tsx)
- JavaScript/JSX (.js, .jsx)
- Rust (.rs)
- Python (.py)
- Go (.go)
- Markdown (.md)

More languages coming soon!

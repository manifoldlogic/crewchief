# Migrating to mxbai-embed-large

**Last Updated**: December 2025

This guide helps you understand the transition from `nomic-embed-text` (768-dim) to `mxbai-embed-large` (1024-dim) as the default embedding model in Maproom.

---

## Executive Summary

### What Changed

Maproom now uses **mxbai-embed-large** (1024 dimensions) as the default embedding model instead of **nomic-embed-text** (768 dimensions).

### Why We Changed It

1. **Better Quality**: Higher-dimensional embeddings provide more nuanced semantic understanding
2. **No Crashes**: mxbai-embed-large handles special characters correctly without sanitization
3. **Stability**: Eliminates GGML tokenization bugs that caused nomic-embed-text to crash on certain inputs

### Who Is Affected

| User Type | Impact | Action Required |
|-----------|--------|-----------------|
| **Zero-config users** | Automatic upgrade | None - works automatically |
| **Explicit config users** | No change | None - your config is preserved |
| **New users** | Better defaults | None - just install and use |

### Backward Compatibility

**Fully backward compatible.** Existing embeddings continue to work. Mixed-dimension search is supported automatically.

---

## For Zero-Config Users

If you've been using Maproom without setting `MAPROOM_EMBEDDING_MODEL` or `MAPROOM_EMBEDDING_DIMENSION`, here's what happens:

### Automatic Upgrade Process

1. **First scan after update**: Maproom will use mxbai-embed-large automatically
2. **New embeddings**: Stored in `vec_code_1024` table (1024 dimensions)
3. **Old embeddings**: Remain in `vec_code_768` table (still searchable)
4. **Search**: Works across both dimension tables seamlessly

### No Action Required

You don't need to do anything. Your next `scan` or `upsert` operation will:

- Automatically pull `mxbai-embed-large` if not present (669 MB download)
- Generate 1024-dimensional embeddings for new/changed files
- Continue using your existing 768-dim embeddings for unchanged files

### Mixed-Dimension Search

Maproom automatically searches across both dimension tables:

```
Query → Generate query embedding (1024-dim)
      → Search vec_code_1024 (new embeddings)
      → Search vec_code_768 (old embeddings)
      → Merge and rank results
```

This means you can gradually migrate without re-indexing everything at once.

---

## For Explicit Config Users

If you want to continue using nomic-embed-text, you can explicitly configure it.

### CLI Users

Set these environment variables before running Maproom:

```bash
# Add to ~/.bashrc, ~/.zshrc, or equivalent
export MAPROOM_EMBEDDING_MODEL=nomic-embed-text
export MAPROOM_EMBEDDING_DIMENSION=768
```

### VSCode Extension Users

The VSCode extension respects the same environment variables. Set them in your shell profile, or create a `.env` file in your project root:

```bash
# .env in your project root
MAPROOM_EMBEDDING_MODEL=nomic-embed-text
MAPROOM_EMBEDDING_DIMENSION=768
```

### MCP Server Users

Configure environment variables in your MCP client settings:

```json
{
  "servers": {
    "maproom": {
      "command": "npx",
      "args": ["-y", "@crewchief/maproom-mcp"],
      "env": {
        "MAPROOM_DATABASE_URL": "sqlite:///Users/you/.maproom/maproom.db",
        "MAPROOM_EMBEDDING_PROVIDER": "ollama",
        "MAPROOM_EMBEDDING_MODEL": "nomic-embed-text",
        "MAPROOM_EMBEDDING_DIMENSION": "768"
      }
    }
  }
}
```

### Verify Your Configuration

After setting environment variables, verify they're applied:

```bash
# Check environment variables
echo $MAPROOM_EMBEDDING_MODEL
# Expected: nomic-embed-text

echo $MAPROOM_EMBEDDING_DIMENSION
# Expected: 768
```

---

## Re-embedding Existing Content

You can optionally re-embed your entire codebase with the new model for improved search quality.

### When to Re-embed

**Consider re-embedding if:**
- You want the best possible search quality
- You're experiencing search result issues with old embeddings
- You have disk space and time for a full re-index

**Skip re-embedding if:**
- Current search quality is sufficient
- You're working with very large codebases (gradual migration is fine)
- Storage is a concern

### CLI Commands

```bash
# Option 1: Re-scan entire repository (generates new embeddings)
crewchief-maproom scan --path /path/to/your/repo --repo myrepo --worktree main

# Option 2: Force regenerate embeddings for existing chunks
crewchief-maproom generate-embeddings --repo myrepo --force
```

### VSCode Extension

1. Open Command Palette (`Cmd+Shift+P` / `Ctrl+Shift+P`)
2. Run `Maproom: Restart Watchers`
3. The extension will re-index changed files automatically

For a full re-index, delete the database and scan again:

```bash
rm ~/.maproom/maproom.db
# Then trigger a scan via VSCode or CLI
```

### MCP Server

MCP server automatically re-indexes when you call the `scan` tool:

```json
{
  "tool": "scan",
  "arguments": {
    "path": "/path/to/your/repo"
  }
}
```

---

## Storage Impact

### Embedding Size Comparison

| Metric | nomic-embed-text | mxbai-embed-large | Difference |
|--------|------------------|-------------------|------------|
| **Dimensions** | 768 | 1024 | +256 dims |
| **Bytes per embedding** | 3,072 bytes | 4,096 bytes | +1,024 bytes |
| **Increase** | - | - | **+33%** |

### Storage Calculator

Use this formula to estimate storage impact:

```
Additional storage = (number of embeddings) × 1,024 bytes
```

**Examples:**

| Codebase Size | Chunks | Old Storage | New Storage | Increase |
|---------------|--------|-------------|-------------|----------|
| Small | 1,000 | 3 MB | 4 MB | +1 MB |
| Medium | 10,000 | 30 MB | 40 MB | +10 MB |
| Large | 100,000 | 300 MB | 400 MB | +100 MB |
| Very Large | 1,000,000 | 3 GB | 4 GB | +1 GB |

### Model Download Size

| Model | Download Size | One-time |
|-------|---------------|----------|
| nomic-embed-text | 274 MB | Yes |
| mxbai-embed-large | 669 MB | Yes |
| **Difference** | **+395 MB** | - |

### Cost-Benefit Analysis

The 33% storage increase is typically worthwhile because:

1. **Storage is cheap**: ~$0.02/GB for cloud, ~$0.10/GB for SSD
2. **Quality matters**: Better search results save developer time
3. **Disk space is abundant**: Most systems have plenty of headroom
4. **Gradual migration**: Old embeddings stay in place, new ones use new dimensions

---

## Troubleshooting

### "Model 'mxbai-embed-large' not found"

**Cause**: The model hasn't been downloaded yet.

**Solution**:
```bash
ollama pull mxbai-embed-large
```

### "Existing embeddings not searchable"

**Cause**: You may be expecting old embeddings to be in the new table.

**Solution**: Mixed-dimension search works automatically. Old embeddings in `vec_code_768` are still searched. If results seem poor, try re-indexing.

### "Concerned about storage increase"

**Context**: 33% more storage sounds significant.

**Reality**: For most codebases, this is negligible:
- 10K chunks = ~10 MB extra
- 100K chunks = ~100 MB extra

Storage is cheap; search quality improvements are valuable.

### "Performance concerns with larger embeddings"

**Impact**: Minimal.

**Details**:
- Embedding generation: ~10-15% slower (larger model)
- Vector search: ~5-10% slower (more dimensions)
- Overall: Imperceptible in practice

### "I want to switch back to nomic-embed-text"

**Solution**: Set explicit environment variables:
```bash
export MAPROOM_EMBEDDING_MODEL=nomic-embed-text
export MAPROOM_EMBEDDING_DIMENSION=768
```

### "Do I need to re-embed all my content?"

**Answer**: No, it's optional.

**Details**:
- Old embeddings continue working
- New/changed files get new embeddings automatically
- Re-embed only if you want improved quality everywhere

### "Are there breaking changes?"

**Answer**: No.

**Details**:
- API unchanged
- CLI unchanged
- Configuration unchanged
- Old embeddings still work

### "Is mixed-dimension search supported?"

**Answer**: Yes, fully tested.

**Details**:
- Search automatically queries both `vec_code_768` and `vec_code_1024`
- Results are merged and ranked together
- No configuration required

---

## Model Comparison

| Feature | nomic-embed-text | mxbai-embed-large |
|---------|------------------|-------------------|
| **Status** | Legacy | **Default** |
| **Dimensions** | 768 | 1024 |
| **Quality Score** | 8.5/10 | **9.0/10** |
| **Special Character Handling** | Crashes (needs sanitization) | **Works correctly** |
| **GGML Tokenization** | Buggy | **Stable** |
| **Model Size** | 274 MB | 669 MB |
| **Storage per Embedding** | 3,072 bytes | 4,096 bytes |
| **Generation Speed** | Faster | Slightly slower |
| **Throughput** | ~8,000 tokens/sec | ~6,780 tokens/sec |

### When to Use nomic-embed-text

- Very storage-constrained environments
- Existing large indexes you don't want to migrate
- Faster embedding generation is critical

### When to Use mxbai-embed-large (Recommended)

- New projects
- Quality-focused applications
- Code containing special characters
- Zero-config deployments

---

## Additional Resources

- [Ollama Setup Guide](../providers/ollama-setup.md) - Detailed Ollama configuration
- [Provider Comparison](../providers/comparison.md) - Compare all embedding providers
- [Performance Tuning](performance-tuning.md) - Optimize embedding generation

---

## Summary

| Question | Answer |
|----------|--------|
| **Is this a breaking change?** | No |
| **Do I need to do anything?** | No (automatic for zero-config users) |
| **Will my old embeddings work?** | Yes |
| **Is mixed-dimension search supported?** | Yes |
| **Should I re-embed everything?** | Optional (for quality improvement) |
| **Can I keep using nomic-embed-text?** | Yes (set explicit env vars) |

**Bottom line**: The migration is automatic for most users. Your existing embeddings continue working, and new embeddings benefit from improved quality. No action required unless you want to customize.

---

**Last Updated**: December 2025

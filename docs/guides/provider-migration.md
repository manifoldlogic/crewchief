# Embedding Provider Migration Guide

This guide helps existing Maproom users migrate between embedding providers or run multiple providers simultaneously.

## Overview

Maproom's multi-provider embedding system allows you to switch between OpenAI, Ollama, and Google Vertex AI embeddings **without losing your existing data**. This guide covers all migration scenarios, from gradual transitions to complete re-indexing.

## What Happens to Existing Embeddings?

### Short Answer

**Your existing embeddings are preserved.** New embeddings are stored in separate database columns based on dimensionality.

### Technical Details

Maproom uses dimension-specific database columns to store embeddings from different providers:

- **1536-dimensional embeddings** (OpenAI `text-embedding-3-small`) → `code_embedding`, `doc_embedding`
- **768-dimensional embeddings** (Ollama, Google Vertex AI) → `code_embedding_ollama`, `doc_embedding_ollama`

**Search behavior**: Search queries automatically use `COALESCE` to prefer 768-dimensional embeddings over 1536-dimensional when both exist. This ensures optimal search performance while preserving backward compatibility.

**Why this matters**: You can switch providers without data loss, run multiple providers simultaneously for testing, and migrate gradually over time.

For technical details, see [Column Selection Logic (MPEMBED-4001)](../../.crewchief/archive/projects/MPEMBED_multi-provider-embeddings/planning-multi-provider-embeddings/phase-4-unified-pipeline.md#mpembed-4001-column-selection-logic).

---

## Migration Scenarios

### Scenario 1: Switch from OpenAI to Ollama (Preserve Existing)

**Goal**: Keep OpenAI embeddings, add Ollama for new/updated chunks

**When to use**: Cost optimization, privacy requirements, or testing local embeddings without re-indexing

**Steps**:

```bash
# 1. Install and configure Ollama
ollama pull nomic-embed-text

# 2. Verify Ollama is running
curl http://localhost:11434/api/tags
# Should return JSON with available models

# 3. Set provider (auto-detected if Ollama is running)
export MAPROOM_EMBEDDING_PROVIDER=ollama

# 4. Scan for missing embeddings
crewchief maproom scan --generate-embeddings

# Result: Only chunks without Ollama embeddings are processed
```

**What happens**:
- Existing chunks with OpenAI embeddings: Continue using OpenAI (1536-dim)
- New chunks: Get Ollama embeddings (768-dim)
- Updated chunks: Get new Ollama embeddings (768-dim)
- Search works seamlessly across both embedding types

**Cost**: $0 (Ollama is free, no API costs)

**Time estimate**: ~5 minutes for initial setup + ongoing incremental updates

**Advantages**:
- Zero downtime
- No re-indexing cost
- Existing search quality maintained
- Natural migration over time

**Disadvantages**:
- Mixed embedding types may have slight quality differences
- Database stores both embedding types (higher storage)

---

### Scenario 2: Full Re-index with Ollama

**Goal**: Replace all OpenAI embeddings with Ollama

**When to use**: Complete cost elimination, full local control, or standardizing on single provider

**Steps**:

```bash
# 1. Back up database (RECOMMENDED)
pg_dump -h localhost -U postgres crewchief > /tmp/crewchief-backup-$(date +%Y%m%d).sql

# 2. Clear existing Ollama embeddings (prepare for clean re-index)
psql -h localhost -U postgres crewchief -c "
  UPDATE chunks
  SET code_embedding_ollama = NULL,
      doc_embedding_ollama = NULL;
"

# 3. Configure Ollama provider
export MAPROOM_EMBEDDING_PROVIDER=ollama

# 4. Re-index with Ollama
crewchief maproom scan --generate-embeddings

# 5. Verify migration (see "Verify Your Migration" section below)

# 6. Optional: Clear OpenAI embeddings to reclaim disk space
psql -h localhost -U postgres crewchief -c "
  UPDATE chunks
  SET code_embedding = NULL,
      doc_embedding = NULL;
"

# 7. Reclaim disk space (IMPORTANT: requires exclusive lock)
psql -h localhost -U postgres crewchief -c "VACUUM FULL chunks;"
```

**Time estimate**: ~30 minutes for 100K chunks (local GPU), ~2-3 hours (CPU only)

**Cost**: $0 (Ollama is free)

**Storage savings**: ~50% reduction (768-dim vs 1536-dim)

**Advantages**:
- Consistent embedding quality across all chunks
- Lower storage requirements
- No API costs ever
- Complete data privacy

**Disadvantages**:
- Requires significant re-indexing time
- Temporary loss of search functionality during re-index
- One-time compute cost (electricity, GPU utilization)

---

### Scenario 3: Gradual Migration (Recommended)

**Goal**: Migrate incrementally without re-indexing, preserve existing embeddings

**When to use**: Production environments, minimizing risk, or testing new provider before commitment

**Steps**:

```bash
# 1. Switch to Ollama for new embeddings
export MAPROOM_EMBEDDING_PROVIDER=ollama

# 2. Continue normal development workflow
# - OpenAI embeddings continue working for existing code
# - New/updated chunks automatically get Ollama embeddings

# 3. Monitor migration progress
psql -h localhost -U postgres crewchief -c "
  SELECT
    COUNT(*) FILTER (WHERE code_embedding IS NOT NULL) AS openai_count,
    COUNT(*) FILTER (WHERE code_embedding_ollama IS NOT NULL) AS ollama_count,
    COUNT(*) FILTER (WHERE code_embedding IS NOT NULL AND code_embedding_ollama IS NOT NULL) AS both_count,
    ROUND(100.0 * COUNT(*) FILTER (WHERE code_embedding_ollama IS NOT NULL) / COUNT(*), 2) AS ollama_percentage
  FROM chunks;
"

# Example output:
#  openai_count | ollama_count | both_count | ollama_percentage
# --------------+--------------+------------+-------------------
#         15234 |         3456 |        456 |             22.68
```

**Timeline**: Natural migration over weeks/months as code changes

**Cost**: $0 (no re-indexing, only new embeddings)

**Advantages**:
- **Zero downtime** - Search works throughout migration
- **No upfront cost** - No re-indexing required
- **Risk-free** - Can revert at any time
- **Natural transition** - Follows your development cycle
- **A/B testing** - Compare search quality over time

**Disadvantages**:
- Extended timeline (depends on code change frequency)
- Mixed embedding types during transition
- Higher storage temporarily (both embedding types)

**Best practices**:
1. Monitor search quality metrics before/after switch
2. Keep OpenAI embeddings for 30-90 days for comparison
3. Document migration start date and percentage targets
4. Set up weekly progress checks
5. Plan full re-index after reaching 80-90% Ollama coverage

---

### Scenario 4: Run Both Providers Simultaneously

**Goal**: Generate both 768-dim and 1536-dim embeddings for all chunks

**When to use**: A/B testing, migration validation, experimentation, or regulatory requirements

**Steps**:

```bash
# 1. Index with OpenAI (if not already done)
export MAPROOM_EMBEDDING_PROVIDER=openai
crewchief maproom scan --generate-embeddings

# 2. Index with Ollama (adds to existing, doesn't replace)
export MAPROOM_EMBEDDING_PROVIDER=ollama
crewchief maproom scan --generate-embeddings

# 3. Verify both embeddings exist
psql -h localhost -U postgres crewchief -c "
  SELECT
    COUNT(*) AS total_chunks,
    COUNT(*) FILTER (WHERE code_embedding IS NOT NULL AND code_embedding_ollama IS NOT NULL) AS both_embeddings,
    ROUND(100.0 * COUNT(*) FILTER (WHERE code_embedding IS NOT NULL AND code_embedding_ollama IS NOT NULL) / COUNT(*), 2) AS percentage
  FROM chunks;
"

# Result: Each chunk has both embeddings
```

**Storage impact**: ~2x embedding storage (~9KB per chunk vs ~3-6KB)

**Search behavior**: Automatically prefers Ollama (768-dim) when both present

**Use cases**:
- **A/B testing**: Compare search quality between providers
- **Migration validation**: Verify Ollama quality before deleting OpenAI
- **Regulatory compliance**: Maintain multiple embedding sources
- **Benchmarking**: Measure provider-specific performance

**Cost**: OpenAI re-indexing cost (~$50 for 100K chunks) + Ollama compute

**Advantages**:
- Direct quality comparison
- Zero risk migration validation
- Instant rollback capability
- Provider-agnostic search

**Disadvantages**:
- Double storage requirements
- Higher indexing time and cost
- Complexity in monitoring and management

---

### Scenario 5: Switch from Ollama to Cloud Provider

**Goal**: Migrate from local Ollama embeddings to OpenAI or Google Vertex AI

**When to use**: Production scale-up, team expansion, or requiring SLA-backed uptime

**Steps**:

```bash
# 1. Choose cloud provider
export MAPROOM_EMBEDDING_PROVIDER=openai  # or "google"

# 2. Configure credentials
export OPENAI_API_KEY="sk-..."  # for OpenAI
# OR
export GOOGLE_APPLICATION_CREDENTIALS="/path/to/service-account.json"  # for Google

# 3. Test configuration
crewchief maproom scan --dry-run
# Verify provider and credentials are detected

# 4. Strategy A: Gradual migration (recommended)
# Simply let new/updated chunks use cloud provider naturally

# 4. Strategy B: Full re-index (for consistency)
crewchief maproom scan --generate-embeddings

# 5. Monitor costs
# OpenAI: Check usage at https://platform.openai.com/usage
# Google: Check billing at https://console.cloud.google.com/billing
```

**Key differences**:

| Aspect | Ollama → OpenAI | Ollama → Google |
|--------|-----------------|-----------------|
| **Embedding dimensions** | 768 → 1536 | 768 → 768 |
| **Column storage** | New columns | Same columns |
| **Re-indexing required** | Yes (different dimensions) | No (same dimensions) |
| **Search behavior change** | Significant | Minimal |
| **Cost** | ~$50 per 100K chunks | ~$250 per 100K chunks |

**Advantages**:
- Production-grade SLA and uptime
- No local hardware requirements
- Automatic scaling
- Professional support

**Disadvantages**:
- Ongoing API costs
- Network dependency
- Data leaves local environment

---

## Verify Your Migration

### Check Embedding Distribution

Use this SQL query to understand your current embedding state:

```sql
SELECT
  CASE
    WHEN code_embedding IS NOT NULL AND code_embedding_ollama IS NULL THEN 'OpenAI only'
    WHEN code_embedding IS NULL AND code_embedding_ollama IS NOT NULL THEN 'Ollama/Google only'
    WHEN code_embedding IS NOT NULL AND code_embedding_ollama IS NOT NULL THEN 'Both providers'
    ELSE 'No embeddings'
  END AS embedding_status,
  COUNT(*) AS chunk_count,
  ROUND(100.0 * COUNT(*) / SUM(COUNT(*)) OVER (), 2) AS percentage
FROM chunks
GROUP BY embedding_status
ORDER BY chunk_count DESC;
```

**Example output**:

```
   embedding_status   | chunk_count | percentage
----------------------+-------------+------------
 OpenAI only          |       12000 |      60.00
 Ollama/Google only   |        6000 |      30.00
 Both providers       |        1500 |       7.50
 No embeddings        |         500 |       2.50
```

### Verify Search Quality

Test search with your most important queries:

```bash
# Save current results as baseline
crewchief maproom search "authentication flow" > /tmp/search-baseline.txt

# After migration, compare
crewchief maproom search "authentication flow" > /tmp/search-migrated.txt
diff /tmp/search-baseline.txt /tmp/search-migrated.txt

# Test multiple query types
crewchief maproom search "database query" --mode hybrid --k 20
crewchief maproom search "error handling" --mode hybrid --k 20
crewchief maproom search "API endpoints" --mode hybrid --k 20
```

**Quality metrics to track**:
- **Relevance**: Are top results still relevant?
- **Recall**: Do all expected results appear?
- **Ranking**: Is ordering still intuitive?
- **Coverage**: Are all code areas represented?

### Check Database Size

Monitor storage impact of your migration:

```sql
SELECT
  pg_size_pretty(pg_total_relation_size('chunks')) AS total_size,
  pg_size_pretty(pg_relation_size('chunks')) AS table_size,
  pg_size_pretty(pg_total_relation_size('chunks') - pg_relation_size('chunks')) AS index_size;
```

**Example output**:

```
 total_size | table_size | index_size
------------+------------+------------
 2456 MB    | 1892 MB    | 564 MB
```

**Typical sizes**:
- OpenAI only: ~6KB per chunk
- Ollama/Google only: ~3KB per chunk
- Both: ~9KB per chunk
- 100K chunks: ~300-900MB total

---

## Rollback Strategy

### If Migration Fails or Quality Degrades

#### Rollback from Database Backup

```bash
# 1. Stop all Maproom processes
pkill -f crewchief-maproom

# 2. Restore from backup
pg_restore -h localhost -U postgres -d crewchief /tmp/crewchief-backup-20251029.sql
# OR for SQL dump:
psql -h localhost -U postgres crewchief < /tmp/crewchief-backup-20251029.sql

# 3. Verify restoration
psql -h localhost -U postgres crewchief -c "SELECT COUNT(*) FROM chunks;"

# 4. Switch back to original provider
export MAPROOM_EMBEDDING_PROVIDER=openai

# 5. Restart Maproom
crewchief maproom scan
```

#### Rollback by Clearing New Embeddings

If you haven't deleted old embeddings:

```bash
# Clear new embeddings, keep original
psql -h localhost -U postgres crewchief -c "
  UPDATE chunks
  SET code_embedding_ollama = NULL,
      doc_embedding_ollama = NULL;
"

# Switch back to OpenAI
export MAPROOM_EMBEDDING_PROVIDER=openai

# Verify search works
crewchief maproom search "test query"
```

#### Rollback from Gradual Migration

```bash
# Simply switch back - existing embeddings still work
export MAPROOM_EMBEDDING_PROVIDER=openai

# No data loss - OpenAI embeddings were preserved
crewchief maproom search "test query"
```

---

## Cost Comparison

### Migration Scenarios Cost Analysis

| Scenario | OpenAI Cost | Ollama Cost | Google Cost | Time |
|----------|-------------|-------------|-------------|------|
| **Keep existing** | $0 | $0 | $0 | 0 min |
| **Add Ollama (incremental)** | $0 | $0 (electricity) | - | ~5 min |
| **Full re-index (100K chunks)** | ~$50 | $0 (electricity) | ~$250 | 30 min - 2 hrs |
| **Run both simultaneously** | ~$50 | $0 (electricity) | - | 1-2 hrs |
| **Gradual migration** | $0 (existing) | $0 | - | Weeks/months |

### Detailed Cost Breakdown

**OpenAI (text-embedding-3-small)**:
- Price: ~$0.00002 per 1,000 tokens (~$0.00003 per 1,000 characters)
- 100K chunks (~50 characters avg): ~$1.50
- Full codebase (1M lines): ~$50

**Google Vertex AI (text-embedding-gecko@003)**:
- Price: ~$0.00025 per 1,000 characters
- 100K chunks (~50 characters avg): ~$12.50
- Full codebase (1M lines): ~$250

**Ollama**:
- Price: Free (electricity + hardware only)
- Hardware: ~$0.10-$0.50 per 100K chunks (GPU electricity)
- One-time hardware cost: $500-$2000 (GPU recommended)

### Cost Optimization Tips

1. **Use Ollama for development**: Zero ongoing costs
2. **Gradual migration**: Avoid re-indexing costs
3. **Selective re-indexing**: Only re-embed changed files
4. **Batch operations**: Maproom batches automatically
5. **Monitor usage**: Set up billing alerts

---

## FAQ

### General Migration Questions

**Q: Will search quality change when I switch providers?**

A: Embedding quality is similar but not identical. Most users report comparable or better results with Ollama (`nomic-embed-text`) compared to OpenAI. Google Vertex AI also provides high-quality embeddings. We recommend:

1. Run A/B tests on your specific codebase
2. Compare search results for your most important queries
3. Monitor user feedback during gradual migration
4. Keep old embeddings for 30-90 days for fallback

**Q: Can I delete OpenAI embeddings after migrating to Ollama?**

A: Yes, but we recommend keeping them for 30-90 days to compare search quality:

```sql
-- After confirming Ollama quality is satisfactory
UPDATE chunks SET code_embedding = NULL, doc_embedding = NULL;

-- Reclaim disk space (requires exclusive lock)
VACUUM FULL chunks;
```

**Q: How much disk space do embeddings use?**

A:
- **OpenAI (1536-dim)**: ~6KB per chunk
- **Ollama/Google (768-dim)**: ~3KB per chunk
- **Both simultaneously**: ~9KB per chunk
- **100K chunks**: 300MB (Ollama) to 900MB (both)

**Q: What if I need to go back to OpenAI later?**

A: Simply set `MAPROOM_EMBEDDING_PROVIDER=openai` and scan. Your OpenAI embeddings are preserved unless explicitly deleted. No data loss occurs during provider switches.

**Q: Do I need to re-index when switching between Ollama and Google?**

A: **No!** Both use 768 dimensions and share the same database columns. Just switch the provider:

```bash
# From Ollama to Google (or vice versa)
export MAPROOM_EMBEDDING_PROVIDER=google
export GOOGLE_APPLICATION_CREDENTIALS="/path/to/service-account.json"

# Embeddings stored in same columns, seamless transition
crewchief maproom scan
```

---

### Technical Questions

**Q: How does COALESCE preference work?**

A: Search queries use this SQL pattern:

```sql
COALESCE(code_embedding_ollama, code_embedding) AS embedding_vector
```

This means:
1. **First choice**: Use 768-dim embedding (Ollama/Google) if available
2. **Fallback**: Use 1536-dim embedding (OpenAI) if 768-dim doesn't exist
3. **Automatic**: No configuration needed

**Q: Why prefer 768-dim over 1536-dim?**

A: Several reasons:
- **Performance**: Smaller vectors = faster search (less computation)
- **Storage**: 50% less disk space
- **Quality**: 768-dim embeddings are sufficient for most code search tasks
- **Cost**: Cheaper to generate (especially with Ollama)

**Q: Can I force using OpenAI embeddings even if Ollama exists?**

A: Currently not supported via environment variable, but you can modify the query manually or clear Ollama embeddings for specific chunks.

**Q: How do I force re-embedding of specific files?**

A:

```bash
crewchief maproom upsert \
  --repo myrepo \
  --worktree main \
  --root /path/to/repo \
  --commit HEAD \
  --paths src/auth.ts src/db.ts src/api/routes.ts
```

This will re-parse and re-embed only the specified files.

---

### Operational Questions

**Q: Can I run migration during business hours?**

A: **Gradual migration**: Yes, zero impact
**Full re-index**: Not recommended (indexing load, temporary degraded search)

**Q: How long does re-indexing take?**

A: Depends on:
- **Chunk count**: 100K chunks = ~30 min (Ollama GPU) to 2-3 hrs (CPU)
- **Provider**: Ollama (local) is faster than cloud APIs
- **Hardware**: GPU significantly faster than CPU
- **Network**: Cloud providers limited by network latency

**Q: What happens if re-indexing fails mid-way?**

A: **Safe**: Maproom tracks progress. Simply restart the scan:

```bash
crewchief maproom scan --generate-embeddings
```

Only chunks without embeddings will be processed. Partial progress is preserved.

**Q: How do I monitor migration progress in production?**

A:

```bash
# Set up monitoring query
watch -n 60 'psql -h localhost -U postgres crewchief -c "
  SELECT
    COUNT(*) FILTER (WHERE code_embedding_ollama IS NOT NULL) AS ollama_count,
    ROUND(100.0 * COUNT(*) FILTER (WHERE code_embedding_ollama IS NOT NULL) / COUNT(*), 2) AS percentage
  FROM chunks;
"'
```

Or integrate with your monitoring stack (Prometheus, Datadog, etc.).

**Q: Should I back up before migration?**

A: **Yes, always** for full re-indexing scenarios:

```bash
# Full backup
pg_dump -h localhost -U postgres crewchief > /tmp/crewchief-backup-$(date +%Y%m%d).sql

# Just embeddings (faster restore)
pg_dump -h localhost -U postgres -t chunks crewchief > /tmp/chunks-backup.sql
```

Not required for gradual migration (no data loss risk).

---

## Best Practices

### Before Migration

1. **Document current state**: Save embedding counts, storage size, search quality metrics
2. **Back up database**: Full `pg_dump` before any re-indexing
3. **Test on subset**: Try migration on one repository first
4. **Measure baseline**: Run search quality benchmarks to compare against
5. **Inform team**: Coordinate migration timing with team members

### During Migration

1. **Monitor progress**: Set up automated monitoring queries
2. **Watch for errors**: Check logs for API failures or timeouts
3. **Track costs**: Monitor cloud provider billing dashboards
4. **Verify incrementally**: Test search quality at 25%, 50%, 75% completion
5. **Keep old embeddings**: Don't delete until migration is verified

### After Migration

1. **Run quality checks**: Compare search results to baseline
2. **Monitor performance**: Track search latency and throughput
3. **Gather feedback**: Ask team members about search quality
4. **Document changes**: Update runbooks and team documentation
5. **Schedule review**: Plan 30-day and 90-day quality reviews

### Production Checklist

- [ ] Database backup completed and verified
- [ ] Migration plan documented and reviewed
- [ ] Rollback procedure tested
- [ ] Monitoring alerts configured
- [ ] Team informed of migration window
- [ ] Search quality baseline captured
- [ ] Test repository migrated successfully
- [ ] Cost tracking enabled
- [ ] Post-migration verification plan ready

---

## Troubleshooting

### Common Issues

**Issue**: Search returns no results after migration

**Cause**: Embeddings not generated or columns empty

**Solution**:
```bash
# Check embedding status
psql -h localhost -U postgres crewchief -c "
  SELECT COUNT(*) AS total,
         COUNT(code_embedding_ollama) AS ollama
  FROM chunks;
"

# Re-run embedding generation
crewchief maproom scan --generate-embeddings
```

---

**Issue**: Migration is very slow

**Cause**: CPU-only embedding generation or network latency

**Solution**:
```bash
# For Ollama: Verify GPU usage
nvidia-smi  # Should show ollama process using GPU

# For cloud providers: Check network
ping api.openai.com
curl -w "@curl-format.txt" -o /dev/null -s https://api.openai.com

# Optimize: Use parallel processing (if supported)
export EMBEDDING_BATCH_SIZE=50  # Increase batch size
```

---

**Issue**: High API costs during migration

**Cause**: Re-embedding already processed chunks

**Solution**:
```bash
# Verify only missing embeddings are processed
psql -h localhost -U postgres crewchief -c "
  SELECT COUNT(*) FROM chunks WHERE code_embedding_ollama IS NULL;
"

# Set budget alerts in cloud provider dashboard
```

---

**Issue**: Search quality degraded after migration

**Cause**: Different embedding spaces between providers

**Solution**:
```bash
# Rollback to original provider temporarily
export MAPROOM_EMBEDDING_PROVIDER=openai

# Run side-by-side comparison
crewchief maproom search "query" --provider openai > results-openai.txt
crewchief maproom search "query" --provider ollama > results-ollama.txt

# Fine-tune search parameters (if needed)
```

---

## Additional Resources

### Documentation

- **[Provider Setup Guides](../providers/README.md)** - Detailed setup for each provider
- **[Provider Comparison](../providers/comparison.md)** - Feature and cost comparison
- **[Column Selection Logic](../../.crewchief/archive/projects/MPEMBED_multi-provider-embeddings/planning-multi-provider-embeddings/phase-4-unified-pipeline.md#mpembed-4001-column-selection-logic)** - Technical details
- **[Search Implementation](../../crates/maproom/src/search/)** - Source code reference

### Tools and Scripts

- **Migration progress tracker**: See examples in "Monitor Migration Progress" sections
- **Cost estimator**: Calculate expected costs before migration
- **Quality benchmarking**: Compare search results across providers

### Community

- **[GitHub Issues](https://github.com/yourusername/crewchief/issues)** - Report migration issues
- **[Discussions](https://github.com/yourusername/crewchief/discussions)** - Share migration experiences

---

## Need Help?

**Planning a migration?** Review this guide and the [Provider Comparison](../providers/comparison.md) to choose the right strategy.

**Experiencing issues?** Check [Troubleshooting](#troubleshooting) or [provider-specific troubleshooting guides](../providers/README.md#troubleshooting).

**Want personalized advice?** Open a [GitHub Discussion](https://github.com/yourusername/crewchief/discussions) with your use case details.

---

**Last Updated**: October 2025

**Related Documentation**:
- [Provider Setup Guides](../providers/README.md)
- [Provider Comparison](../providers/comparison.md)
- [OpenAI Setup](../providers/openai-setup.md)
- [Ollama Setup](../providers/ollama-setup.md)
- [Google Vertex AI Setup](../providers/google-vertex-ai-setup.md)

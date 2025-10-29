# Ticket: MPEMBED-5006: Migration guide for existing users

## Status
- [x] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- mcp-tools-engineer
- verify-ticket
- commit-ticket

## Summary
Document migration paths for existing users with OpenAI embeddings. Cover switching providers, preserving existing embeddings, running mixed embedding setups, and re-indexing strategies.

## Background
This ticket addresses a critical user experience question: "I already have OpenAI embeddings. What happens when I switch to Ollama?" The guide must clearly explain data preservation, migration options, and best practices.

Reference: crewchief_context/maproom/MPEMBED-multi-provider-embeddings/phase-5-mcp-documentation.md

## Acceptance Criteria
- [x] Clear explanation of what happens to existing embeddings
- [x] Step-by-step guide to switch from OpenAI to Ollama
- [x] Instructions for preserving OpenAI embeddings
- [x] Guide for running both providers simultaneously
- [x] Re-indexing strategy and commands
- [x] Cost comparison for migration scenarios
- [x] FAQ section addressing common concerns

## Technical Requirements
- Document format: Markdown with clear sections
- Include code examples for all migration scenarios
- Provide database query examples to inspect embeddings
- Link to relevant technical docs (COALESCE logic, column selection)
- Test migration scenarios on sample database

## Implementation Notes
```markdown
# Embedding Provider Migration Guide

## Overview
This guide helps existing Maproom users migrate between embedding providers or run multiple providers simultaneously.

## What Happens to Existing Embeddings?

### Short Answer
**Your existing embeddings are preserved.** New embeddings are stored in separate columns based on dimension.

### Technical Details
- **1536-dim embeddings** (OpenAI) → `code_embedding`, `doc_embedding`
- **768-dim embeddings** (Ollama/Google) → `code_embedding_ollama`, `doc_embedding_ollama`

Search automatically uses COALESCE to prefer 768-dim over 1536-dim when both exist.

## Migration Scenarios

### Scenario 1: Switch from OpenAI to Ollama (Preserve Existing)

**Goal**: Keep OpenAI embeddings, add Ollama for new/updated chunks

**Steps**:
```bash
# 1. Install and configure Ollama
ollama pull nomic-embed-text

# 2. Set provider (optional - auto-detected)
export EMBEDDING_PROVIDER=ollama

# 3. Scan for missing embeddings
crewchief maproom scan --generate-embeddings

# Result: Only chunks without Ollama embeddings are processed
```

**What happens**:
- Chunks with only OpenAI embeddings: Keep using OpenAI (1536-dim)
- New chunks: Get Ollama embeddings (768-dim)
- Search works across both embedding types

**Cost**: $0 (Ollama is free)

---

### Scenario 2: Full Re-index with Ollama

**Goal**: Replace all OpenAI embeddings with Ollama

**Steps**:
```bash
# 1. Back up database (optional but recommended)
pg_dump crewchief > backup.sql

# 2. Clear existing embeddings
psql crewchief -c "UPDATE chunks SET code_embedding_ollama = NULL, doc_embedding_ollama = NULL"

# 3. Re-index with Ollama
export EMBEDDING_PROVIDER=ollama
crewchief maproom scan --generate-embeddings

# 4. Optional: Clear OpenAI embeddings to save space
psql crewchief -c "UPDATE chunks SET code_embedding = NULL, doc_embedding = NULL"
```

**Time estimate**: ~30 minutes for 100K chunks (local GPU)

**Cost**: $0 (Ollama is free)

---

### Scenario 3: Gradual Migration (Recommended)

**Goal**: Migrate incrementally, preserve existing embeddings

**Steps**:
```bash
# 1. Switch to Ollama for new embeddings
export EMBEDDING_PROVIDER=ollama

# 2. Continue using mixed embeddings
# - OpenAI embeddings continue working
# - New/updated chunks get Ollama embeddings

# 3. Monitor progress
psql crewchief -c "
  SELECT
    COUNT(*) FILTER (WHERE code_embedding IS NOT NULL) AS openai_count,
    COUNT(*) FILTER (WHERE code_embedding_ollama IS NOT NULL) AS ollama_count,
    COUNT(*) FILTER (WHERE code_embedding IS NOT NULL AND code_embedding_ollama IS NOT NULL) AS both_count
  FROM chunks
"
```

**Advantages**:
- No downtime
- No re-indexing cost
- Gradual transition over weeks/months

---

### Scenario 4: Run Both Providers Simultaneously

**Goal**: Generate both 768-dim and 1536-dim embeddings

**Use case**: Experimentation, A/B testing, migration validation

**Steps**:
```bash
# 1. Index with OpenAI
export EMBEDDING_PROVIDER=openai
crewchief maproom scan --generate-embeddings

# 2. Index with Ollama (adds to existing)
export EMBEDDING_PROVIDER=ollama
crewchief maproom scan --generate-embeddings

# Result: Each chunk has both embeddings
```

**Storage impact**: ~2x embedding storage (~1.5MB per 1K chunks)

**Search behavior**: Prefers Ollama (768-dim) when both present

---

## Verify Your Migration

### Check Embedding Distribution
```sql
SELECT
  CASE
    WHEN code_embedding IS NOT NULL AND code_embedding_ollama IS NULL THEN 'OpenAI only'
    WHEN code_embedding IS NULL AND code_embedding_ollama IS NOT NULL THEN 'Ollama only'
    WHEN code_embedding IS NOT NULL AND code_embedding_ollama IS NOT NULL THEN 'Both'
    ELSE 'Neither'
  END AS embedding_status,
  COUNT(*) AS chunk_count
FROM chunks
GROUP BY embedding_status;
```

### Test Search Quality
```bash
# Search with your typical queries
crewchief maproom search "authentication flow"
crewchief maproom search "database query"
crewchief maproom search "error handling"

# Compare results to pre-migration bookmarks
```

## Rollback Strategy

### If Migration Fails
```bash
# 1. Restore from backup
pg_restore -d crewchief backup.sql

# 2. Or clear new embeddings
psql crewchief -c "UPDATE chunks SET code_embedding_ollama = NULL, doc_embedding_ollama = NULL"

# 3. Switch back to OpenAI
export EMBEDDING_PROVIDER=openai
```

## FAQ

**Q: Will search quality change when I switch providers?**

A: Embedding quality is similar but not identical. Most users report comparable or better results with Ollama (nomic-embed-text). Run A/B tests on your codebase.

**Q: Can I delete OpenAI embeddings after migrating?**

A: Yes, but keep for 30-90 days to compare quality:
```sql
-- After confirming Ollama quality is good
UPDATE chunks SET code_embedding = NULL, doc_embedding = NULL;
VACUUM FULL chunks; -- Reclaim disk space
```

**Q: How much disk space do embeddings use?**

- OpenAI (1536-dim): ~6KB per chunk
- Ollama (768-dim): ~3KB per chunk
- Both: ~9KB per chunk

**Q: What if I need to go back to OpenAI?**

Simply set `EMBEDDING_PROVIDER=openai` and scan. OpenAI embeddings are preserved unless explicitly deleted.

**Q: Do I need to re-index when switching between Ollama and Google?**

No, both use 768 dimensions. Just switch the provider:
```bash
export EMBEDDING_PROVIDER=google
# Embeddings stored in same columns
```

**Q: How do I force re-embedding of specific files?**

```bash
crewchief maproom upsert \
  --repo myrepo \
  --worktree main \
  --root /path/to/repo \
  --commit HEAD \
  --paths src/auth.ts src/db.ts
```

## Cost Comparison

| Scenario | OpenAI Cost | Ollama Cost | Time |
|----------|-------------|-------------|------|
| Keep existing | $0 | $0 | 0 min |
| Add Ollama (incremental) | $0 | $0 | ~5 min |
| Full re-index (100K chunks) | ~$50 | $0 | ~30 min |

## Best Practices

1. **Start with gradual migration** - Let new embeddings use Ollama naturally
2. **Monitor search quality** - Keep metrics before/after for comparison
3. **Backup before re-indexing** - Database backups are cheap insurance
4. **Test on subset first** - Try migration on one repository before all
5. **Document your decision** - Leave notes for future team members

## Need Help?

- Check troubleshooting: docs/providers/troubleshooting.md
- Provider setup guides: docs/providers/
- GitHub issues: https://github.com/your-repo/issues
```

## Dependencies
- MPEMBED-5005 (Setup guides for linking)

## Risk Assessment
- **Risk**: Users may accidentally delete embeddings
  - **Mitigation**: Emphasize backup steps, provide rollback instructions

## Files/Packages Affected
- docs/guides/provider-migration.md (create)
- docs/guides/README.md (modify - add link)

## Implementation Notes

**Completed**: 2025-10-29

**Files Created**:
1. `/workspace/docs/guides/provider-migration.md` (788 lines)
   - Comprehensive migration guide with 5 detailed scenarios
   - Technical explanations of column storage and COALESCE logic
   - Step-by-step instructions with code examples
   - Verification queries and rollback procedures
   - Cost comparison tables
   - Extensive FAQ section (14 Q&A pairs)
   - Troubleshooting section with common issues

2. `/workspace/docs/guides/README.md` (59 lines)
   - Index page for guides directory
   - Links to migration guide and related documentation
   - Quick navigation for users

**Files Modified**:
3. `/workspace/docs/providers/README.md`
   - Updated "Switching Providers" section with correct link to migration guide
   - Added more detailed bullet points about migration features

**Key Features Implemented**:
- **5 Migration Scenarios**: Preserve existing, full re-index, gradual, simultaneous, Ollama-to-cloud
- **Verification Section**: SQL queries to check embedding distribution, search quality tests, database size monitoring
- **Rollback Strategy**: Three rollback methods with detailed steps
- **Cost Analysis**: Comprehensive table comparing all migration scenarios with time estimates
- **FAQ Section**: 14 frequently asked questions covering general, technical, and operational concerns
- **Best Practices**: Before/during/after checklists and production checklist
- **Troubleshooting**: Common issues with causes and solutions

**Content Highlights**:
- Clear explanation of dimension-based column storage (768-dim vs 1536-dim)
- COALESCE preference logic explanation (prefers 768-dim)
- Database backup recommendations throughout
- Time and cost estimates for each scenario
- Concrete SQL queries for verification and monitoring
- Links to related technical documentation

**Quality Assurance**:
- All acceptance criteria met
- Follows existing documentation style (matches providers README structure)
- Includes code examples for all scenarios
- Comprehensive cross-linking to related docs
- Production-ready with operational checklists

# Decomposition: Maproom Semantic Search Improvements

This initiative decomposes into **5 projects** across 3 phases.

## Project Summaries

The following projects are ready for creation via `/create-project`:

### Phase 1: Foundation (Weeks 1-4)

**1. SRCHTRNSP: Search Transparency and Error Diagnostics**
- Slug: `SRCHTRNSP_search-transparency`
- Effort: Small (1-2 weeks)
- Priority: Critical
- Summary: Replace generic RPC_ERROR with actionable diagnostics, add query understanding feedback

**2. SRCHFLTR: Result Type Filtering and Smart Defaults**
- Slug: `SRCHFLTR_result-filtering`
- Effort: Small-Medium (2-3 weeks)
- Priority: Critical
- Summary: Add type/path filtering with smart defaults (code-first, exclude archived)

### Phase 2: Intelligence (Weeks 5-10)

**3. SRCHCONF: Confidence Scoring and Progressive Results**
- Slug: `SRCHCONF_confidence-scoring`
- Effort: Medium (2-3 weeks)
- Priority: High
- Summary: Normalize scores to confidence bands, progressive filtering of low-confidence results

**4. SRCHREL: Relationship-Aware Search**
- Slug: `SRCHREL_relationship-search`
- Effort: Medium-Large (3-4 weeks)
- Priority: Medium-High
- Summary: Cluster related chunks via graph traversal, expose architectural relationships

### Phase 3: Validation (Weeks 11-13)

**5. SRCHTST: Comprehensive Search Test Suites**
- Slug: `SRCHTST_search-test-suites`
- Effort: Medium (2-3 weeks)
- Priority: High
- Summary: Validate semantic understanding, architectural discovery, grep-impossible tasks

## Execution Order

1. Phase 1: SRCHTRNSP + SRCHFLTR (parallel)
2. Phase 2: SRCHCONF → SRCHREL (sequential)
3. Phase 3: SRCHTST (validates all)

## Next Steps

Use `/create-project` with the slugs above to scaffold each project with full planning docs.

Example:
```
/create-project SRCHTRNSP_search-transparency "Replace generic RPC_ERROR with actionable diagnostics and add query understanding feedback to search responses"
```

See `multi-project-overview.md` for detailed dependencies and timeline.

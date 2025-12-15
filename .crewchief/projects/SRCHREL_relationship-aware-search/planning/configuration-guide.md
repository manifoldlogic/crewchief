# Quality-Weighted Graph Scoring Configuration Guide

**Ticket:** SRCHREL-3001
**Feature:** Relationship-Aware Search Ranking (SRCHREL)

---

## Overview

Quality-weighted graph scoring improves search ranking by differentiating between edges originating from production code vs test code. Functions called by production code rank higher than functions only called by tests.

### Key Benefits

- **Better relevance**: Entry points and core business logic surface higher
- **Reduced noise**: Test utilities don't inflate importance scores
- **Configurable weights**: Tune for your codebase's test/production ratio

---

## Quick Start

Enable quality-weighted scoring with a single environment variable:

```bash
export MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_QUALITY_WEIGHTED_GRAPH=true
```

Or via YAML configuration:

```yaml
feature_flags:
  enable_quality_weighted_graph: true
```

---

## Configuration Reference

### Feature Flags

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enable_quality_weighted_graph` | bool | `false` | Enable quality-weighted graph scoring |

**Environment Variable:** `MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_QUALITY_WEIGHTED_GRAPH`

### Graph Importance Config

Located under `graph_importance` section in YAML:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enable_quality_scoring` | bool | `false` | Enable quality-weighted edge scoring |
| `edge_quality_weights` | object | see below | Weight configuration for edge types |
| `fusion_weight_override` | f32? | `null` | Override graph weight in fusion (0.0-1.0) |

**Environment Variables:**
- `MAPROOM_SEARCH_GRAPH_IMPORTANCE_ENABLE_QUALITY_SCORING`

### Edge Quality Weights

Located under `graph_importance.edge_quality_weights`:

| Field | Type | Default | Valid Range | Description |
|-------|------|---------|-------------|-------------|
| `production_code` | f32 | `1.0` | 0.0-10.0 | Weight for edges from production code |
| `test_code` | f32 | `0.5` | 0.0-10.0 | Weight for edges from test code |
| `calls` | f32 | `1.0` | 0.0-10.0 | Weight multiplier for 'calls' edge type |

**Environment Variables:**
- `MAPROOM_SEARCH_EDGE_QUALITY_WEIGHTS_PRODUCTION_CODE`
- `MAPROOM_SEARCH_EDGE_QUALITY_WEIGHTS_TEST_CODE`
- `MAPROOM_SEARCH_EDGE_QUALITY_WEIGHTS_CALLS`

---

## Example Configurations

### Default (Conservative)

Use defaults for most codebases. Test code contributes half as much as production.

```yaml
feature_flags:
  enable_quality_weighted_graph: true

graph_importance:
  enable_quality_scoring: true
  edge_quality_weights:
    production_code: 1.0
    test_code: 0.5
    calls: 1.0
```

### Heavy Test Penalty (Test-Heavy Codebases)

For codebases with extensive tests (>50% test code), reduce test edge weight further:

```yaml
feature_flags:
  enable_quality_weighted_graph: true

graph_importance:
  enable_quality_scoring: true
  edge_quality_weights:
    production_code: 1.0
    test_code: 0.2  # Tests contribute only 20%
    calls: 1.0
```

**When to use:**
- Large test suites dominate edge counts
- Test helpers are frequently appearing in top results
- You want production entry points to rank much higher

### Boosted Calls (Call Graphs Primary Signal)

When function call relationships are the most important signal:

```yaml
feature_flags:
  enable_quality_weighted_graph: true

graph_importance:
  enable_quality_scoring: true
  edge_quality_weights:
    production_code: 1.0
    test_code: 0.5
    calls: 2.0  # Calls edges weighted 2x
```

**When to use:**
- Import edges are less meaningful than call edges
- Deep call hierarchies indicate importance
- You want to prioritize frequently-called functions

### A/B Testing with Fusion Override

Override graph weight in the fusion formula to test impact:

```yaml
feature_flags:
  enable_quality_weighted_graph: true

graph_importance:
  enable_quality_scoring: true
  fusion_weight_override: 0.15  # Increase graph weight from default 0.10
```

**Note:** Other fusion weights (FTS, vector, recency, churn) are automatically renormalized to maintain sum=1.0.

### Disabled (Legacy Mode)

Revert to legacy graph scoring without quality weighting:

```yaml
feature_flags:
  enable_quality_weighted_graph: false

# graph_importance settings are ignored when flag is disabled
```

---

## Tuning Guidelines

### When to Increase `production_code` Weight

- Your production code has distinct architectural layers
- Entry points should rank significantly higher
- Core business logic is your primary search target

### When to Decrease `test_code` Weight

- Large test suite (>50% of codebase)
- Test helpers appear in top results too often
- Tests use heavy mocking that inflates edge counts

### When to Adjust `calls` Weight

- Call graphs are more meaningful than imports
- Your codebase uses dependency injection (reduce calls weight)
- Function calls represent strong coupling (increase calls weight)

### When to Use `fusion_weight_override`

- A/B testing different configurations
- Your codebase benefits from stronger/weaker graph signals
- Fine-tuning search relevance for specific use cases

---

## Test Detection Patterns

Files are classified as "test code" based on path patterns:

| Pattern | Example |
|---------|---------|
| `**/test/**` | `src/test/utils.ts` |
| `**/tests/**` | `tests/integration/api.test.ts` |
| `**/__tests__/**` | `src/__tests__/component.test.js` |
| `*.test.*` | `handler.test.ts` |
| `*.spec.*` | `service.spec.js` |
| `*_test.*` | `auth_test.go` |

Additionally, chunks with `kind` containing "test" are classified as test code.

---

## Monitoring and Validation

### Verify Configuration

Run a debug search to see score breakdown:

```bash
cargo run --bin crewchief-maproom -- search \
  --repo your-repo \
  --query "your query" \
  --debug
```

The `--debug` flag shows score components including graph importance.

### Compare Legacy vs Quality-Weighted

```bash
# Legacy mode
MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_QUALITY_WEIGHTED_GRAPH=false \
  cargo run --bin crewchief-maproom -- search --repo your-repo --query "handler" --debug

# Quality-weighted mode
MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_QUALITY_WEIGHTED_GRAPH=true \
  cargo run --bin crewchief-maproom -- search --repo your-repo --query "handler" --debug
```

### Expected Improvements

| Query Type | Expected Change |
|------------|-----------------|
| Entry points, handlers | ↑ Higher ranking |
| Core business logic | ↑ Higher ranking |
| Utility functions | → Same or slight change |
| Test helpers | ↓ Lower ranking (correctly) |
| Test fixtures | ↓ Lower ranking (correctly) |

---

## Troubleshooting

### Quality scoring not affecting results

1. **Check feature flag is enabled:**
   ```bash
   echo $MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_QUALITY_WEIGHTED_GRAPH
   ```

2. **Verify configuration loaded:**
   - Look for log message: "Override: feature_flags.enable_quality_weighted_graph = true"

3. **Check edge data exists:**
   - Quality scoring requires `chunk_edges` table to be populated
   - Run indexer to populate edges: `cargo run --bin crewchief-maproom -- scan ...`

### Test files not being detected

1. **Check file paths match patterns:**
   - Ensure test files are in `/test/`, `/tests/`, `/__tests__/` directories
   - Or use `.test.`, `.spec.`, `_test.` naming conventions

2. **Non-standard test locations:**
   - Consider adjusting weights if your tests use unusual paths
   - Files not matching patterns get `production_code` weight

### Performance issues

- Quality-weighted scoring adds ~2.4× computational overhead but remains sub-microsecond
- If p95 latency exceeds 35ms, check database indexing (see performance-results.md)
- Ensure `idx_chunk_edges_dst_type_src` index exists for optimal query performance

---

## Configuration File Locations

Configuration is loaded from (in order):

1. `./config/maproom-search.yml` (relative to current directory)
2. `../config/maproom-search.yml` (relative to binary)
3. `/etc/maproom/maproom-search.yml` (system-wide)

Environment variables override file values.

---

## Complete YAML Reference

```yaml
# maproom-search.yml

feature_flags:
  enable_vector_search: true
  enable_hybrid_fusion: true
  enable_graph_signals: true
  enable_temporal_signals: true
  enable_query_cache: true
  enable_hot_reload: true
  enable_quality_weighted_graph: false  # Enable quality scoring

graph_importance:
  enable_quality_scoring: false
  edge_quality_weights:
    production_code: 1.0
    test_code: 0.5
    calls: 1.0
  fusion_weight_override: null  # Optional: override graph weight in fusion

# ... other configuration sections (embedding, fusion, performance, etc.)
```

---

**Document Version:** 1.0
**Author:** docs-engineer agent
**Last Updated:** 2025-12-15

# Maproom Search Configuration Guide

## Overview

Maproom's hybrid search system provides a comprehensive configuration system that allows you to customize search behavior, performance characteristics, and feature availability. This guide covers all configuration options, best practices, and troubleshooting.

## Table of Contents

- [Configuration File Location](#configuration-file-location)
- [Configuration Sections](#configuration-sections)
  - [Embedding Configuration](#embedding-configuration)
  - [Fusion Configuration](#fusion-configuration)
  - [Performance Configuration](#performance-configuration)
  - [Index Configuration](#index-configuration)
  - [Feature Flags](#feature-flags)
- [Environment Variable Overrides](#environment-variable-overrides)
- [Hot Reload](#hot-reload)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)
- [Examples](#examples)

## Configuration File Location

Maproom searches for configuration files in the following locations (in order):

1. `./config/maproom-search.yml` (relative to current directory)
2. `../config/maproom-search.yml` (relative to binary location)
3. `/etc/maproom/maproom-search.yml` (system-wide)

If no configuration file is found, Maproom uses sensible defaults.

## Configuration Sections

### Embedding Configuration

Controls vector embedding generation for semantic search.

```yaml
embedding:
  # Provider: openai, cohere, or local
  provider: openai

  # Model name (provider-specific)
  model_name: text-embedding-3-small

  # Embedding dimension (must match model output)
  dimension: 1536

  # Cache configuration
  cache_size: 10000  # Number of embeddings to cache
  cache_ttl_seconds: 3600  # Time-to-live for cached embeddings
```

#### Provider Options

**OpenAI Models:**
- `text-embedding-3-small` (1536 dimensions, fast, cost-effective)
- `text-embedding-3-large` (3072 dimensions, highest quality)
- `text-embedding-ada-002` (1536 dimensions, legacy)

**Cohere Models:**
- `embed-english-v3.0` (1024 dimensions)
- `embed-multilingual-v3.0` (1024 dimensions)

**Local Models:**
- `all-MiniLM-L6-v2` (384 dimensions, fast)
- Configure dimension to match your local model

#### Validation Rules

- `provider` cannot be empty
- `model_name` cannot be empty
- `dimension` must be greater than 0
- `cache_size` can be 0 to disable caching (not recommended for production)

### Fusion Configuration

Controls how different search signals are combined.

```yaml
fusion:
  # Fusion method: rrf, weighted, or learned
  method: rrf

  # RRF k parameter (controls rank decay, higher = slower decay)
  rrf_k: 60

  # Individual signal weights (used for weighted fusion method)
  weights:
    fts: 0.40      # Full-text search weight
    vector: 0.35   # Vector similarity weight
    graph: 0.10    # Graph importance weight
    recency: 0.10  # Recency signal weight
    churn: 0.05    # Code churn weight (inverted)
```

#### Fusion Methods

**RRF (Reciprocal Rank Fusion)** - Recommended for production
- Combines results based on their ranks rather than raw scores
- More robust to score scale differences
- `rrf_k` parameter controls decay (typical range: 30-100)
- Higher `rrf_k` = slower decay = more weight to lower-ranked results

**Weighted** - Simple weighted average
- Direct combination of normalized scores
- Weights should ideally sum to 1.0
- Good for A/B testing different weight combinations

**Learned** - ML-based fusion (future)
- Not yet implemented
- Will use machine learning to learn optimal weights

#### Weight Tuning Guidelines

**Default weights (recommended starting point):**
- FTS: 0.40 - Highest weight for keyword matching
- Vector: 0.35 - Strong semantic similarity signal
- Graph: 0.10 - Moderate importance boost
- Recency: 0.10 - Moderate recency boost
- Churn: 0.05 - Slight penalty for unstable code

**Code-heavy repositories:**
- Increase FTS weight (0.50+)
- Decrease vector weight (0.25-0.30)

**Documentation-heavy repositories:**
- Increase vector weight (0.40+)
- Decrease graph weight (0.05)

**Stable codebases:**
- Increase recency weight (0.15+)
- Decrease churn weight (0.02)

#### Validation Rules

- All weights must be non-negative
- `rrf_k` must be greater than 0
- Weights should sum to 1.0 (warning if not, but not enforced)

### Performance Configuration

Controls search execution performance and limits.

```yaml
performance:
  # Maximum candidates per search method
  max_candidates_per_method: 100

  # Final result limit (after fusion)
  final_result_limit: 20

  # Query timeout in milliseconds
  timeout_ms: 1000

  # Enable parallel query execution
  parallel_execution: true
```

#### Performance Tuning

**High-throughput scenarios:**
```yaml
max_candidates_per_method: 50
final_result_limit: 10
timeout_ms: 500
parallel_execution: true
```

**High-quality scenarios:**
```yaml
max_candidates_per_method: 200
final_result_limit: 50
timeout_ms: 2000
parallel_execution: true
```

**Resource-constrained environments:**
```yaml
max_candidates_per_method: 30
final_result_limit: 10
timeout_ms: 500
parallel_execution: false
```

#### Validation Rules

- `max_candidates_per_method` must be greater than 0
- `final_result_limit` must be greater than 0
- `timeout_ms` can be 0 for no timeout (not recommended)

### Index Configuration

Controls vector index parameters and maintenance.

```yaml
index:
  # IVFFlat index parameters (PostgreSQL pgvector)
  ivfflat_lists: 100
  ivfflat_probes: 10

  # Index refresh interval in seconds
  refresh_interval_seconds: 3600
```

#### IVFFlat Index Tuning

The IVFFlat index partitions vectors into clusters for faster search.

**Small datasets (<10K vectors):**
```yaml
ivfflat_lists: 50
ivfflat_probes: 5
```

**Medium datasets (10K-100K vectors):**
```yaml
ivfflat_lists: 100
ivfflat_probes: 10
```

**Large datasets (>100K vectors):**
```yaml
ivfflat_lists: 200
ivfflat_probes: 20
```

**Guidelines:**
- `ivfflat_lists`: Recommended ~√(total_vectors)
- `ivfflat_probes`: Higher = better recall, slower queries (typical: 10-20)
- `ivfflat_probes` should not exceed `ivfflat_lists`

#### Validation Rules

- `ivfflat_lists` must be greater than 0
- `ivfflat_probes` must be greater than 0
- Warning if `ivfflat_probes` > `ivfflat_lists`

### Feature Flags

Enable/disable specific search features.

```yaml
feature_flags:
  # Enable vector similarity search
  enable_vector_search: true

  # Enable hybrid fusion (combine multiple signals)
  enable_hybrid_fusion: true

  # Enable graph-based ranking signals
  enable_graph_signals: true

  # Enable temporal signals (recency and churn)
  enable_temporal_signals: true

  # Enable query result caching
  enable_query_cache: true

  # Enable hot reload of fusion weights
  enable_hot_reload: true
```

#### Feature Flag Combinations

**Full hybrid search (default):**
- All flags enabled

**FTS-only mode (fastest):**
```yaml
enable_vector_search: false
enable_hybrid_fusion: false
enable_graph_signals: false
enable_temporal_signals: false
enable_query_cache: true
enable_hot_reload: false
```

**Vector-only mode:**
```yaml
enable_vector_search: true
enable_hybrid_fusion: false
enable_graph_signals: false
enable_temporal_signals: false
```

**Semantic search without graph:**
```yaml
enable_vector_search: true
enable_hybrid_fusion: true
enable_graph_signals: false
enable_temporal_signals: true
```

## Environment Variable Overrides

Any configuration value can be overridden using environment variables with the pattern:

```bash
MAPROOM_SEARCH_<SECTION>_<KEY>=<value>
```

### Examples

```bash
# Override fusion weights
export MAPROOM_SEARCH_FUSION_WEIGHTS_FTS=0.5
export MAPROOM_SEARCH_FUSION_WEIGHTS_VECTOR=0.3

# Override performance settings
export MAPROOM_SEARCH_PERFORMANCE_TIMEOUT_MS=500
export MAPROOM_SEARCH_PERFORMANCE_PARALLEL_EXECUTION=false

# Override feature flags
export MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_VECTOR_SEARCH=false

# Override embedding settings
export MAPROOM_SEARCH_EMBEDDING_DIMENSION=2000
export MAPROOM_SEARCH_EMBEDDING_CACHE_SIZE=20000

# Override index settings
export MAPROOM_SEARCH_INDEX_IVFFLAT_PROBES=15
```

### Nested Configuration

For nested values in the YAML (like `fusion.weights.fts`), use underscores:

```bash
# YAML: fusion.weights.fts
export MAPROOM_SEARCH_FUSION_WEIGHTS_FTS=0.5

# YAML: embedding.cache_size
export MAPROOM_SEARCH_EMBEDDING_CACHE_SIZE=5000
```

### Override Precedence

1. Environment variables (highest priority)
2. Configuration file
3. Default values (lowest priority)

### Checking Active Overrides

Maproom logs all active environment variable overrides at startup:

```
INFO  Configuration loaded successfully from: config/maproom-search.yml
DEBUG Override: fusion.weights.fts = 0.5
DEBUG Override: performance.timeout_ms = 500
```

## Hot Reload

Fusion weights can be updated at runtime without restarting the service when `feature_flags.enable_hot_reload=true`.

### How It Works

1. Maproom watches the configuration file for changes
2. When changes are detected, the new configuration is loaded
3. The new configuration is validated
4. If validation passes, only hot-reloadable fields are updated
5. If validation fails, the change is rejected and logged

### Hot-Reloadable Fields

Currently, only fusion-related settings are hot-reloadable:
- `fusion.weights.fts`
- `fusion.weights.vector`
- `fusion.weights.graph`
- `fusion.weights.recency`
- `fusion.weights.churn`
- `fusion.rrf_k`
- `fusion.method`

### Non-Hot-Reloadable Fields

The following require a service restart to take effect:
- Embedding configuration
- Performance configuration
- Index configuration
- Feature flags

### Hot Reload Best Practices

1. **Test changes in development first** - Validate configuration changes before applying to production
2. **Make incremental changes** - Adjust one or two weights at a time
3. **Monitor query performance** - Watch latency and result quality after changes
4. **Keep backups** - Save working configurations before making changes
5. **Use version control** - Track configuration changes in git

### Manual Reload

You can trigger a configuration reload programmatically:

```rust
use crewchief_maproom::config::{SearchConfig, ConfigReloader};
use std::sync::Arc;
use tokio::sync::RwLock;

let config = Arc::new(RwLock::new(SearchConfig::load_default().await?));
let reloader = ConfigReloader::new(config.clone(), "config/maproom-search.yml")?;

// Manually trigger reload
reloader.reload().await?;
```

## Best Practices

### 1. Start with Defaults

Begin with the default configuration and adjust based on observed behavior:

```yaml
# Use defaults as starting point
fusion:
  method: rrf
  rrf_k: 60
  weights:
    fts: 0.40
    vector: 0.35
    graph: 0.10
    recency: 0.10
    churn: 0.05
```

### 2. Tune for Your Use Case

**Code search optimization:**
- Increase FTS weight for better keyword matching
- Consider disabling recency for stable codebases

**Documentation search optimization:**
- Increase vector weight for better semantic matching
- Reduce graph weight (documentation typically has fewer graph connections)

### 3. Monitor and Iterate

1. Enable query logging
2. Collect relevance metrics
3. A/B test different configurations
4. Adjust weights based on data

### 4. Use Environment Variables for Deployment

Keep the same configuration file across environments and use environment variables for environment-specific overrides:

```bash
# production.env
export MAPROOM_SEARCH_PERFORMANCE_TIMEOUT_MS=1500
export MAPROOM_SEARCH_INDEX_IVFFLAT_LISTS=200

# staging.env
export MAPROOM_SEARCH_PERFORMANCE_TIMEOUT_MS=2000
export MAPROOM_SEARCH_INDEX_IVFFLAT_LISTS=50
```

### 5. Version Control Configuration

Track configuration changes in version control with meaningful commit messages:

```bash
git add config/maproom-search.yml
git commit -m "config: increase FTS weight for code-heavy repos"
```

### 6. Document Custom Configurations

Add comments to your configuration file explaining why specific values were chosen:

```yaml
fusion:
  # Increased FTS weight to 0.5 based on A/B test results (2024-10-15)
  # Showed 15% improvement in code search relevance
  weights:
    fts: 0.50
    vector: 0.30
    graph: 0.10
    recency: 0.08
    churn: 0.02
```

## Troubleshooting

### Configuration Not Loading

**Problem:** Configuration file not found

**Solution:**
1. Check file exists at expected path
2. Verify file permissions (must be readable)
3. Check logs for file path being searched

```bash
# Check if file exists and is readable
ls -la config/maproom-search.yml

# Check Maproom logs
grep "Loading configuration" logs/maproom.log
```

### Invalid Configuration

**Problem:** Configuration validation fails

**Solution:**
1. Check for YAML syntax errors
2. Verify all required fields are present
3. Check value ranges and types

```bash
# Validate YAML syntax
yamllint config/maproom-search.yml

# Check Maproom logs for specific validation errors
grep "validation failed" logs/maproom.log
```

### Hot Reload Not Working

**Problem:** Configuration changes not taking effect

**Solution:**
1. Verify `enable_hot_reload: true` in feature flags
2. Check only hot-reloadable fields are changed
3. Verify file watcher has permissions
4. Check logs for reload errors

```bash
# Check if hot reload is enabled
grep "enable_hot_reload" config/maproom-search.yml

# Check for reload events in logs
grep "Configuration reloaded" logs/maproom.log
```

### Performance Issues

**Problem:** Queries are slow

**Solution:**
1. Reduce `max_candidates_per_method`
2. Decrease `ivfflat_probes`
3. Disable unused features
4. Enable parallel execution

```yaml
# Performance-optimized config
performance:
  max_candidates_per_method: 50
  final_result_limit: 10
  timeout_ms: 500
  parallel_execution: true

index:
  ivfflat_probes: 5

feature_flags:
  enable_graph_signals: false
  enable_temporal_signals: false
```

### Poor Result Quality

**Problem:** Search results not relevant

**Solution:**
1. Increase `max_candidates_per_method`
2. Increase `ivfflat_probes`
3. Adjust fusion weights
4. Enable all search signals

```yaml
# Quality-optimized config
performance:
  max_candidates_per_method: 200
  final_result_limit: 50

index:
  ivfflat_probes: 20

feature_flags:
  enable_vector_search: true
  enable_hybrid_fusion: true
  enable_graph_signals: true
  enable_temporal_signals: true
```

## Examples

### Minimal FTS-Only Configuration

Fastest configuration for high-throughput scenarios:

```yaml
embedding:
  provider: openai
  model_name: text-embedding-3-small
  dimension: 1536
  cache_size: 0  # Disable embedding cache
  cache_ttl_seconds: 3600

fusion:
  method: rrf
  rrf_k: 60
  weights:
    fts: 1.0
    vector: 0.0
    graph: 0.0
    recency: 0.0
    churn: 0.0

performance:
  max_candidates_per_method: 50
  final_result_limit: 10
  timeout_ms: 500
  parallel_execution: false

index:
  ivfflat_lists: 50
  ivfflat_probes: 5
  refresh_interval_seconds: 3600

feature_flags:
  enable_vector_search: false
  enable_hybrid_fusion: false
  enable_graph_signals: false
  enable_temporal_signals: false
  enable_query_cache: true
  enable_hot_reload: false
```

### High-Quality Hybrid Search

Best quality configuration for production:

```yaml
embedding:
  provider: openai
  model_name: text-embedding-3-large
  dimension: 3072
  cache_size: 20000
  cache_ttl_seconds: 7200

fusion:
  method: rrf
  rrf_k: 60
  weights:
    fts: 0.40
    vector: 0.35
    graph: 0.10
    recency: 0.10
    churn: 0.05

performance:
  max_candidates_per_method: 200
  final_result_limit: 50
  timeout_ms: 2000
  parallel_execution: true

index:
  ivfflat_lists: 200
  ivfflat_probes: 20
  refresh_interval_seconds: 1800

feature_flags:
  enable_vector_search: true
  enable_hybrid_fusion: true
  enable_graph_signals: true
  enable_temporal_signals: true
  enable_query_cache: true
  enable_hot_reload: true
```

### Development Configuration

Balanced configuration for local development:

```yaml
embedding:
  provider: openai
  model_name: text-embedding-3-small
  dimension: 1536
  cache_size: 5000
  cache_ttl_seconds: 3600

fusion:
  method: rrf
  rrf_k: 60
  weights:
    fts: 0.40
    vector: 0.35
    graph: 0.10
    recency: 0.10
    churn: 0.05

performance:
  max_candidates_per_method: 100
  final_result_limit: 20
  timeout_ms: 1000
  parallel_execution: true

index:
  ivfflat_lists: 50
  ivfflat_probes: 10
  refresh_interval_seconds: 3600

feature_flags:
  enable_vector_search: true
  enable_hybrid_fusion: true
  enable_graph_signals: true
  enable_temporal_signals: true
  enable_query_cache: true
  enable_hot_reload: true
```

## Additional Resources

- [Maproom Architecture Documentation](../.agents/archive/projects/HYBRID_SEARCH_hybrid-retrieval-system/planning/HYBRID_SEARCH_ARCHITECTURE.md)
- [Fusion Strategies Guide](../src/search/fusion/README.md)
- [Performance Tuning Guide](../docs/performance_tuning.md)
- [API Reference](https://docs.rs/crewchief-maproom)

## Support

For questions or issues:
- Open an issue on GitHub
- Check existing documentation
- Review logs for detailed error messages

# HYBRID_SEARCH Architecture: Hybrid Retrieval System

## System Architecture

### High-Level Flow
```
┌─────────────────────┐
│   Search Query      │
│  "authentication"   │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────────────────────┐
│       Query Processor                │
│  - Tokenization                      │
│  - Expansion (synonyms)              │
│  - Embedding generation              │
└─────────┬───────────────────────────┘
          │
          ├──────────────┬──────────────┬──────────────┐
          ▼              ▼              ▼              ▼
┌─────────────┐ ┌─────────────┐ ┌─────────────┐ ┌─────────────┐
│  FTS Query  │ │Vector Query │ │Graph Query  │ │Signal Query │
│  (tsvector) │ │ (embedding) │ │  (edges)    │ │(recency/churn)│
└─────────────┘ └─────────────┘ └─────────────┘ └─────────────┘
          │              │              │              │
          └──────────────┴──────────────┴──────────────┘
                                │
                                ▼
                    ┌─────────────────────┐
                    │   Score Fusion      │
                    │   (RRF/Weighted)    │
                    └─────────────────────┘
                                │
                                ▼
                    ┌─────────────────────┐
                    │   Result Assembly   │
                    │   (Top-K selection) │
                    └─────────────────────┘
```

## Core Components

### 1. Query Processor
```rust
pub struct QueryProcessor {
    tokenizer: Tokenizer,
    embedder: EmbeddingClient,
    expander: QueryExpander,
}

impl QueryProcessor {
    pub async fn process(&self, query: &str) -> ProcessedQuery {
        // Parallel processing
        let (tokens, embedding, expanded) = tokio::join!(
            self.tokenizer.tokenize(query),
            self.embedder.embed(query),
            self.expander.expand(query)
        );

        ProcessedQuery {
            original: query.to_string(),
            tokens,
            embedding,
            expanded_terms: expanded,
            mode: self.detect_mode(query),
        }
    }

    fn detect_mode(&self, query: &str) -> SearchMode {
        // Heuristics for query type detection
        if query.contains("::") || query.contains("->") {
            SearchMode::Code
        } else if query.split_whitespace().count() > 3 {
            SearchMode::Text
        } else {
            SearchMode::Auto
        }
    }
}
```

### 2. Embedding Service
```rust
pub struct EmbeddingService {
    client: OpenAIClient,  // Or local model
    cache: Arc<RwLock<LruCache<String, Vector>>>,
    config: EmbeddingConfig,
}

impl EmbeddingService {
    pub async fn embed_text(&self, text: &str) -> Result<Vector> {
        // Check cache first
        if let Some(cached) = self.cache.read().await.get(text) {
            return Ok(cached.clone());
        }

        // Generate embedding
        let embedding = match self.config.provider {
            Provider::OpenAI => {
                self.client.create_embedding(
                    "text-embedding-3-small",
                    text,
                    1536
                ).await?
            },
            Provider::Local => {
                self.local_model.encode(text).await?
            }
        };

        // Cache result
        self.cache.write().await.put(text.to_string(), embedding.clone());
        Ok(embedding)
    }
}
```

### 3. Search Executors

#### Full-Text Search
```sql
-- Optimized FTS query with ranking
WITH fts_results AS (
  SELECT
    c.id,
    c.file_id,
    c.symbol_name,
    ts_rank_cd(c.ts_doc, query, 32) as fts_score,
    -- Proximity boost for exact matches
    CASE
      WHEN c.symbol_name ILIKE '%' || $1 || '%' THEN 0.2
      ELSE 0.0
    END as exact_bonus
  FROM maproom.chunks c
  WHERE c.ts_doc @@ plainto_tsquery('simple', $1)
    AND ($2 IS NULL OR c.file_id IN (
      SELECT id FROM maproom.files WHERE repo_id = $2
    ))
)
SELECT
  id,
  (fts_score + exact_bonus) as score,
  ROW_NUMBER() OVER (ORDER BY fts_score + exact_bonus DESC) as rank
FROM fts_results
ORDER BY score DESC
LIMIT $3;
```

#### Vector Search
```sql
-- Optimized vector similarity search
WITH vector_results AS (
  SELECT
    c.id,
    c.file_id,
    c.symbol_name,
    1 - (c.code_embedding <=> $1::vector) as code_similarity,
    1 - (c.text_embedding <=> $1::vector) as text_similarity
  FROM maproom.chunks c
  WHERE c.code_embedding IS NOT NULL
  ORDER BY
    CASE $2
      WHEN 'code' THEN c.code_embedding <=> $1::vector
      WHEN 'text' THEN c.text_embedding <=> $1::vector
      ELSE LEAST(c.code_embedding <=> $1::vector,
                 c.text_embedding <=> $1::vector)
    END
  LIMIT $3
)
SELECT
  id,
  CASE $2
    WHEN 'code' THEN code_similarity
    WHEN 'text' THEN text_similarity
    ELSE (code_similarity * 0.6 + text_similarity * 0.4)
  END as score,
  ROW_NUMBER() OVER (ORDER BY score DESC) as rank
FROM vector_results;
```

#### Graph-Enhanced Ranking
```sql
-- Add graph signals to ranking
WITH edge_counts AS (
  SELECT
    dst_chunk_id as chunk_id,
    COUNT(*) FILTER (WHERE type = 'calls') as callers,
    COUNT(*) FILTER (WHERE type = 'imports') as importers,
    COUNT(*) FILTER (WHERE type = 'test_of') as tests
  FROM maproom.chunk_edges
  GROUP BY dst_chunk_id
)
SELECT
  c.id,
  -- PageRank-like importance score
  COALESCE(
    LOG(2 + e.callers) * 0.3 +
    LOG(2 + e.importers) * 0.2 +
    LOG(2 + e.tests) * 0.1,
    0
  ) as graph_score
FROM maproom.chunks c
LEFT JOIN edge_counts e ON e.chunk_id = c.id;
```

### 4. Score Fusion Engine

#### Reciprocal Rank Fusion (RRF)
```rust
pub struct RRFFusion {
    k: f32,  // Typically 60
}

impl RRFFusion {
    pub fn fuse(&self, result_sets: Vec<RankedResults>) -> Vec<FusedResult> {
        let mut scores: HashMap<ChunkId, f32> = HashMap::new();

        for results in result_sets {
            for (rank, result) in results.iter().enumerate() {
                let rrf_score = 1.0 / (self.k + rank as f32 + 1.0);
                *scores.entry(result.chunk_id).or_insert(0.0) += rrf_score;
            }
        }

        let mut fused: Vec<_> = scores.into_iter()
            .map(|(id, score)| FusedResult { chunk_id: id, score })
            .collect();

        fused.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        fused
    }
}
```

#### Weighted Linear Combination
```rust
pub struct WeightedFusion {
    weights: FusionWeights,
}

#[derive(Clone, Debug)]
pub struct FusionWeights {
    pub fts: f32,      // 0.4
    pub vector: f32,   // 0.35
    pub graph: f32,    // 0.1
    pub recency: f32,  // 0.1
    pub churn: f32,    // 0.05
}

impl WeightedFusion {
    pub fn fuse(&self, signals: SearchSignals) -> f32 {
        self.weights.fts * signals.fts_score +
        self.weights.vector * signals.vector_score +
        self.weights.graph * signals.graph_score +
        self.weights.recency * signals.recency_score +
        self.weights.churn * (1.0 / (1.0 + signals.churn_score))
    }
}
```

### 5. Configuration Schema

```yaml
# maproom-search.yml
search:
  embedding:
    provider: openai  # openai|cohere|local
    model: text-embedding-3-small
    dimension: 1536
    cache_size: 10000
    cache_ttl: 3600

  fusion:
    method: rrf  # rrf|weighted|learned
    rrf_k: 60
    weights:
      fts: 0.40
      vector: 0.35
      graph: 0.10
      recency: 0.10
      churn: 0.05

  performance:
    max_candidates: 1000
    final_limit: 20
    timeout_ms: 100
    parallel_queries: true

  index:
    ivfflat_lists: 200
    ivfflat_probes: 10
    refresh_interval: 300
```

### 6. Query Pipeline

```rust
pub struct SearchPipeline {
    processor: QueryProcessor,
    executors: SearchExecutors,
    fusion: Box<dyn ScoreFusion>,
    reranker: Option<CrossEncoder>,
}

impl SearchPipeline {
    pub async fn search(&self, query: &str, options: SearchOptions)
        -> Result<SearchResults> {

        // Stage 1: Process query
        let processed = self.processor.process(query).await?;

        // Stage 2: Execute parallel searches
        let (fts, vector, graph, signals) = tokio::join!(
            self.executors.fts_search(&processed, options.limit * 3),
            self.executors.vector_search(&processed, options.limit * 3),
            self.executors.graph_search(&processed, options.limit * 2),
            self.executors.signal_search(&processed)
        );

        // Stage 3: Fuse scores
        let fused = self.fusion.fuse(vec![
            fts?, vector?, graph?, signals?
        ]);

        // Stage 4: Optional reranking
        let final_results = if let Some(reranker) = &self.reranker {
            reranker.rerank(query, fused, options.limit).await?
        } else {
            fused.into_iter().take(options.limit).collect()
        };

        Ok(SearchResults {
            query: query.to_string(),
            results: final_results,
            metadata: self.build_metadata(&processed),
        })
    }
}
```

### 7. Caching Strategy

```rust
pub struct SearchCache {
    query_cache: Arc<RwLock<LruCache<String, SearchResults>>>,
    embedding_cache: Arc<RwLock<LruCache<String, Vector>>>,
    stats: Arc<CacheStats>,
}

impl SearchCache {
    pub async fn get_or_compute<F, Fut>(
        &self,
        key: &str,
        compute: F
    ) -> Result<SearchResults>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<SearchResults>>,
    {
        // Check cache
        if let Some(cached) = self.query_cache.read().await.get(key) {
            self.stats.hits.fetch_add(1, Ordering::Relaxed);
            return Ok(cached.clone());
        }

        // Compute and cache
        self.stats.misses.fetch_add(1, Ordering::Relaxed);
        let result = compute().await?;

        self.query_cache.write().await.put(
            key.to_string(),
            result.clone()
        );

        Ok(result)
    }
}
```

## Database Optimizations

### Index Configuration
```sql
-- Partial indices for performance
CREATE INDEX idx_chunks_recent
  ON maproom.chunks (recency_score)
  WHERE recency_score > 0.5;

CREATE INDEX idx_chunks_high_churn
  ON maproom.chunks (churn_score)
  WHERE churn_score > 10;

-- Composite indices for common filters
CREATE INDEX idx_files_repo_worktree
  ON maproom.files (repo_id, worktree_id);

-- Statistics for query planner
ANALYZE maproom.chunks;
ANALYZE maproom.files;
ANALYZE maproom.chunk_edges;
```

### Query Optimization
```sql
-- Materialized view for expensive computations
CREATE MATERIALIZED VIEW maproom.chunk_importance AS
SELECT
  c.id,
  COUNT(DISTINCT e1.src_chunk_id) as in_degree,
  COUNT(DISTINCT e2.dst_chunk_id) as out_degree,
  c.recency_score,
  c.churn_score,
  (
    COUNT(DISTINCT e1.src_chunk_id) * 0.4 +
    c.recency_score * 0.3 +
    (1.0 / (1.0 + c.churn_score)) * 0.3
  ) as importance_score
FROM maproom.chunks c
LEFT JOIN maproom.chunk_edges e1 ON e1.dst_chunk_id = c.id
LEFT JOIN maproom.chunk_edges e2 ON e2.src_chunk_id = c.id
GROUP BY c.id, c.recency_score, c.churn_score;

CREATE INDEX idx_importance ON maproom.chunk_importance(importance_score);
```

## Performance Considerations

### Parallelization
- Execute FTS, vector, and graph queries concurrently
- Use connection pooling for database
- Async embedding generation
- Parallel result assembly

### Caching Layers
1. **Query Cache**: Full results for common queries
2. **Embedding Cache**: Generated embeddings
3. **Score Cache**: Computed fusion scores
4. **Database Cache**: PostgreSQL buffer cache

### Resource Management
```rust
pub struct ResourceLimits {
    max_concurrent_queries: usize,  // 10
    max_embedding_batch: usize,     // 100
    max_result_size: usize,         // 1000
    timeout_ms: u64,                // 100
}
```

## Monitoring & Observability

### Metrics
```rust
pub struct SearchMetrics {
    query_latency: Histogram,
    fusion_time: Histogram,
    cache_hit_rate: Gauge,
    result_count: Histogram,
    error_rate: Counter,
}
```

### Logging
```rust
info!("Search query: '{}', mode: {:?}", query, mode);
debug!("FTS: {} results, Vector: {} results", fts_count, vector_count);
trace!("Fusion scores: {:?}", fusion_scores);
```

### Debugging
```sql
-- Explain query plan
EXPLAIN (ANALYZE, BUFFERS)
SELECT * FROM maproom.chunks
WHERE ts_doc @@ plainto_tsquery('simple', 'search term');

-- Score breakdown
SELECT
  id,
  fts_score,
  vector_score,
  graph_score,
  final_score,
  score_explanation
FROM search_results_debug;
```
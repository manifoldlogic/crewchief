# PERF_OPT Architecture: Performance Optimization

## Optimization Areas

### 1. Database Optimizations

#### Index Strategy
```sql
-- Covering indices for common queries
CREATE INDEX idx_chunks_search ON maproom.chunks(file_id, kind, start_line)
  INCLUDE (symbol_name, preview);

-- Partial indices for hot paths
CREATE INDEX idx_recent_chunks ON maproom.chunks(recency_score)
  WHERE recency_score > 0.7;

-- BRIN indices for large tables
CREATE INDEX idx_files_modified_brin ON maproom.files
  USING BRIN(last_modified);
```

#### Query Optimization
```sql
-- Materialized view for expensive joins
CREATE MATERIALIZED VIEW maproom.chunk_search_view AS
SELECT
  c.*,
  f.relpath,
  f.repo_id,
  f.worktree_id,
  COUNT(e1.src_chunk_id) as importance
FROM maproom.chunks c
JOIN maproom.files f ON c.file_id = f.id
LEFT JOIN maproom.chunk_edges e1 ON e1.dst_chunk_id = c.id
GROUP BY c.id, f.relpath, f.repo_id, f.worktree_id;

-- Refresh strategy
CREATE INDEX idx_search_view_repo ON maproom.chunk_search_view(repo_id);
REFRESH MATERIALIZED VIEW CONCURRENTLY maproom.chunk_search_view;
```

### 2. Parallel Processing

#### Indexing Pipeline
```rust
pub struct ParallelIndexer {
    thread_pool: ThreadPool,
    chunk_size: usize,
}

impl ParallelIndexer {
    pub fn index_files(&self, files: Vec<PathBuf>) -> Result<Stats> {
        let chunks: Vec<Vec<PathBuf>> = files
            .chunks(self.chunk_size)
            .map(|c| c.to_vec())
            .collect();

        let results: Vec<ChunkResult> = chunks
            .par_iter()
            .map(|batch| self.process_batch(batch))
            .collect();

        Ok(self.aggregate_stats(results))
    }
}
```

#### Search Parallelization
```rust
pub async fn parallel_search(&self, query: ProcessedQuery) -> Results {
    let (fts, vector, graph) = tokio::join!(
        self.fts_search(&query),
        self.vector_search(&query),
        self.graph_search(&query)
    );

    self.fuse_results(vec![fts?, vector?, graph?])
}
```

### 3. Caching Strategy

#### Multi-Layer Cache
```rust
pub struct CacheSystem {
    l1_query: Arc<RwLock<LruCache<String, SearchResults>>>,     // 100 entries
    l2_embedding: Arc<RwLock<LruCache<String, Vector>>>,        // 1000 entries
    l3_context: Arc<RwLock<LruCache<u64, ContextBundle>>>,     // 500 entries
}

impl CacheSystem {
    pub async fn get_with_ttl<T>(&self,
                                  key: &str,
                                  ttl: Duration,
                                  compute: impl Future<Output = T>) -> T {
        if let Some(cached) = self.get(key).await {
            if cached.age() < ttl {
                return cached.value;
            }
        }

        let value = compute.await;
        self.set(key, value.clone()).await;
        value
    }
}
```

### 4. Memory Optimizations

#### String Interning
```rust
pub struct StringInterner {
    strings: HashMap<String, Arc<str>>,
}

impl StringInterner {
    pub fn intern(&mut self, s: String) -> Arc<str> {
        self.strings.entry(s.clone())
            .or_insert_with(|| Arc::from(s.as_str()))
            .clone()
    }
}
```

#### Vector Quantization
```rust
pub fn quantize_embedding(embedding: &[f32]) -> Vec<i8> {
    embedding.iter()
        .map(|&v| (v * 127.0).round() as i8)
        .collect()
}

pub fn dequantize_embedding(quantized: &[i8]) -> Vec<f32> {
    quantized.iter()
        .map(|&v| v as f32 / 127.0)
        .collect()
}
```

### 5. Connection Management

```rust
pub struct ConnectionPool {
    pool: bb8::Pool<PostgresConnectionManager>,
}

impl ConnectionPool {
    pub fn new(size: u32) -> Self {
        let manager = PostgresConnectionManager::new(config);
        let pool = bb8::Pool::builder()
            .max_size(size)
            .min_idle(Some(size / 4))
            .max_lifetime(Some(Duration::from_secs(30 * 60)))
            .idle_timeout(Some(Duration::from_secs(10 * 60)))
            .build(manager)
            .await?;

        Self { pool }
    }
}
```

## Performance Monitoring

### Metrics Collection
```rust
pub struct PerformanceMetrics {
    indexing_rate: Histogram,
    search_latency: Histogram,
    cache_hit_rate: Gauge,
    memory_usage: Gauge,
    query_throughput: Counter,
}
```

### Profiling Integration
```rust
#[cfg(feature = "profiling")]
pub fn profile_operation<T>(name: &str, op: impl FnOnce() -> T) -> T {
    puffin::profile_scope!(name);
    op()
}
```

## Configuration

```yaml
performance:
  indexing:
    parallel_workers: 8
    batch_size: 50
    max_file_size: 10485760  # 10MB

  database:
    pool_size: 20
    statement_timeout: 5000
    work_mem: "256MB"

  cache:
    query_cache_size: 100
    embedding_cache_size: 1000
    ttl_seconds: 3600

  monitoring:
    enabled: true
    sample_rate: 0.1
    export_interval: 60
```
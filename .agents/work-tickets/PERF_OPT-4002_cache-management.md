# Ticket: PERF_OPT-4002: Cache Management

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- performance-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Implement cache management features including TTL configuration, eviction policies, cache warming strategies, and invalidation logic to maintain cache effectiveness and data freshness.

## Background
After implementing cache systems in PERF_OPT-4001, we need proper management to ensure caches remain effective over time. PERF_OPT_PLAN.md (lines 86-90) identifies key management tasks: TTL configuration, eviction policies, cache warming, and invalidation logic.

Without proper management, caches can serve stale data, grow unbounded, or become ineffective due to poor eviction policies. This ticket ensures caches remain performant and correct.

## Acceptance Criteria
- [ ] TTL configuration implemented per cache layer
- [ ] Eviction policies implemented (LRU, TTL-based, size-based)
- [ ] Cache warming strategy implemented for critical queries
- [ ] Invalidation logic implemented for file changes and re-indexing
- [ ] No stale data served (verified by tests)
- [ ] Cache effectiveness monitoring shows sustained >60% hit rate
- [ ] CLI commands for cache inspection and management

## Technical Requirements

### TTL Configuration
Implement configurable TTL per cache layer:
```rust
pub struct CacheConfig {
    query_ttl: Duration,        // 1 hour default
    embedding_ttl: Duration,    // 24 hours default
    context_ttl: Duration,      // 30 minutes default
    parse_tree_ttl: Option<Duration>,  // None = until invalidated
}
```

Configuration (PERF_OPT_ARCHITECTURE.md lines 200-203):
```yaml
cache:
  query_cache_size: 100
  embedding_cache_size: 1000
  ttl_seconds: 3600
```

Per-layer TTL:
```yaml
cache:
  query:
    size: 100
    ttl: 3600  # 1 hour
  embedding:
    size: 1000
    ttl: 86400  # 24 hours
  context:
    size: 500
    ttl: 1800  # 30 minutes
```

### Eviction Policies
Implement multiple eviction strategies:

1. **LRU (Least Recently Used)** - Default for all caches
2. **TTL-based** - Expire entries after TTL
3. **Size-based** - Evict when memory limit reached
4. **Access-based** - Evict least frequently used (LFU variant)

```rust
pub enum EvictionPolicy {
    Lru,
    Ttl(Duration),
    Size(usize),
    AccessCount(u64),
}

impl CacheSystem {
    pub fn evict(&mut self, policy: EvictionPolicy) {
        match policy {
            EvictionPolicy::Lru => self.evict_lru(),
            EvictionPolicy::Ttl(ttl) => self.evict_expired(ttl),
            EvictionPolicy::Size(max_size) => self.evict_to_size(max_size),
            EvictionPolicy::AccessCount(min_count) => self.evict_low_access(min_count),
        }
    }
}
```

### Cache Warming
Pre-populate caches with common queries:

```rust
pub struct CacheWarmer {
    cache: Arc<CacheSystem>,
    warm_queries: Vec<String>,
}

impl CacheWarmer {
    pub async fn warm(&self) -> Result<()> {
        for query in &self.warm_queries {
            let results = self.execute_search(query).await?;
            self.cache.l1_query.write().await.put(
                query.clone(),
                CacheEntry::new(results)
            );
        }
        Ok(())
    }
}
```

Warming strategies:
- On startup: Warm most common queries
- On idle: Background warming of predicted queries
- On invalidation: Re-warm affected queries
- On schedule: Refresh high-value cache entries

### Invalidation Logic
Implement smart invalidation:

```rust
pub struct CacheInvalidator {
    cache: Arc<CacheSystem>,
    file_watcher: FileWatcher,
}

impl CacheInvalidator {
    // Invalidate on file change
    pub async fn on_file_changed(&self, path: &Path) {
        // Invalidate parse tree cache
        self.cache.invalidate_parse_tree(path).await;

        // Invalidate context bundles containing this file
        let affected_chunks = self.get_chunks_in_file(path).await;
        for chunk_id in affected_chunks {
            self.cache.invalidate_context(chunk_id).await;
        }
    }

    // Invalidate on re-index
    pub async fn on_reindex(&self, repo_id: i64) {
        self.cache.clear_all().await;
        info!("Cache cleared for repo re-index: {}", repo_id);
    }

    // Invalidate specific queries
    pub async fn invalidate_query(&self, pattern: &str) {
        let mut cache = self.cache.l1_query.write().await;
        cache.retain(|key, _| !key.contains(pattern));
    }
}
```

Invalidation triggers:
- File modification detected
- Repository re-indexed
- Manual invalidation via CLI
- Database schema change
- Configuration change

### Cache Inspection CLI
Add CLI commands:
```bash
# Show cache statistics
maproom cache stats

# Show cache entries
maproom cache list [--layer query|embedding|context|parse]

# Clear cache
maproom cache clear [--layer query|embedding|context|parse|all]

# Warm cache
maproom cache warm [--queries queries.txt]

# Invalidate specific entries
maproom cache invalidate --pattern "search_term"
```

### Background Maintenance
Implement background task for cache maintenance:
```rust
pub async fn cache_maintenance_loop(cache: Arc<CacheSystem>) {
    let mut interval = tokio::time::interval(Duration::from_secs(60));

    loop {
        interval.tick().await;

        // Evict expired entries
        cache.evict_all_expired().await;

        // Log statistics
        let stats = cache.get_stats().await;
        info!("Cache stats: hit_rate={:.2}%, size={}MB",
              stats.hit_rate() * 100.0,
              stats.memory_usage_mb());

        // Alert if hit rate is low
        if stats.hit_rate() < 0.4 {
            warn!("Cache hit rate below 40%, consider adjusting TTL or size");
        }
    }
}
```

## Implementation Notes

### Memory Monitoring
Track memory usage per cache:
```rust
impl CacheSystem {
    pub fn memory_usage(&self) -> MemoryStats {
        MemoryStats {
            l1_bytes: self.l1_query.read().await.memory_usage(),
            l2_bytes: self.l2_embedding.read().await.memory_usage(),
            l3_bytes: self.l3_context.read().await.memory_usage(),
            total_bytes: /* sum */,
        }
    }
}
```

Alert when approaching 500MB limit (PERF_OPT_PLAN.md line 124).

### TTL Cleanup
Periodically scan for expired entries:
```rust
pub fn cleanup_expired(&mut self, ttl: Duration) {
    self.cache.retain(|_, entry| !entry.is_expired(ttl));
}
```

Run cleanup:
- On access (lazy cleanup)
- Periodically in background (eager cleanup)
- When memory pressure detected

### Smart Invalidation
Only invalidate affected entries:
- File change → Invalidate parse tree + contexts containing file
- Query pattern → Invalidate matching queries only
- Full re-index → Clear all caches

### Cache Warming Strategies
1. **Startup warming**: Load common queries from config
2. **Predictive warming**: Analyze query patterns, pre-warm likely queries
3. **Scheduled warming**: Refresh popular entries before TTL expiration
4. **User-defined warming**: Allow warming specific queries via CLI

### Performance Impact
Ensure management doesn't hurt performance:
- Background cleanup, not on critical path
- Lazy eviction when possible
- Batch invalidations
- Lock-free statistics where possible

## Dependencies
- **PERF_OPT-4001** - Requires cache systems to be implemented
- **PERF_OPT-1001** - Uses metrics system for cache monitoring
- File watching library for invalidation triggers
- tokio for background tasks

## Risk Assessment
- **Risk**: Aggressive eviction may reduce hit rate
  - **Mitigation**: Monitor hit rate, tune TTL and eviction policies based on metrics
- **Risk**: Cache warming may slow down startup
  - **Mitigation**: Async warming, don't block startup, warm in background
- **Risk**: Invalidation bugs may serve stale data
  - **Mitigation**: Comprehensive testing, conservative invalidation (when in doubt, invalidate)
- **Risk**: Background maintenance may impact performance
  - **Mitigation**: Low-priority background tasks, rate limiting, minimal locking

## Files/Packages Affected
- `crates/maproom/src/cache/config.rs` - Cache configuration
- `crates/maproom/src/cache/eviction.rs` - Eviction policies
- `crates/maproom/src/cache/warming.rs` - Cache warming
- `crates/maproom/src/cache/invalidation.rs` - Invalidation logic
- `crates/maproom/src/cache/maintenance.rs` - Background maintenance
- `crates/maproom/src/cli/cache.rs` - Cache CLI commands
- `crates/maproom/src/main.rs` - Spawn cache maintenance task
- `crates/maproom/tests/cache_management.rs` - New test file for cache management

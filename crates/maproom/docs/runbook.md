# Maproom Hybrid Search Troubleshooting Runbook

This runbook provides step-by-step procedures for diagnosing and resolving common issues with the Maproom hybrid search system.

## Table of Contents

1. [Quick Reference](#quick-reference)
2. [High Latency](#high-latency)
3. [High Error Rate](#high-error-rate)
4. [Low Cache Hit Rate](#low-cache-hit-rate)
5. [Slow Fusion](#slow-fusion)
6. [System Down](#system-down)
7. [Low Query Rate](#low-query-rate)
8. [Low Result Count](#low-result-count)
9. [Database Issues](#database-issues)
10. [Memory Issues](#memory-issues)

## Quick Reference

### Alert Response Matrix

| Alert | Severity | Response Time | First Action |
|-------|----------|---------------|--------------|
| SearchLatencyP95Critical | Critical | < 15 min | Check database + logs |
| SearchErrorRateCritical | Critical | < 15 min | Check logs for error types |
| SearchSystemDown | Critical | < 5 min | Check process status |
| SearchLatencyP95High | Warning | < 1 hour | Monitor trends |
| SearchErrorRateHigh | Warning | < 1 hour | Review error patterns |
| SearchCacheHitRateLow | Warning | < 4 hours | Review cache config |
| SearchFusionTimeHigh | Warning | < 2 hours | Check fusion strategy |

### Emergency Contacts

- **On-call Engineer**: [Your team's contact info]
- **Database Team**: [Database support contact]
- **Infrastructure Team**: [Infrastructure support contact]

### Quick Diagnostic Commands

```bash
# Check process status
ps aux | grep maproom

# Check metrics endpoint
curl http://localhost:9090/metrics | grep maproom_search

# Check recent errors in logs
grep "ERROR" logs/search.log | tail -n 50

# Check database connectivity
psql -h localhost -U maproom -d maproom -c "SELECT 1"

# Check Prometheus targets
curl http://localhost:9090/api/v1/targets
```

## High Latency

### Alert: SearchLatencyP95High / SearchLatencyP95Critical

**Symptoms**:
- p95 query latency > 50ms (warning) or > 100ms (critical)
- Users reporting slow search responses
- Dashboard shows latency spike

### Diagnosis

#### Step 1: Identify the Scope
```promql
# Check which mode is affected
histogram_quantile(0.95, rate(maproom_search_query_latency_seconds_bucket{status="success"}[5m]))
```

#### Step 2: Check Component Breakdown

Look at structured logs for timing breakdown:
```bash
grep "Search exceeded 50ms" logs/search.log | tail -n 20
```

Logs will show:
```
Search exceeded 50ms target: 87.23ms (processing: 5.12ms, search: 65.34ms, fusion: 3.45ms, assembly: 13.32ms)
```

#### Step 3: Identify Bottleneck

- **Processing time high** (> 10ms): Embedding service slow
- **Search time high** (> 40ms): Database queries slow
- **Fusion time high** (> 10ms): Score fusion inefficient
- **Assembly time high** (> 15ms): Chunk detail fetching slow

### Resolution

#### If Database Queries Are Slow (search time > 40ms)

1. **Check database load**:
   ```sql
   SELECT * FROM pg_stat_activity WHERE state = 'active';
   ```

2. **Check for slow queries**:
   ```sql
   SELECT query, mean_exec_time, calls
   FROM pg_stat_statements
   ORDER BY mean_exec_time DESC
   LIMIT 10;
   ```

3. **Check index health**:
   ```sql
   SELECT schemaname, tablename, indexname, idx_scan, idx_tup_read, idx_tup_fetch
   FROM pg_stat_user_indexes
   WHERE schemaname = 'maproom'
   ORDER BY idx_scan;
   ```

4. **Actions**:
   - Run `ANALYZE` on search tables
   - Check for missing indexes
   - Restart connection pool: `pg_ctl reload`
   - Scale database if sustained high load

#### If Embedding Service Is Slow (processing time > 10ms)

1. **Check embedding service status**:
   ```bash
   curl http://embedding-service:8080/health
   ```

2. **Check embedding cache hit rate**:
   - Review logs for cache performance
   - Increase embedding cache size if low hit rate

3. **Actions**:
   - Restart embedding service
   - Scale embedding service replicas
   - Check network latency to embedding service

#### If Assembly Is Slow (assembly time > 15ms)

1. **Check chunk fetch query performance**:
   ```sql
   EXPLAIN ANALYZE
   SELECT c.id, c.file_id, f.relpath, c.symbol_name, c.kind::text,
          c.start_line, c.end_line, c.preview
   FROM maproom.chunks c
   JOIN maproom.files f ON f.id = c.file_id
   WHERE c.id = ANY(ARRAY[1, 2, 3, 4, 5]);
   ```

2. **Actions**:
   - Ensure indexes on `chunks.id` and `files.id` exist
   - Consider caching chunk details
   - Batch chunk fetching more efficiently

#### If Fusion Is Slow (fusion time > 10ms)

See [Slow Fusion](#slow-fusion) section.

### Prevention

- **Regular Index Maintenance**: Weekly `ANALYZE` and `VACUUM`
- **Connection Pool Tuning**: Adjust pool size based on load
- **Cache Warming**: Pre-populate caches during deployment
- **Load Testing**: Regular performance testing under realistic load

## High Error Rate

### Alert: SearchErrorRateHigh / SearchErrorRateCritical

**Symptoms**:
- Error rate > 1% (warning) or > 5% (critical)
- Users reporting failed searches
- Dashboard shows error spike

### Diagnosis

#### Step 1: Identify Error Types
```promql
# Errors by type
sum by (error_type) (rate(maproom_search_errors_total[5m]))
```

#### Step 2: Check Logs for Details
```bash
# Recent errors
grep "ERROR" logs/search.log | tail -n 100

# Errors by type
grep "ERROR" logs/search.log | grep "query_processing" | tail -n 20
grep "ERROR" logs/search.log | grep "search_execution" | tail -n 20
grep "ERROR" logs/search.log | grep "database" | tail -n 20
```

### Resolution by Error Type

#### query_processing Errors

**Common Causes**:
- Embedding service unavailable
- Invalid query format
- Tokenization failures

**Actions**:
1. Check embedding service health
2. Review problematic queries in logs
3. Check for malformed input validation

**Quick Fix**:
```bash
# Restart embedding service
systemctl restart embedding-service
```

#### search_execution Errors

**Common Causes**:
- Database connection failures
- Query timeout
- Missing database objects

**Actions**:
1. Check database connectivity:
   ```bash
   psql -h localhost -U maproom -d maproom -c "SELECT 1"
   ```

2. Check connection pool:
   ```sql
   SELECT count(*) FROM pg_stat_activity
   WHERE datname = 'maproom';
   ```

3. Review query timeouts in configuration

**Quick Fix**:
```bash
# Restart connection pool
# Application restart may be required
```

#### database Errors

**Common Causes**:
- Database overload
- Connection pool exhausted
- Database locks/deadlocks

**Actions**:
1. Check database load:
   ```sql
   SELECT * FROM pg_stat_activity WHERE state = 'active';
   ```

2. Check for locks:
   ```sql
   SELECT * FROM pg_locks WHERE NOT granted;
   ```

3. Check connection pool stats:
   - Review deadpool metrics if available

**Quick Fix**:
```sql
-- Kill long-running queries if necessary
SELECT pg_terminate_backend(pid)
FROM pg_stat_activity
WHERE state = 'active'
  AND query_start < NOW() - INTERVAL '5 minutes'
  AND datname = 'maproom';
```

#### fusion Errors

**Common Causes**:
- Invalid fusion weights
- Empty result sets
- Score calculation errors

**Actions**:
1. Review fusion configuration
2. Check result set sizes in logs
3. Validate fusion weights sum to expected value

### Prevention

- **Input Validation**: Strict query validation
- **Circuit Breakers**: Implement circuit breakers for external dependencies
- **Connection Pool Monitoring**: Alert on pool exhaustion
- **Graceful Degradation**: Return partial results on non-critical errors

## Low Cache Hit Rate

### Alert: SearchCacheHitRateLow / SearchCacheHitRateCriticallyLow

**Symptoms**:
- Cache hit rate < 50% (warning) or < 20% (critical)
- Higher than normal latency
- Increased database load

### Diagnosis

#### Step 1: Check Current Hit Rate
```promql
maproom_search_cache_hit_rate
```

#### Step 2: Analyze Query Patterns

Check logs for query diversity:
```bash
# Sample of recent queries
grep "Starting search pipeline" logs/search.log | tail -n 100 | awk -F"'" '{print $2}' | sort | uniq -c | sort -rn | head -n 20
```

#### Step 3: Check Cache Statistics

If cache stats are exposed:
- Cache size vs capacity
- Eviction rate
- Expiration rate

### Resolution

#### If Query Diversity Is High

**Symptom**: Many unique queries, few repeats

**Actions**:
1. **Increase cache size**:
   ```rust
   // In configuration
   let cache = SearchCache::new(2000); // Increase from 1000
   ```

2. **Consider query normalization**:
   - Normalize whitespace
   - Lowercase standardization
   - Remove common stop words

3. **Implement query clustering**:
   - Cache similar queries together
   - Use fuzzy matching for cache lookups

#### If TTL Is Too Short

**Symptom**: High expiration rate

**Actions**:
1. **Extend TTL**:
   ```rust
   let cache = SearchCache::with_ttl(1000, 7200); // 2 hours instead of 1
   ```

2. **Implement adaptive TTL**:
   - Longer TTL for popular queries
   - Shorter TTL for rare queries

#### If Cache Size Is Too Small

**Symptom**: High eviction rate

**Actions**:
1. **Increase cache capacity**:
   - Monitor memory usage
   - Scale gradually: 1000 → 2000 → 5000

2. **Optimize cache entry size**:
   - Store only essential data
   - Compress large results

### Prevention

- **Baseline Measurement**: Measure hit rate in staging with production-like traffic
- **Cache Warming**: Pre-populate cache with common queries
- **Query Analysis**: Regular analysis of query patterns
- **Capacity Planning**: Plan cache size based on expected traffic

## Slow Fusion

### Alert: SearchFusionTimeHigh

**Symptoms**:
- p95 fusion time > 10ms
- Fusion contributing significantly to overall latency
- Dashboard shows fusion time spike

### Diagnosis

#### Step 1: Check Fusion Time
```promql
histogram_quantile(0.95, rate(maproom_search_fusion_time_seconds_bucket[5m]))
```

#### Step 2: Check Result Set Sizes

Look for large result sets being fused:
```bash
grep "Score fusion completed" logs/search.log | tail -n 50
```

#### Step 3: Identify Fusion Strategy

Check which strategy is slow:
```promql
histogram_quantile(0.95, rate(maproom_search_fusion_time_seconds_bucket[5m])) by (strategy)
```

### Resolution

#### If Processing Large Result Sets

**Symptom**: Fusion time correlates with result count

**Actions**:
1. **Reduce intermediate result count**:
   ```rust
   // In search pipeline
   options.limit * 2  // Instead of options.limit * 3
   ```

2. **Implement early termination**:
   - Stop fusion when top-k results are stable
   - Use score thresholds to filter early

3. **Optimize data structures**:
   - Use more efficient score lookup
   - Pre-allocate result vectors

#### If RRF Fusion Is Slow

**Symptom**: RRF strategy shows high fusion time

**Actions**:
1. **Optimize rank computation**:
   - Cache rank lookups
   - Use efficient sorting algorithms

2. **Adjust k parameter**:
   ```rust
   // Lower k value for faster RRF
   let rrf = RRFFusion::with_k(30.0); // Instead of 60.0
   ```

#### If Weighted Fusion Is Slow

**Symptom**: Basic weighted fusion shows high fusion time

**Actions**:
1. **Profile score calculation**:
   - Check for expensive operations in score computation
   - Optimize hot paths

2. **Parallelize fusion** (if not already):
   - Process result sources in parallel
   - Use rayon for parallel iteration

### Prevention

- **Benchmark Fusion**: Regular benchmarking of fusion strategies
- **Profile Regularly**: Profile fusion code for hotspots
- **Optimize Data Structures**: Use efficient HashMap/BTreeMap
- **Test with Production Data**: Test fusion with realistic result set sizes

## System Down

### Alert: SearchSystemDown

**Symptoms**:
- No queries processed in 5+ minutes
- Search service unreachable
- Dashboard shows flat lines

### Diagnosis

#### Step 1: Check Process Status
```bash
# Check if process is running
ps aux | grep maproom

# Check systemd status (if using systemd)
systemctl status maproom
```

#### Step 2: Check Logs
```bash
# Recent logs
tail -n 100 logs/search.log

# Check for crash/panic
grep -i "panic\|fatal\|crashed" logs/search.log
```

#### Step 3: Check Dependencies
```bash
# Database connectivity
psql -h localhost -U maproom -d maproom -c "SELECT 1"

# Embedding service
curl http://embedding-service:8080/health

# Network connectivity
ping <service-host>
```

### Resolution

#### If Process Crashed

**Actions**:
1. **Restart service**:
   ```bash
   systemctl restart maproom
   ```

2. **Check for core dumps**:
   ```bash
   ls -l /var/crash/
   ```

3. **Review panic backtrace in logs**

4. **If persists**: Roll back to previous version

#### If Database Is Down

**Actions**:
1. **Check database status**:
   ```bash
   systemctl status postgresql
   ```

2. **Start database if needed**:
   ```bash
   systemctl start postgresql
   ```

3. **Verify connectivity**:
   ```bash
   psql -h localhost -U maproom -d maproom
   ```

#### If Embedding Service Is Down

**Actions**:
1. **Check embedding service**:
   ```bash
   systemctl status embedding-service
   ```

2. **Restart if needed**:
   ```bash
   systemctl restart embedding-service
   ```

### Prevention

- **Health Checks**: Implement regular health checks
- **Auto-restart**: Configure systemd auto-restart
- **Dependency Monitoring**: Monitor all dependencies
- **Graceful Shutdown**: Implement proper shutdown handlers

## Low Query Rate

### Alert: SearchQueryRateDropped

**Symptoms**:
- Query rate dropped > 50% compared to baseline
- May indicate upstream issues or service degradation

### Diagnosis

#### Step 1: Check Current Query Rate
```promql
rate(maproom_search_queries_total[5m])
```

#### Step 2: Compare with Historical Baseline
```promql
rate(maproom_search_queries_total[5m]) / rate(maproom_search_queries_total[5m] offset 1h)
```

#### Step 3: Check Upstream Services

- Check load balancer health
- Check API gateway metrics
- Check client application health

### Resolution

#### If Service Is Healthy But No Traffic

**Actions**:
1. **Check upstream services**:
   - Load balancer configuration
   - API gateway routing
   - DNS resolution

2. **Check network connectivity**:
   ```bash
   netstat -an | grep :9090
   ```

3. **Review recent deployments**:
   - API changes breaking clients
   - Configuration changes affecting routing

#### If This Is Expected (e.g., Off-hours)

**Actions**:
1. **Adjust alert thresholds** for time-of-day patterns
2. **Use anomaly detection** instead of fixed thresholds

### Prevention

- **Upstream Monitoring**: Monitor client applications
- **Time-aware Alerts**: Adjust thresholds for known patterns
- **Dependency Tracking**: Track all upstream dependencies

## Low Result Count

### Alert: SearchResultCountLow

**Symptoms**:
- Median result count < 1
- Users reporting "no results" frequently
- May indicate indexing issues

### Diagnosis

#### Step 1: Check Result Count Distribution
```promql
histogram_quantile(0.50, rate(maproom_search_result_count_bucket[5m]))
```

#### Step 2: Check Index Health

```sql
-- Check chunk count
SELECT COUNT(*) FROM maproom.chunks;

-- Check recent index updates
SELECT MAX(indexed_at) FROM maproom.files;

-- Check file count by repo
SELECT repo_id, COUNT(*)
FROM maproom.files
GROUP BY repo_id;
```

#### Step 3: Check Query Patterns

```bash
# Queries with no results
grep "returned 0 results" logs/search.log | tail -n 50
```

### Resolution

#### If Index Is Empty/Outdated

**Actions**:
1. **Check indexer status**:
   ```bash
   systemctl status maproom-indexer
   ```

2. **Trigger reindex**:
   ```bash
   maproom index --repo <repo-id> --force
   ```

3. **Verify index completion**:
   ```sql
   SELECT repo_id, COUNT(*), MAX(indexed_at)
   FROM maproom.files
   GROUP BY repo_id;
   ```

#### If Queries Are Too Restrictive

**Actions**:
1. **Review query expansion settings**
2. **Check tokenization logic**
3. **Adjust FTS query construction**

#### If Vector Embeddings Are Missing

**Actions**:
1. **Check embedding generation**:
   ```sql
   SELECT COUNT(*) FROM maproom.chunks WHERE embedding IS NULL;
   ```

2. **Regenerate missing embeddings**:
   ```bash
   maproom generate-embeddings --backfill
   ```

### Prevention

- **Index Monitoring**: Monitor index freshness
- **Automated Reindexing**: Schedule regular reindexing
- **Query Testing**: Test queries against staging index
- **Embedding Pipeline**: Ensure embedding generation is working

## Database Issues

### Common Database Problems

#### Connection Pool Exhausted

**Symptoms**:
- "connection pool exhausted" errors
- Increased query latency
- Timeouts

**Diagnosis**:
```sql
SELECT count(*) as total_connections,
       count(*) FILTER (WHERE state = 'active') as active,
       count(*) FILTER (WHERE state = 'idle') as idle
FROM pg_stat_activity
WHERE datname = 'maproom';
```

**Resolution**:
1. **Increase pool size** (carefully):
   ```rust
   let pool_config = deadpool_postgres::Config {
       pool: Some(PoolConfig {
           max_size: 20,  // Increase from 10
           ..Default::default()
       }),
       ..Default::default()
   };
   ```

2. **Kill long-running queries**:
   ```sql
   SELECT pg_terminate_backend(pid)
   FROM pg_stat_activity
   WHERE state = 'active'
     AND query_start < NOW() - INTERVAL '5 minutes';
   ```

3. **Review query efficiency**

#### Slow Queries

**Symptoms**:
- Queries taking > 100ms
- Database CPU high
- Increased latency

**Diagnosis**:
```sql
-- Enable query logging temporarily
ALTER DATABASE maproom SET log_min_duration_statement = 100;

-- Check slow queries
SELECT query, mean_exec_time, calls
FROM pg_stat_statements
ORDER BY mean_exec_time DESC
LIMIT 10;
```

**Resolution**:
1. **Run ANALYZE**:
   ```sql
   ANALYZE maproom.chunks;
   ANALYZE maproom.files;
   ANALYZE maproom.chunk_edges;
   ```

2. **Check missing indexes**:
   ```sql
   SELECT schemaname, tablename, attname, n_distinct, correlation
   FROM pg_stats
   WHERE schemaname = 'maproom'
   ORDER BY abs(correlation) DESC;
   ```

3. **Optimize query plans**:
   - Review EXPLAIN ANALYZE output
   - Consider materialized views for common queries

## Memory Issues

### High Memory Usage

**Symptoms**:
- Process memory > 2GB
- OOM (Out of Memory) kills
- Increased swap usage

**Diagnosis**:
```bash
# Check process memory
ps aux | grep maproom | awk '{print $6}'

# Check system memory
free -h

# Check for memory leaks (if running locally)
valgrind --leak-check=full ./target/release/maproom
```

**Resolution**:

1. **Check cache sizes**:
   - Reduce search cache size
   - Reduce embedding cache size

2. **Check connection pool**:
   - Reduce max connections if possible

3. **Profile memory usage**:
   ```bash
   # Use heaptrack or similar
   heaptrack ./target/release/maproom
   ```

4. **Restart service** (temporary fix):
   ```bash
   systemctl restart maproom
   ```

### Prevention

- **Memory Limits**: Set systemd memory limits
- **Regular Restarts**: Schedule weekly restarts to clear caches
- **Profile Regularly**: Regular memory profiling
- **Monitor Trends**: Track memory usage over time

## Escalation Procedures

### When to Escalate

Escalate to senior engineer if:
- Issue persists after following runbook
- Data loss is imminent
- Multiple services affected
- Security concern identified

### How to Escalate

1. **Gather diagnostic information**:
   - Alert details
   - Log snippets
   - Metrics screenshots
   - Steps already taken

2. **Create incident ticket** with:
   - Severity level
   - Impact assessment
   - Timeline
   - Actions taken

3. **Contact on-call**:
   - Use escalation contact list
   - Provide incident ticket reference
   - Stay available for questions

## Post-Incident Actions

After resolving an incident:

1. **Document resolution** in incident ticket
2. **Update runbook** with new learnings
3. **Schedule post-mortem** (for critical incidents)
4. **Implement preventive measures**
5. **Update monitoring/alerts** if needed

## Additional Resources

- **Monitoring Guide**: See `monitoring_guide.md`
- **Architecture**: See `HYBRID_SEARCH_ARCHITECTURE.md`
- **Database Schema**: See `migrations/` directory
- **Log Locations**: `/var/log/maproom/` or configured path

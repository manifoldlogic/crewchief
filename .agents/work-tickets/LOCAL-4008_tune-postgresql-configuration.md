# Ticket: LOCAL-4008: Tune PostgreSQL Configuration

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- database-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Optimize PostgreSQL configuration based on stress test results from LOCAL-4007 to improve query performance, reduce resource usage, and ensure long-term stability for Maproom's semantic code search features.

## Background
After comprehensive stress testing (LOCAL-4007), we need to tune PostgreSQL settings to optimize performance for Maproom's specific workload patterns:
- High-volume vector similarity search (pgvector)
- Full-text search (FTS) queries
- Concurrent read-heavy operations
- Background embedding generation

PostgreSQL's default configuration is conservative and not optimized for our specific use case (SSD storage, 4-8GB RAM systems, vector operations, FTS workloads). Proper tuning can significantly reduce query latency and improve system responsiveness.

This is the final optimization phase - making queries as fast as possible while staying within documented system requirements.

## Acceptance Criteria
- [ ] postgresql.conf updated with optimized settings based on stress test analysis
- [ ] Query performance improved from baseline (measured with EXPLAIN ANALYZE)
- [ ] Vector search latency reduced or maintained (no regression)
- [ ] FTS search latency reduced or maintained (no regression)
- [ ] All settings documented with clear rationale in comments
- [ ] No negative side effects (crashes, out-of-memory errors, connection failures)
- [ ] Settings appropriate for documented system requirements (4-8GB RAM, SSD storage)
- [ ] Stress test results show improvement after tuning (re-run LOCAL-4007 tests)

## Technical Requirements

### 1. Memory Settings Optimization
Based on LOCAL_ARCHITECTURE.md lines 546-559:
- Tune `shared_buffers` (currently 256MB) - typically 25% of system RAM
- Adjust `effective_cache_size` (currently 1GB) - typically 50-75% of system RAM
- Optimize `work_mem` (currently 4MB) - balance between sort performance and memory usage
- Configure `maintenance_work_mem` (currently 64MB) - affects VACUUM, CREATE INDEX

### 2. Query Planner Tuning
- Verify `random_page_cost` (currently 1.1 for SSD) - should be low for SSD storage
- Set `effective_io_concurrency` (currently 200) - appropriate for SSD
- Adjust `default_statistics_target` (currently 100) - affects query plan quality

### 3. WAL and Checkpoint Configuration
- Configure `wal_buffers` (currently 16MB)
- Tune `checkpoint_completion_target` (currently 0.9)
- Set appropriate `min_wal_size` and `max_wal_size`

### 4. Connection Management
- Review `max_connections` (currently 100)
- Configure connection timeout settings
- Ensure settings align with connection pool usage

### 5. Vector Index Optimization
- Tune ivfflat `lists` parameter (currently 100)
- Consider HNSW indexes if available in pgvector version
- Optimize index build parameters for faster indexing

### 6. Maintenance Settings
- Configure autovacuum settings for vector tables
- Set appropriate analyze frequency
- Define index rebuild/maintenance strategy

## Implementation Notes

### Tuning Process
1. **Analyze Stress Test Results**: Review LOCAL-4007 output to identify bottlenecks
   - Slow query logs
   - Wait events from pg_stat_activity
   - Cache hit ratios from pg_stat_database
   - Index usage statistics

2. **Identify Slow Queries**: Use EXPLAIN ANALYZE on representative queries
   - Vector similarity search queries
   - FTS search queries
   - Combined hybrid search queries
   - Background indexing queries

3. **Review PostgreSQL Statistics Views**:
   - pg_stat_bgwriter - checkpoint and background writer activity
   - pg_stat_database - database-wide statistics
   - pg_stat_user_tables - table access patterns
   - pg_stat_user_indexes - index usage patterns
   - pg_statio_* - I/O statistics

4. **Incremental Tuning**: Adjust configuration one section at a time
   - Make changes to postgresql.conf
   - Reload configuration (pg_reload_conf() or service restart if needed)
   - Re-run representative tests
   - Measure impact before proceeding

5. **Validation**: Re-run stress tests to validate improvements
   - Compare metrics before/after tuning
   - Ensure no regressions in any workload pattern
   - Monitor for stability issues

6. **Documentation**: Document final settings with rationale
   - Inline comments in postgresql.conf
   - Summary of changes and expected impact
   - Baseline vs tuned performance metrics

### Reference Resources
- PostgreSQL Tuning Guide: https://wiki.postgresql.org/wiki/Tuning_Your_PostgreSQL_Server
- pgvector Performance: https://github.com/pgvector/pgvector#performance
- PGTune Configuration Generator: https://pgtune.leopard.in.ua/
- PostgreSQL Configuration Documentation: https://www.postgresql.org/docs/current/runtime-config.html

### Configuration File Location
- Primary config: `/etc/postgresql/postgresql.conf` (or Docker volume mount)
- May need to be integrated into Docker image build or mounted as volume
- Consider environment variable overrides for key settings

### Testing After Changes
```bash
# Reload configuration without restart (if possible)
docker exec maproom-postgres psql -U maproom -c "SELECT pg_reload_conf();"

# Or restart PostgreSQL container
docker-compose restart postgres

# Verify settings applied
docker exec maproom-postgres psql -U maproom -c "SHOW shared_buffers;"
docker exec maproom-postgres psql -U maproom -c "SHOW effective_cache_size;"
```

## Dependencies
- **LOCAL-4007**: Stress test Maproom under load
  - Required for identifying performance bottlenecks
  - Provides baseline metrics for comparison
  - Identifies slow queries and resource constraints

## Risk Assessment
- **Risk**: Aggressive memory settings cause out-of-memory errors on 4GB systems
  - **Mitigation**: Test on minimum spec system (4GB RAM), provide different config profiles for different system sizes, document memory requirements clearly

- **Risk**: Configuration changes cause PostgreSQL startup failures
  - **Mitigation**: Validate syntax before deployment, test in local environment first, keep backup of working configuration, document rollback procedure

- **Risk**: Tuning optimizes for stress test patterns but degrades real-world usage
  - **Mitigation**: Use diverse query patterns in testing, validate against actual Maproom usage scenarios, monitor production metrics after deployment

- **Risk**: Settings optimized for one PostgreSQL version incompatible with others
  - **Mitigation**: Document PostgreSQL version requirements, test against target version(s), use version-appropriate settings

- **Risk**: Tuning reduces write performance while improving read performance
  - **Mitigation**: Balance read/write workloads in testing, monitor indexing performance, ensure embedding generation not degraded

## Files/Packages Affected
- `docker/postgresql.conf` (or equivalent PostgreSQL config file)
- `docker/Dockerfile` (if config baked into image)
- `docker-compose.yml` (if using environment variable overrides)
- `LOCAL_ARCHITECTURE.md` (update with final tuned settings and rationale)
- `docs/TROUBLESHOOTING.md` (add performance tuning guidance)
- `docs/CONFIGURATION.md` (document tuning parameters)
- Possibly: `scripts/init-db.sh` (if settings applied via SQL)

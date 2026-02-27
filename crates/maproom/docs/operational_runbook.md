# Operational Runbook: Multi-Language Parser System

## Purpose

This runbook provides operational procedures, troubleshooting guides, and reference materials for managing the Maproom multi-language parser system in production. It is intended for on-call engineers, SREs, and operations teams.

**Scope**: TypeScript/JavaScript, Python, Rust, and Go language parsing

## Quick Reference

### Health Check Commands

```bash
# Check parser service status
systemctl status maproom

# Verify all parsers functional
echo 'function test() {}' | ./maproom parse --language ts  # TypeScript
echo 'def test(): pass' | ./maproom parse --language py   # Python
echo 'fn test() {}' | ./maproom parse --language rs        # Rust
echo 'func test() {}' | ./maproom parse --language go      # Go

# Check database connectivity
psql -U maproom_user -d maproom_db -c "SELECT COUNT(*) FROM chunks;"

# View recent errors
tail -100 /var/log/maproom/errors.log

# Check memory usage
ps aux | grep maproom

# Verify search working
./maproom search "function"
```

### Emergency Contacts

- **On-Call Engineer**: [Primary Contact]
- **Database Admin**: [DB Contact]
- **Team Lead**: [TL Contact]
- **Escalation**: [Manager Contact]

### Critical Files

- **Service Config**: `/etc/systemd/system/maproom.service`
- **Application Config**: `/opt/maproom/config/maproom.toml`
- **Logs**: `/var/log/maproom/`
- **Binary**: `/opt/maproom/bin/maproom`
- **Database Backups**: `/backups/maproom/` or S3

## System Architecture

### Components

```
┌─────────────────────────────────────────────┐
│          Maproom Service                     │
│  ┌─────────────────────────────────────┐   │
│  │  Parser Dispatcher                   │   │
│  │  - Language detection                │   │
│  │  - Parser selection                  │   │
│  │  - Result aggregation                │   │
│  └─────────────────────────────────────┘   │
│                    │                         │
│     ┌──────────────┼──────────────┐         │
│     │              │              │         │
│  ┌──▼──┐      ┌───▼───┐     ┌───▼───┐     │
│  │ TS/ │      │Python │     │ Rust  │     │
│  │ JS  │      │Parser │     │Parser │     │
│  └─────┘      └───────┘     └───────┘     │
│                                             │
│     └──────────────┬──────────────┘         │
│                    │                         │
│                 ┌──▼───┐                     │
│                 │  Go  │                     │
│                 │Parser│                     │
│                 └──────┘                     │
└─────────────────────┬───────────────────────┘
                      │
                ┌─────▼─────┐
                │PostgreSQL │
                │ Database  │
                └───────────┘
```

### Data Flow

1. **File Scan**: System scans repository files
2. **Language Detection**: Extension determines parser (.ts/.py/.rs/.go)
3. **Parsing**: Appropriate parser extracts symbols
4. **Chunk Creation**: Symbols converted to chunks
5. **Database Storage**: Chunks inserted into PostgreSQL
6. **Indexing**: Full-text and vector indexes updated
7. **Search**: Queries retrieve chunks across all languages

### Performance Baselines

**Parsing Performance**:
- TypeScript/JavaScript: Baseline (reference)
- Python: 50,000-60,000 files/min
- Rust: 75,000 files/min
- Go: 60,000 files/min

**Batch Throughput**: 52,303 files/min (mixed languages)

**Quality Metrics**:
- Name completeness: 100% (all languages)
- Documentation coverage: 60-86% (language-dependent)
- Error rate: 0% (validation tests)

## Common Operations

### Starting the Service

```bash
# Start service
sudo systemctl start maproom

# Verify startup
sudo journalctl -u maproom -f --since "1 minute ago"

# Check process
ps aux | grep maproom

# Test functionality
./maproom search "test"
```

**Expected Startup Time**: 5-10 seconds
**Expected Memory**: 100-200MB initial, 300-500MB under load

### Stopping the Service

```bash
# Graceful stop
sudo systemctl stop maproom

# Wait for processes to terminate
# Timeout: 30 seconds

# Force stop if needed (avoid if possible)
sudo systemctl kill -s SIGKILL maproom
```

### Restarting the Service

```bash
# Graceful restart
sudo systemctl restart maproom

# Verify restart
sudo systemctl status maproom
./maproom search "test"
```

**Expected Downtime**: 10-15 seconds

### Viewing Logs

```bash
# Real-time logs
sudo journalctl -u maproom -f

# Last 100 lines
sudo journalctl -u maproom -n 100

# Errors only
sudo journalctl -u maproom -p err

# Specific time range
sudo journalctl -u maproom --since "1 hour ago" --until "30 minutes ago"

# Application logs
tail -f /var/log/maproom/application.log
tail -f /var/log/maproom/errors.log
tail -f /var/log/maproom/indexing.log
```

### Checking Database Health

```sql
-- Connection count
SELECT
  application_name,
  count(*) as connections,
  state
FROM pg_stat_activity
WHERE application_name LIKE 'maproom%'
GROUP BY application_name, state;

-- Database size
SELECT
  pg_database.datname,
  pg_size_pretty(pg_database_size(pg_database.datname)) AS size
FROM pg_database
WHERE datname = 'maproom_db';

-- Table sizes
SELECT
  schemaname,
  tablename,
  pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size
FROM pg_tables
WHERE schemaname = 'public'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;

-- Recent activity
SELECT
  query_start,
  state,
  query
FROM pg_stat_activity
WHERE datname = 'maproom_db'
  AND state != 'idle'
ORDER BY query_start DESC;
```

## Troubleshooting

### Decision Tree: Service Not Starting

```
Service won't start
├── Check systemctl status
│   ├── Failed → Check journalctl for errors
│   │   ├── Port conflict → Kill conflicting process
│   │   ├── Permission denied → Fix file permissions
│   │   └── Config error → Validate config file
│   └── Inactive → Start service manually
├── Check binary exists
│   └── Missing → Deploy binary from release
├── Check database connectivity
│   └── Connection refused → Verify PostgreSQL running
└── Check disk space
    └── No space → Clear old logs/backups
```

### Issue: Parse Errors Increasing

**Symptoms**:
- Error log growing rapidly
- Parse error rate >5%
- User reports missing search results

**Diagnosis**:

```sql
-- Error analysis
SELECT
  language,
  error_type,
  COUNT(*) as error_count,
  MAX(error_message) as sample_error,
  MAX(file_path) as sample_file
FROM indexing_errors
WHERE created_at > NOW() - INTERVAL '1 hour'
GROUP BY language, error_type
ORDER BY error_count DESC;

-- Recent failures
SELECT
  file_path,
  language,
  error_message,
  created_at
FROM indexing_errors
WHERE created_at > NOW() - INTERVAL '1 hour'
ORDER BY created_at DESC
LIMIT 20;
```

**Resolution Steps**:

1. **Identify Pattern**:
   - Single repository? → Specific repo issue
   - Single language? → Parser bug
   - All languages? → System issue

2. **For Specific Repository**:
   ```bash
   # Exclude problematic repo temporarily
   echo "excluded_repos = ['/path/to/repo']" >> /opt/maproom/config/maproom.toml
   sudo systemctl restart maproom

   # Test repository offline
   ./maproom scan --path /path/to/repo --dry-run

   # Check for encoding issues
   file -i /path/to/repo/**/*
   ```

3. **For Specific Language**:
   ```bash
   # Test parser with sample files
   echo 'def test(): pass' | ./maproom parse --language py --verbose

   # Check parser tests
   cargo test --test python_parser_test
   cargo test --test rust_parser_test
   cargo test --test go_parser_test
   ```

4. **For System-Wide Issues**:
   ```bash
   # Check system resources
   df -h  # Disk space
   free -h  # Memory
   top  # CPU and processes

   # Check database
   psql -U maproom_user -d maproom_db -c "SELECT pg_database_size('maproom_db');"

   # Restart service
   sudo systemctl restart maproom
   ```

**Escalation**: If error rate >10% persists for >30 minutes, escalate to team lead.

### Issue: High Memory Usage

**Symptoms**:
- Memory usage >1GB
- OOM errors in logs
- Service restarts due to memory

**Diagnosis**:

```bash
# Check current memory
ps aux | grep maproom | awk '{print $6}'

# Monitor over time
watch -n 10 'ps aux | grep maproom'

# Check for memory leaks
sudo journalctl -u maproom | grep -i "memory\|oom"

# Review large files being processed
tail -100 /var/log/maproom/indexing.log | grep "large file"
```

**Resolution Steps**:

1. **Immediate Relief** (restart service):
   ```bash
   sudo systemctl restart maproom
   ```

2. **Investigation**:
   ```sql
   -- Find files processed before memory spike
   SELECT
     file_path,
     file_size_bytes,
     created_at
   FROM chunks
   WHERE created_at > NOW() - INTERVAL '30 minutes'
   ORDER BY file_size_bytes DESC
   LIMIT 20;
   ```

3. **Mitigation**:
   ```bash
   # Add memory limits to service
   sudo systemctl edit maproom

   # Add these lines:
   [Service]
   MemoryMax=1G
   MemoryHigh=800M

   # Reload and restart
   sudo systemctl daemon-reload
   sudo systemctl restart maproom
   ```

4. **Long-term Fix**:
   - Profile code for memory leaks
   - Implement streaming for large files
   - Add file size limits
   - Optimize parser memory usage

**Escalation**: If memory continues growing after restart, escalate immediately.

### Issue: Slow Search Performance

**Symptoms**:
- Search queries taking >1 second
- User complaints about slow results
- High database CPU

**Diagnosis**:

```sql
-- Slow queries
SELECT
  query,
  mean_exec_time,
  calls,
  total_exec_time
FROM pg_stat_statements
WHERE query LIKE '%chunks%'
ORDER BY mean_exec_time DESC
LIMIT 10;

-- Index usage
SELECT
  schemaname,
  tablename,
  indexname,
  idx_scan,
  idx_tup_read
FROM pg_stat_user_indexes
WHERE schemaname = 'public'
ORDER BY idx_scan DESC;

-- Table bloat
SELECT
  schemaname,
  tablename,
  pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) as size,
  n_live_tup,
  n_dead_tup
FROM pg_stat_user_tables
WHERE schemaname = 'public'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;
```

**Resolution Steps**:

1. **Immediate Fix** (rebuild indexes):
   ```sql
   -- Reindex main tables
   REINDEX TABLE chunks;
   REINDEX TABLE chunk_embeddings;

   -- Analyze tables
   ANALYZE chunks;
   ANALYZE chunk_embeddings;
   ```

2. **Vacuum if needed**:
   ```sql
   -- Check if vacuum needed
   SELECT
     schemaname,
     tablename,
     n_dead_tup,
     last_vacuum,
     last_autovacuum
   FROM pg_stat_user_tables
   WHERE n_dead_tup > 10000;

   -- Run vacuum
   VACUUM ANALYZE chunks;
   ```

3. **Performance Tuning**:
   ```sql
   -- Update statistics
   ANALYZE;

   -- Check query plans
   EXPLAIN ANALYZE
   SELECT * FROM chunks
   WHERE to_tsvector('english', ts_doc) @@ plainto_tsquery('english', 'test')
   LIMIT 20;
   ```

**Escalation**: If performance doesn't improve after reindex, escalate to database admin.

### Issue: Database Connection Failures

**Symptoms**:
- "connection refused" errors
- Service can't connect to database
- Intermittent connectivity issues

**Diagnosis**:

```bash
# Check PostgreSQL status
sudo systemctl status postgresql

# Check connections
sudo -u postgres psql -c "SELECT count(*) FROM pg_stat_activity;"

# Test connection manually
psql -U maproom_user -h localhost -d maproom_db -c "SELECT 1;"

# Check connection limits
sudo -u postgres psql -c "SHOW max_connections;"
sudo -u postgres psql -c "SELECT count(*) FROM pg_stat_activity;"
```

**Resolution Steps**:

1. **PostgreSQL Not Running**:
   ```bash
   sudo systemctl start postgresql
   sudo systemctl status postgresql
   ```

2. **Connection Limit Reached**:
   ```sql
   -- Kill idle connections
   SELECT pg_terminate_backend(pid)
   FROM pg_stat_activity
   WHERE datname = 'maproom_db'
     AND state = 'idle'
     AND state_change < NOW() - INTERVAL '10 minutes';
   ```

3. **Connection Pool Issues**:
   ```bash
   # Restart maproom service
   sudo systemctl restart maproom

   # Check connection pool config
   grep -i "pool" /opt/maproom/config/maproom.toml
   ```

**Escalation**: If database won't start, escalate to database admin immediately.

## Performance Tuning

### Parser Configuration

**Memory Limits**:
```toml
[parser]
max_memory_mb = 500
max_file_size_mb = 10
batch_size = 100
```

**Concurrency**:
```toml
[indexing]
worker_threads = 4
queue_size = 1000
timeout_seconds = 30
```

**Per-Language Settings**:
```toml
[parser.python]
max_nesting_depth = 20
extract_docstrings = true

[parser.rust]
max_nesting_depth = 25
extract_doc_comments = true

[parser.go]
max_nesting_depth = 20
extract_doc_comments = true
```

### Database Tuning

**Connection Pool**:
```toml
[database]
max_connections = 20
min_connections = 5
connection_timeout = 30
```

**Performance**:
```sql
-- Recommended PostgreSQL settings for Maproom
-- (Add to postgresql.conf)

shared_buffers = 256MB
effective_cache_size = 1GB
maintenance_work_mem = 64MB
checkpoint_completion_target = 0.9
wal_buffers = 16MB
default_statistics_target = 100
random_page_cost = 1.1
effective_io_concurrency = 200
work_mem = 16MB
```

### Monitoring Recommendations

**Key Metrics to Monitor**:
1. Parse success rate (target: >98%)
2. Parse time p95 (target: <150ms)
3. Memory usage (target: <500MB)
4. Database connection count (target: <20)
5. Disk space usage (alert: <20% free)
6. Error log growth rate

**Alert Thresholds**:
```yaml
# Critical (immediate response)
- parse_error_rate > 10%
- memory_usage > 1GB
- disk_free < 10%
- service_down

# Warning (respond within 1 hour)
- parse_error_rate > 5%
- memory_usage > 500MB
- disk_free < 20%
- slow_queries > 1s

# Info (review daily)
- parse_error_rate > 2%
- unusual_file_types
- performance_degradation < 20%
```

## Maintenance Procedures

### Log Rotation

```bash
# Configure logrotate
sudo cat > /etc/logrotate.d/maproom <<EOF
/var/log/maproom/*.log {
    daily
    rotate 7
    compress
    delaycompress
    missingok
    notifempty
    create 0644 maproom maproom
    postrotate
        systemctl reload maproom
    endscript
}
EOF

# Test rotation
sudo logrotate -d /etc/logrotate.d/maproom
sudo logrotate -f /etc/logrotate.d/maproom
```

### Database Maintenance

**Weekly Tasks**:
```sql
-- Run vacuum
VACUUM ANALYZE chunks;
VACUUM ANALYZE chunk_embeddings;
VACUUM ANALYZE repositories;

-- Reindex if needed
REINDEX TABLE CONCURRENTLY chunks;

-- Update statistics
ANALYZE;
```

**Monthly Tasks**:
```bash
# Full database backup
pg_dump -U maproom_user -d maproom_db -F c -f maproom_backup_$(date +%Y%m%d).dump

# Verify backup
pg_restore --list maproom_backup_*.dump | head -20

# Archive old backups
find /backups/maproom/ -name "*.dump" -mtime +30 -delete
```

### Binary Updates

```bash
# 1. Backup current binary
cp /opt/maproom/bin/maproom /opt/maproom/bin/maproom.backup

# 2. Deploy new binary
scp ./target/release/maproom prod-server:/opt/maproom/bin/

# 3. Verify binary
/opt/maproom/bin/maproom --version

# 4. Restart service
sudo systemctl restart maproom

# 5. Verify functionality
/opt/maproom/bin/maproom search "test"

# 6. Monitor for issues
sudo journalctl -u maproom -f
```

## Runbook Maintenance

**Review Schedule**: Monthly or after major incidents

**Update Triggers**:
- New parser added
- System architecture changes
- Common issues discovered
- Performance characteristics change
- Team feedback

**Version History**:
- v1.0 (2025-10-25): Initial multi-language runbook

---

**Document Version**: 1.0
**Last Updated**: 2025-10-25
**Next Review**: 2025-11-25
**Owner**: Operations Team

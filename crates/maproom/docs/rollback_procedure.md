# Rollback Procedure: Multi-Language Support

## Executive Summary

This document describes the rollback procedure for the multi-language support deployment. The rollback is **simple and fast** (< 5 minutes) because all changes are **additive and backwards compatible**.

**Key Point**: No database rollback is required. Simply deploying the previous binary version restores TypeScript/JavaScript-only functionality.

## Rollback Decision Criteria

Consider rollback if:

1. **Critical parsing failures**
   - Error rate >10% for any language
   - Systematic crashes during indexing
   - Data corruption detected

2. **Severe performance degradation**
   - Indexing speed <50% of baseline
   - Search latency >500ms p95
   - Database resource exhaustion

3. **Production incidents**
   - Service unavailable >5 minutes
   - Data integrity issues
   - Security vulnerabilities discovered

**Do NOT rollback for**:
- Individual file parsing failures (expected edge cases)
- Minor performance variations (<20%)
- Feature requests or non-critical bugs

## Quick Rollback (5 Minutes)

### Step 1: Deploy Previous Binary

```bash
# Stop current maproom service
systemctl stop maproom

# Restore previous binary from backup
cp /opt/maproom/bin/maproom.backup /opt/maproom/bin/maproom

# Or download previous release
wget https://releases/maproom/v1.x.x/maproom -O /opt/maproom/bin/maproom
chmod +x /opt/maproom/bin/maproom

# Verify binary version
/opt/maproom/bin/maproom --version

# Start maproom service
systemctl start maproom
```

### Step 2: Verify Service Health

```bash
# Check service status
systemctl status maproom

# Verify TypeScript/JavaScript parsing still works
echo 'function test() { return 42; }' | \
  /opt/maproom/bin/maproom parse --language ts

# Check database connectivity
psql -U maproom_user -d maproom_db -c "SELECT COUNT(*) FROM chunks;"

# Monitor logs
tail -f /var/log/maproom/service.log
```

### Step 3: Communicate Rollback

```
Subject: Maproom Multi-Language Support - Rolled Back

Team,

We've rolled back the multi-language support deployment due to [reason].

Current status:
- TypeScript/JavaScript search: ✓ Working normally
- Python/Rust/Go search: ✗ Temporarily unavailable
- All existing data: ✓ Preserved

Investigation in progress. ETA for re-deployment: [timeframe]
```

**Total Time**: < 5 minutes

## Detailed Rollback Procedure

### Phase 1: Immediate Service Restoration (2 minutes)

**Objective**: Restore TypeScript/JavaScript functionality as quickly as possible.

```bash
# 1. Identify previous working version
ls -lh /opt/maproom/bin/maproom*

# 2. Stop service (graceful shutdown)
systemctl stop maproom

# 3. Replace binary
mv /opt/maproom/bin/maproom /opt/maproom/bin/maproom.failed
cp /opt/maproom/bin/maproom.backup /opt/maproom/bin/maproom

# 4. Start service
systemctl start maproom

# 5. Verify startup
journalctl -u maproom -f --since "1 minute ago"
```

### Phase 2: Verification (2 minutes)

**Objective**: Confirm TypeScript/JavaScript functionality is fully restored.

```sql
-- Verify existing data intact
SELECT
  language,
  COUNT(*) as chunk_count,
  MIN(created_at) as earliest,
  MAX(created_at) as latest
FROM chunks
GROUP BY language
ORDER BY chunk_count DESC;

-- Check recent indexing (should show TS/JS only after rollback)
SELECT
  file_path,
  language,
  created_at
FROM chunks
WHERE created_at > NOW() - INTERVAL '5 minutes'
ORDER BY created_at DESC
LIMIT 10;

-- Verify search functionality
SELECT symbol_name, file_path, kind
FROM chunks
WHERE to_tsvector('english', ts_doc) @@ plainto_tsquery('english', 'function')
LIMIT 5;
```

```bash
# Test search via CLI
/opt/maproom/bin/maproom search "test function"

# Expected: Results from .ts/.js files only
# No errors in output
```

### Phase 3: Cleanup and Monitoring (1 minute)

```bash
# Monitor error logs
tail -f /var/log/maproom/errors.log | grep -v "INFO"

# Check resource usage
top -bn1 | grep maproom
ps aux | grep maproom

# Verify database connections
psql -U maproom_user -d maproom_db -c "
SELECT
  application_name,
  count(*) as connection_count,
  state
FROM pg_stat_activity
WHERE application_name LIKE 'maproom%'
GROUP BY application_name, state;
"
```

## What Happens to Multi-Language Data?

**Python/Rust/Go chunks remain in database** - they are not deleted during rollback.

**Effects**:
- ✅ Existing multi-language chunks preserved (no data loss)
- ✅ TypeScript/JavaScript search continues working normally
- ⚠️ No NEW Python/Rust/Go files will be indexed until re-deployment
- ⚠️ Existing Python/Rust/Go chunks remain searchable (data persists)

**To completely remove multi-language data** (only if necessary):

```sql
-- CAUTION: This deletes multi-language chunks permanently
-- Only run if specifically required
BEGIN;

-- Count chunks to be deleted
SELECT language, COUNT(*) FROM chunks
WHERE language IN ('py', 'rs', 'go')
GROUP BY language;

-- Delete Python/Rust/Go chunks (if confirmed)
DELETE FROM chunks WHERE language = 'py';
DELETE FROM chunks WHERE language = 'rs';
DELETE FROM chunks WHERE language = 'go';

-- Verify deletion
SELECT language, COUNT(*) FROM chunks GROUP BY language;

COMMIT; -- or ROLLBACK to cancel
```

**Recommendation**: Keep multi-language data unless there's a specific reason to remove it (e.g., data corruption).

## Database Rollback (If Schema Issues Occur)

**Note**: This is rarely needed since all schema changes are additive.

### Check If Database Rollback Needed

```sql
-- Verify schema integrity
SELECT table_name, column_name, data_type
FROM information_schema.columns
WHERE table_schema = 'public'
  AND table_name = 'chunks'
ORDER BY ordinal_position;

-- Check for orphaned data
SELECT COUNT(*) as orphaned_count
FROM chunks c
LEFT JOIN repositories r ON c.repo_id = r.id
WHERE r.id IS NULL;

-- Verify foreign key constraints
SELECT
  tc.table_name,
  kcu.column_name,
  ccu.table_name AS foreign_table_name,
  ccu.column_name AS foreign_column_name
FROM information_schema.table_constraints AS tc
JOIN information_schema.key_column_usage AS kcu
  ON tc.constraint_name = kcu.constraint_name
JOIN information_schema.constraint_column_usage AS ccu
  ON ccu.constraint_name = tc.constraint_name
WHERE tc.constraint_type = 'FOREIGN KEY'
  AND tc.table_name = 'chunks';
```

### Database Rollback (if needed)

**Option 1: Rollback specific migration**

```bash
# Identify current migration version
./target/release/maproom db status

# Rollback to specific version (if tool supports it)
./target/release/maproom db rollback --to 0010

# Verify rollback
./target/release/maproom db status
```

**Option 2: Restore from backup**

```bash
# Stop service
systemctl stop maproom

# Drop current database
dropdb -U postgres maproom_db

# Restore from backup
pg_restore -U postgres -d postgres -C maproom_backup_YYYYMMDD_HHMMSS.dump

# Verify restoration
psql -U maproom_user -d maproom_db -c "SELECT COUNT(*) FROM chunks;"

# Start service
systemctl start maproom
```

**Estimated Time**:
- Rollback migration: 1-2 minutes
- Full restore: 5-30 minutes (depends on database size)

## Post-Rollback Investigation

### 1. Capture Evidence

```bash
# Save error logs
cp /var/log/maproom/errors.log /var/log/maproom/rollback_investigation/errors_$(date +%Y%m%d_%H%M%S).log

# Export database statistics
psql -U maproom_user -d maproom_db -c "
COPY (
  SELECT * FROM pg_stat_user_tables WHERE schemaname = 'public'
) TO '/tmp/db_stats_$(date +%Y%m%d).csv' CSV HEADER;
"

# Save system metrics
dmesg > /tmp/dmesg_rollback_$(date +%Y%m%d).txt
free -h > /tmp/memory_rollback_$(date +%Y%m%d).txt
df -h > /tmp/disk_rollback_$(date +%Y%m%d).txt
```

### 2. Analyze Root Cause

**Common Issues and Diagnosis**:

| Issue | Investigation | Log Location |
|-------|---------------|--------------|
| Parser crashes | `grep "panic" /var/log/maproom/errors.log` | errors.log |
| Memory exhaustion | `dmesg | grep -i "out of memory"` | dmesg, system |
| Database errors | `grep "ERROR" /var/log/postgresql/*.log` | postgres logs |
| Performance | `pg_stat_statements` analysis | database |

### 3. Document Findings

Create incident report:
```markdown
# Rollback Incident Report

**Date**: [Date]
**Duration**: [Start] to [End]
**Reason**: [Brief description]

## Timeline
- [Time]: Deployment started
- [Time]: Issue detected
- [Time]: Rollback initiated
- [Time]: Service restored

## Root Cause
[Detailed analysis of what went wrong]

## Impact
- Users affected: [count/percentage]
- Data loss: None / [description]
- Downtime: [duration]

## Prevention
[Steps to prevent recurrence]

## Action Items
1. [Fix/improvement]
2. [Additional testing]
3. [Documentation update]
```

## Rollback Testing (Pre-Deployment)

**Run this in staging BEFORE production deployment**:

```bash
# 1. Deploy multi-language version
cargo build --release
./scripts/deploy_staging.sh

# 2. Index sample data
./target/release/maproom scan --path /test/polyglot/repo

# 3. Verify multi-language works
cargo test --test large_scale_validation_test

# 4. Perform test rollback
./scripts/rollback_staging.sh

# 5. Verify TypeScript/JavaScript still works
cargo test --test integration_test -- --test-threads=1

# 6. Check data integrity
psql -U maproom_user -d maproom_staging -f scripts/verify_integrity.sql
```

## Rollback Decision Matrix

| Scenario | Rollback? | Rationale |
|----------|-----------|-----------|
| 5% error rate on Python files | NO | Document edge cases, fix in next release |
| 50% error rate on all languages | YES | Systematic parser issue |
| Search latency +10% | NO | Within acceptable range |
| Search latency +200% | YES | Severe performance degradation |
| Single repository indexing fails | NO | Investigate specific repo issues |
| All repositories failing | YES | Critical bug |
| Minor memory increase +10MB | NO | Expected with new parsers |
| Memory leak +1GB/hour | YES | Resource exhaustion risk |
| User complaints about accuracy | NO | Validate and adjust, not rollback |
| Service outage >5 minutes | YES | Availability priority |

## Success Criteria (Do NOT Rollback)

- ✅ Error rate <5% across all languages
- ✅ Indexing speed >100 files/min
- ✅ Search latency <200ms p95
- ✅ Memory usage stable over 24 hours
- ✅ No service interruptions
- ✅ TypeScript/JavaScript unaffected
- ✅ Users report improved search across languages

## Emergency Contacts

**On-Call Engineer**: [Contact]
**Database Admin**: [Contact]
**Team Lead**: [Contact]
**Incident Commander**: [Contact]

## Rollback Checklist

- [ ] Rollback decision approved by incident commander
- [ ] Team notified via [communication channel]
- [ ] Previous binary version identified and available
- [ ] Database backup verified and recent (<1 hour)
- [ ] Monitoring dashboards open and visible
- [ ] Runbook open and ready to follow
- [ ] Rollback executed (binary replaced)
- [ ] Service restarted and health verified
- [ ] TypeScript/JavaScript functionality tested
- [ ] Database integrity verified
- [ ] Monitoring confirms normal operation
- [ ] Users notified of rollback
- [ ] Incident report started
- [ ] Post-mortem scheduled

---

**Document Version**: 1.0
**Last Updated**: 2025-10-25
**Tested**: [Date of last rollback test in staging]
**Next Review**: After first production deployment

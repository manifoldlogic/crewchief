# Production Rollout Plan: Multi-Language Parser Deployment

## Executive Summary

This document outlines the staged production rollout strategy for deploying Python, Rust, and Go language parsing capabilities to the Maproom semantic code search system. The rollout uses a phased approach to minimize risk and ensure production stability.

**Rollout Status**: Ready for deployment
- All validation tests passing (0% error rate)
- Performance validated (52,303 files/min throughput)
- Search quality confirmed (100% name completeness, 90%+ consistency)
- Migration and rollback procedures documented

**Rollout Timeline**: 7 days total (3 stages × 48h monitoring + 1 day final validation)

## Rollout Strategy

### Phased Deployment Approach

**Why staged rollout?**
1. **Risk Mitigation**: Deploy most stable parser first, validate before proceeding
2. **Early Detection**: Catch issues with one language before affecting others
3. **Gradual Load**: Assess performance impact incrementally
4. **Learning**: Refine procedures based on early stage feedback

**Language Order**:
1. **Python** (Stage 1) - Most mature, highest test coverage (12 core + 107 production tests)
2. **Rust** (Stage 2) - Strong validation (20 tests), moderate complexity
3. **Go** (Stage 3) - Recent addition (23 tests), lowest risk last

### Rollout Phases

```
Day 0: Pre-Deployment
Day 1: Python Rollout
Days 2-3: Python Monitoring
Day 3: Rust Rollout (if Python stable)
Days 4-5: Rust Monitoring
Day 5: Go Rollout (if Rust stable)
Days 6-7: Go Monitoring
Day 7: Final Validation & Documentation
```

## Pre-Deployment Checklist

### Infrastructure Readiness

- [ ] Database backup created and verified (< 24 hours old)
- [ ] Staging environment tested with production data copy
- [ ] Production binary built and smoke-tested
- [ ] Rollback procedure tested in staging
- [ ] Monitoring dashboards configured and accessible
- [ ] Alert notifications configured (Slack/Email/PagerDuty)
- [ ] On-call rotation scheduled for rollout period
- [ ] Team trained on monitoring dashboards and runbook

### Validation Confirmation

- [ ] All large-scale validation tests passing (LANG_PARSE-4001)
- [ ] All search quality tests passing (LANG_PARSE-4002)
- [ ] Migration guide reviewed (LANG_PARSE-4003)
- [ ] Rollback procedure reviewed (LANG_PARSE-4003)
- [ ] Performance baselines documented from validation tests

### Communication

- [ ] Stakeholders notified of rollout schedule
- [ ] User communication drafted (new language support announcement)
- [ ] Incident response plan reviewed
- [ ] Rollback decision authority identified

## Stage 1: Python Parser Deployment

### Day 1 - Python Rollout

**Objective**: Deploy Python parsing with comprehensive monitoring

**Prerequisites**:
- [ ] All pre-deployment checklist items complete
- [ ] Python validation tests passing (12 core + 107 production)
- [ ] Baseline metrics documented

**Deployment Steps**:

```bash
# 1. Deploy new binary
scp ./target/release/crewchief-maproom prod-server:/opt/maproom/bin/
systemctl restart maproom

# 2. Verify Python parser available
./crewchief-maproom parse --language py < test_file.py

# 3. Index first Python repository
./crewchief-maproom scan --path /repos/python/django-sample

# 4. Monitor metrics
# - Open Grafana dashboard
# - Check parse success rate
# - Monitor memory usage
# - Verify chunk creation

# 5. Gradual rollout
# Hour 1: 1 Python repo
# Hour 2: 5 Python repos
# Hour 4: 25% of Python repos
# Hour 8: 50% of Python repos
# Day 2: 100% of Python repos
```

**Success Criteria** (must meet ALL for 48 hours):
- ✅ Parse error rate < 2%
- ✅ p95 parse time < 100ms
- ✅ Memory usage < 500MB per process
- ✅ No parser crashes or restarts
- ✅ Files/min throughput > 100 files/min
- ✅ No user-reported search quality issues

**Monitoring Focus**:
- Parse error logs (check every 2 hours)
- Memory trends (check every 4 hours)
- Search result quality (spot checks daily)
- Database query performance

**Rollback Triggers**:
- Parse error rate > 10%
- Parser crashes > 2 in 24 hours
- Memory leak detected (>1GB/hour growth)
- Critical search functionality broken
- Database performance degradation >50%

### Days 2-3 - Python Monitoring Period

**Objective**: Validate Python parser stability before proceeding to Rust

**Daily Activities**:
- [ ] Morning: Review overnight metrics and logs
- [ ] Midday: Spot-check search quality across Python codebases
- [ ] Afternoon: Verify memory usage stable
- [ ] Evening: Review error logs and prepare daily report

**Validation Queries**:

```sql
-- Python parsing statistics
SELECT
  COUNT(*) as python_files,
  COUNT(DISTINCT file_path) as unique_files,
  AVG(ARRAY_LENGTH(string_to_array(signature, ','), 1)) as avg_symbols_per_file
FROM chunks
WHERE language = 'py'
  AND created_at > NOW() - INTERVAL '48 hours';

-- Error rate check
SELECT
  DATE_TRUNC('hour', created_at) as hour,
  COUNT(*) as error_count
FROM indexing_errors
WHERE language = 'py'
  AND created_at > NOW() - INTERVAL '48 hours'
GROUP BY hour
ORDER BY hour;

-- Performance check
SELECT
  PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY duration_ms) as p50,
  PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY duration_ms) as p95,
  PERCENTILE_CONT(0.99) WITHIN GROUP (ORDER BY duration_ms) as p99
FROM parse_metrics
WHERE language = 'py'
  AND timestamp > NOW() - INTERVAL '48 hours';
```

**Go/No-Go Decision for Stage 2**:
- [ ] All success criteria met for 48 consecutive hours
- [ ] No unresolved critical issues
- [ ] Team confidence level: High
- [ ] Rollback procedure tested and validated

**Decision Point**: End of Day 3
- ✅ **PROCEED to Rust** if all criteria met
- 🛑 **EXTEND MONITORING** if minor issues (add 24h)
- ⚠️ **ROLLBACK** if critical issues

## Stage 2: Rust Parser Deployment

### Day 3 - Rust Rollout

**Objective**: Deploy Rust parsing building on Python success

**Prerequisites**:
- [ ] Python parser stable for 48+ hours
- [ ] Rust validation tests passing (20 tests)
- [ ] Lessons learned from Python rollout documented

**Deployment Steps**:

```bash
# 1. Verify current system health
./crewchief-maproom db status
psql -U maproom_user -d maproom_db -c "SELECT COUNT(*) FROM chunks WHERE language = 'py';"

# 2. Deploy Rust parser (already in binary)
# No new deployment needed - Rust integrated in same binary as Python

# 3. Index first Rust repository
./crewchief-maproom scan --path /repos/rust/tokio-sample

# 4. Gradual rollout
# Hour 1: 1 Rust repo
# Hour 2: 5 Rust repos
# Hour 4: 25% of Rust repos
# Hour 8: 50% of Rust repos
# Day 4: 100% of Rust repos
```

**Success Criteria** (must meet ALL for 48 hours):
- ✅ Parse error rate < 2%
- ✅ p95 parse time < 150ms (Rust more complex than Python)
- ✅ Memory usage < 600MB per process
- ✅ No parser crashes or restarts
- ✅ Files/min throughput > 100 files/min
- ✅ Python parser continues stable

**Monitoring Focus**:
- Compare Rust vs Python metrics
- Memory growth trends across both parsers
- Cross-language search quality
- Database storage growth rate

### Days 4-5 - Rust Monitoring Period

**Objective**: Validate Rust parser stability alongside Python

**Validation Queries**:

```sql
-- Multi-language statistics
SELECT
  language,
  COUNT(*) as file_count,
  AVG(end_line - start_line) as avg_lines_per_chunk
FROM chunks
WHERE language IN ('py', 'rs')
  AND created_at > NOW() - INTERVAL '48 hours'
GROUP BY language;

-- Cross-language consistency check
SELECT
  DATE_TRUNC('day', created_at) as day,
  language,
  COUNT(*) as chunks_created
FROM chunks
WHERE language IN ('py', 'rs')
  AND created_at > NOW() - INTERVAL '48 hours'
GROUP BY day, language
ORDER BY day, language;
```

**Go/No-Go Decision for Stage 3**:
- [ ] All Rust success criteria met for 48 consecutive hours
- [ ] Python parser remains stable
- [ ] No unresolved critical issues
- [ ] Database performance acceptable

**Decision Point**: End of Day 5
- ✅ **PROCEED to Go** if all criteria met
- 🛑 **EXTEND MONITORING** if minor issues
- ⚠️ **ROLLBACK RUST** if Rust issues (keep Python)

## Stage 3: Go Parser Deployment

### Day 5 - Go Rollout

**Objective**: Complete multi-language rollout with Go

**Prerequisites**:
- [ ] Python and Rust parsers stable for 48+ hours each
- [ ] Go validation tests passing (23 tests)
- [ ] System resources adequate for all three parsers

**Deployment Steps**:

```bash
# 1. Final system health check
./crewchief-maproom search "test" # Verify search working
psql -U maproom_user -d maproom_db -c "SELECT language, COUNT(*) FROM chunks GROUP BY language;"

# 2. Index first Go repository
./crewchief-maproom scan --path /repos/go/kubernetes-sample

# 3. Gradual rollout
# Hour 1: 1 Go repo
# Hour 2: 5 Go repos
# Hour 4: 25% of Go repos
# Hour 8: 50% of Go repos
# Day 6: 100% of Go repos
```

**Success Criteria** (must meet ALL for 48 hours):
- ✅ Parse error rate < 2%
- ✅ p95 parse time < 120ms
- ✅ Memory usage < 550MB per process
- ✅ No parser crashes or restarts
- ✅ Files/min throughput > 100 files/min
- ✅ Python and Rust parsers continue stable
- ✅ Cross-language search quality excellent

**Monitoring Focus**:
- All three languages simultaneously
- Memory usage trends for complete system
- Cross-language search consistency
- Production readiness validation

### Days 6-7 - Go Monitoring & Final Validation

**Objective**: Confirm complete multi-language system stability

**Final Validation Queries**:

```sql
-- Complete multi-language summary
SELECT
  language,
  COUNT(*) as total_chunks,
  COUNT(DISTINCT file_path) as unique_files,
  COUNT(CASE WHEN symbol_name IS NOT NULL THEN 1 END) as symbols,
  ROUND(100.0 * COUNT(CASE WHEN docstring IS NOT NULL THEN 1 END) / COUNT(*), 2) as doc_coverage_pct
FROM chunks
GROUP BY language
ORDER BY total_chunks DESC;

-- Error analysis across all languages
SELECT
  language,
  error_type,
  COUNT(*) as error_count,
  MAX(created_at) as last_occurrence
FROM indexing_errors
WHERE created_at > NOW() - INTERVAL '7 days'
GROUP BY language, error_type
ORDER BY error_count DESC;

-- Performance summary
SELECT
  language,
  PERCENTILE_CONT(0.5) WITHIN GROUP (ORDER BY duration_ms) as p50_ms,
  PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY duration_ms) as p95_ms,
  COUNT(*) as parse_count
FROM parse_metrics
WHERE timestamp > NOW() - INTERVAL '7 days'
GROUP BY language;
```

**Final Go/No-Go Decision**:
- [ ] All three parsers stable for 48+ hours
- [ ] No critical issues outstanding
- [ ] Performance within targets
- [ ] Search quality validated
- [ ] User feedback positive
- [ ] Documentation updated

## Post-Rollout Activities

### Day 7 - Rollout Completion

**Completion Checklist**:
- [ ] All three languages deployed and stable
- [ ] Monitoring dashboards updated with production baselines
- [ ] Alert thresholds tuned based on observed metrics
- [ ] User documentation published
- [ ] Internal knowledge base updated
- [ ] Rollout retrospective scheduled

**Documentation Updates**:
- [ ] Update README with multi-language capabilities
- [ ] Publish user guide for searching Python/Rust/Go code
- [ ] Document new file type support
- [ ] Update system requirements if needed

**Communication**:

```
Subject: Multi-Language Search Now Available!

Team,

We've successfully rolled out Python, Rust, and Go language support!

What's new:
✓ Search Python code (functions, classes, methods)
✓ Search Rust code (functions, structs, traits, impl blocks)
✓ Search Go code (functions, methods, interfaces, structs)
✓ Cross-language search queries
✓ Consistent search quality across all languages

Statistics from rollout:
- [X] Python files indexed
- [Y] Rust files indexed
- [Z] Go files indexed
- 0% error rate across all languages
- Excellent performance (>100 files/min)

Try it now: Search for code concepts across all your polyglot repositories!

Questions? See: docs/multi-language-search-guide.md
```

## Monitoring Guidelines

### Key Metrics to Track

**Health Metrics** (check every 2 hours during rollout):
- Parse success rate per language
- Error count and error types
- Parser process status
- Memory usage trends

**Performance Metrics** (check every 4 hours):
- Files processed per minute
- Parse time percentiles (p50, p95, p99)
- Database query latency
- Search result quality

**Resource Metrics** (check daily):
- Disk space usage
- Database size growth
- Index sizes
- CPU utilization

### Alert Thresholds

**Critical Alerts** (immediate response required):
- Parse error rate > 10%
- Parser crash/restart
- Memory usage > 1GB per process
- Search service unavailable
- Database connection failures

**Warning Alerts** (investigate within 1 hour):
- Parse error rate > 5%
- p95 parse time > 2x baseline
- Memory usage > 500MB per process
- Disk space < 20% free
- Error log growth rate high

**Info Alerts** (investigate within 4 hours):
- Parse error rate > 2%
- Unusual file type encountered
- Performance degradation < 20%
- Database query slow but functional

### Monitoring Queries

```sql
-- Real-time parse success rate
SELECT
  language,
  COUNT(*) as total_attempts,
  COUNT(CASE WHEN success = true THEN 1 END) as successful,
  ROUND(100.0 * COUNT(CASE WHEN success = true THEN 1 END) / COUNT(*), 2) as success_rate
FROM parse_attempts
WHERE timestamp > NOW() - INTERVAL '1 hour'
GROUP BY language;

-- Recent errors
SELECT
  language,
  error_type,
  file_path,
  error_message,
  created_at
FROM indexing_errors
WHERE created_at > NOW() - INTERVAL '1 hour'
ORDER BY created_at DESC
LIMIT 20;

-- Memory usage trend
SELECT
  DATE_TRUNC('hour', timestamp) as hour,
  language,
  AVG(memory_mb) as avg_memory,
  MAX(memory_mb) as peak_memory
FROM process_metrics
WHERE timestamp > NOW() - INTERVAL '24 hours'
GROUP BY hour, language
ORDER BY hour, language;
```

## Rollback Procedures

### When to Rollback

**Immediate Rollback** (within 5 minutes):
- Parse error rate > 25% for any language
- Multiple parser crashes (>3 in 1 hour)
- Search functionality completely broken
- Database corruption detected
- Security vulnerability discovered

**Planned Rollback** (within 1 hour):
- Parse error rate 10-25% sustained for 2+ hours
- Performance degradation >50% sustained
- Memory leak confirmed
- User-reported critical issues
- Database performance impact severe

### Rollback Execution

**Per-Language Rollback** (if specific language has issues):

```bash
# Language-specific parsers are always integrated
# Rollback = stop indexing that language, no code change needed

# 1. Stop indexing for problematic language
# Document which repos to exclude

# 2. Verify other languages unaffected
./crewchief-maproom search "test function" # Should still work for other languages

# 3. Investigate issues offline
# 4. Re-deploy when fixed
```

**Complete Rollback** (all languages):

See [rollback_procedure.md](./rollback_procedure.md) for detailed steps.

Quick rollback:
1. Deploy previous binary version
2. Verify TypeScript/JavaScript functionality
3. Investigate issues
4. Total time: <5 minutes

## Troubleshooting Guide

### Issue: High Parse Error Rate

**Symptoms**: Error rate >5% for a language

**Investigation**:
```sql
-- Identify error patterns
SELECT
  error_type,
  COUNT(*) as count,
  MAX(error_message) as sample_message
FROM indexing_errors
WHERE language = 'py' -- or 'rs', 'go'
  AND created_at > NOW() - INTERVAL '1 hour'
GROUP BY error_type
ORDER BY count DESC;

-- Find problematic files
SELECT
  file_path,
  error_message,
  COUNT(*) as error_count
FROM indexing_errors
WHERE language = 'py'
  AND created_at > NOW() - INTERVAL '1 hour'
GROUP BY file_path, error_message
ORDER BY error_count DESC
LIMIT 10;
```

**Resolution**:
1. Check if errors are from specific repositories
2. Validate file encoding (should be UTF-8)
3. Test problematic files in isolation
4. Check for edge cases in syntax
5. Consider excluding problematic repos temporarily
6. File bug report with reproduction case

### Issue: Performance Degradation

**Symptoms**: Parse time >2x baseline

**Investigation**:
```sql
-- Performance analysis
SELECT
  language,
  AVG(duration_ms) as avg_ms,
  PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY duration_ms) as p95_ms,
  MAX(duration_ms) as max_ms
FROM parse_metrics
WHERE timestamp > NOW() - INTERVAL '1 hour'
GROUP BY language;

-- Find slow files
SELECT
  file_path,
  language,
  duration_ms,
  file_size_bytes
FROM parse_metrics
WHERE timestamp > NOW() - INTERVAL '1 hour'
ORDER BY duration_ms DESC
LIMIT 20;
```

**Resolution**:
1. Check if slowness is file-size related
2. Verify database not overloaded
3. Check system resources (CPU, memory, disk I/O)
4. Consider parsing optimizations for large files
5. Monitor for memory leaks

### Issue: Memory Growth

**Symptoms**: Memory usage increasing over time

**Investigation**:
```bash
# Check process memory
ps aux | grep crewchief-maproom

# Monitor memory over time
watch -n 60 'ps aux | grep crewchief-maproom | awk "{print \$6}"'

# Check for memory leaks
valgrind --leak-check=full ./crewchief-maproom scan --path /test/repo
```

**Resolution**:
1. Restart parser process to free memory (temporary)
2. Profile code for memory leaks
3. Check if large files causing memory spikes
4. Consider batch size adjustments
5. Monitor for improvement

## Success Metrics

**Rollout Complete When**:
- ✅ All three languages deployed (Python, Rust, Go)
- ✅ 48+ hours stable operation for each language
- ✅ Error rate <2% across all languages
- ✅ Performance within targets (>100 files/min)
- ✅ Memory usage stable (<500MB per process)
- ✅ Search quality validated (user feedback positive)
- ✅ No critical issues outstanding
- ✅ Documentation complete and published

**Quantitative Targets**:
- Parse success rate: >98%
- p95 parse time: <150ms
- Files/min throughput: >100 files/min
- Memory per process: <500MB
- Error resolution time: <1 hour for warnings, <15 min for critical
- Rollback time: <5 minutes if needed

**Qualitative Targets**:
- Users report improved search across languages
- Internal team confidence: High
- Monitoring visibility: Excellent
- Documentation quality: Production-ready
- Operational readiness: Team trained and prepared

---

**Document Version**: 1.0
**Last Updated**: 2025-10-25
**Rollout Status**: Ready to Execute
**Next Review**: After Stage 1 completion (Day 3)

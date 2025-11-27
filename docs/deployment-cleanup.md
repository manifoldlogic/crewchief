# Stale Worktree Cleanup - Deployment Guide

This guide covers the deployment procedures, rollback plans, and monitoring setup for the stale worktree cleanup feature.

## Pre-Deployment Checklist

Before deploying to any environment, verify:

- [ ] All unit tests passing (`cargo test -p crewchief-maproom`)
- [ ] All integration tests passing (`cleanup_detection_test.rs`, `cleanup_deletion_test.rs`, `cleanup_cli_test.rs`)
- [ ] Staging validation complete (IDXCLEAN-3004)
- [ ] Database backup procedure verified and tested
- [ ] Team briefed on feature behavior and rollback procedures
- [ ] Monitoring alerts configured (if applicable)
- [ ] Incident response team identified

## Deployment Phases

### Phase 1: Development Validation

**Objective:** Validate cleanup works correctly in development environment.

1. **Build the binary:**
   ```bash
   cargo build --release --bin crewchief-maproom
   ```

2. **Run dry-run to preview:**
   ```bash
   ./target/release/crewchief-maproom db cleanup-stale --verbose
   ```

3. **Review output:**
   - Verify detected stale worktrees are actually stale
   - Check paths don't exist: `ls -la /path/to/worktree`
   - Confirm no false positives (valid worktrees flagged)

4. **Execute cleanup:**
   ```bash
   ./target/release/crewchief-maproom db cleanup-stale --confirm --verbose
   ```

5. **Verify results:**
   - Search for previously duplicated content
   - Confirm cleanup reduced duplicates
   - Check database integrity with `status` command

### Phase 2: Staging Deployment

**Objective:** Validate cleanup in production-like environment.

1. **Deploy CLI binary to staging environment**

2. **Create database backup:**
   ```bash
   cp ~/.maproom/maproom.db ~/.maproom/maproom.db.backup.$(date +%Y%m%d)
   ```

3. **Run dry-run on staging:**
   ```bash
   crewchief-maproom db cleanup-stale --verbose > dry-run-results.txt
   ```

4. **Review dry-run results with team:**
   - Expected number of deletions
   - Sample verification of stale paths
   - Sign-off from team lead

5. **Execute cleanup:**
   ```bash
   crewchief-maproom db cleanup-stale --confirm --verbose 2>&1 | tee cleanup-execution.log
   ```

6. **Post-execution verification:**
   - Compare results to dry-run (should match)
   - Run search queries to verify quality improvement
   - Check for any errors in logs

7. **Monitor for 24 hours:**
   - Watch for error logs
   - Monitor search performance
   - Check for user reports of missing data

### Phase 3: Production Deployment

**Objective:** Deploy cleanup to production with full safeguards.

1. **Pre-deployment database backup:**
   ```bash
   # Verify backup
   cp ~/.maproom/maproom.db ~/.maproom/maproom.db.pre-cleanup.$(date +%Y%m%d)

   # Test restore works
   cp ~/.maproom/maproom.db.pre-cleanup.$(date +%Y%m%d) /tmp/test-restore.db
   sqlite3 /tmp/test-restore.db "SELECT COUNT(*) FROM worktrees;"
   ```

2. **Run dry-run on production:**
   ```bash
   crewchief-maproom db cleanup-stale --verbose > prod-dry-run.txt
   ```

3. **Get explicit team approval:**
   - Review dry-run results
   - Confirm deletion scope is expected
   - Document approval (email/Slack/ticket)

4. **Execute during low-traffic window:**
   ```bash
   # Morning or after-hours preferred
   crewchief-maproom db cleanup-stale --confirm --verbose 2>&1 | tee prod-cleanup.log
   ```

5. **Immediate verification:**
   - Check exit code (should be 0)
   - Review log for errors
   - Run sample search queries

6. **Extended monitoring (48 hours):**
   - Watch for user reports
   - Monitor search latency
   - Check error logs daily

### Phase 4: Watch Integration (Future)

**Note:** Watch integration is currently BLOCKED pending watch command reimplementation.

When available:

1. **Enable in staging:**
   ```bash
   export MAPROOM_AUTO_CLEANUP=true
   crewchief-maproom watch
   ```

2. **Monitor behavior:**
   - Startup cleanup runs in background
   - Periodic cleanup every 30 minutes
   - Only runs when indexer idle

3. **After 1 week of stability:**
   - Enable in production
   - Monitor for additional week
   - Document any issues

## Rollback Procedures

### CLI-Only Deployment

**Impact:** Minimal - cleanup only runs when explicitly invoked.

**Rollback:** Simply don't run the command. No changes needed.

### Watch Integration Rollback

**Disable automatic cleanup:**
```bash
export MAPROOM_AUTO_CLEANUP=false
# Or unset
unset MAPROOM_AUTO_CLEANUP
```

**Restart watch to apply:**
```bash
# Stop existing watch
pkill -f "crewchief-maproom watch"

# Start without auto-cleanup
crewchief-maproom watch
```

### Data Recovery

If data was incorrectly deleted:

1. **Stop any cleanup operations:**
   ```bash
   export MAPROOM_AUTO_CLEANUP=false
   ```

2. **Identify affected data from logs:**
   ```bash
   grep "Deleted:" cleanup-execution.log
   ```

3. **Restore from backup:**
   ```bash
   # Full database restore
   cp ~/.maproom/maproom.db.pre-cleanup ~/.maproom/maproom.db

   # Or selective restore (advanced)
   # See incident response playbook
   ```

4. **Re-index affected repositories (if backup unavailable):**
   ```bash
   crewchief-maproom scan --path /path/to/repo --repo myrepo --worktree main
   ```

## Configuration Reference

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `MAPROOM_DATABASE_URL` | `~/.maproom/maproom.db` | Database location |
| `MAPROOM_AUTO_CLEANUP` | `false` | Enable automatic cleanup in watch |
| `RUST_LOG` | `info` | Logging level (info, debug, trace) |

### CLI Flags

| Flag | Description |
|------|-------------|
| `--confirm` | Actually delete (required for execution) |
| `--verbose` | Show detailed progress |

### Exit Codes

| Code | Meaning | Action |
|------|---------|--------|
| `0` | Success | Continue |
| `1` | Error | Check logs, investigate |
| `2` | No stale worktrees | Normal - nothing to clean |

## Monitoring Configuration

### Error Alerts (Recommended Thresholds)

| Condition | Severity | Action |
|-----------|----------|--------|
| Cleanup exit code 1 | High | Investigate immediately |
| Cleanup duration > 10s | Medium | Check database size |
| Memory usage > 100MB | Low | Monitor trend |

### Log Queries

**Find all cleanup operations:**
```bash
grep -E "(Detecting|Deleted|Cleanup)" /var/log/maproom/*.log
```

**Find errors:**
```bash
grep -E "(ERROR|WARN|failed)" /var/log/maproom/*.log
```

**Audit trail (what was deleted):**
```bash
grep "Deleted:" /var/log/maproom/cleanup-*.log | sort -t: -k2
```

### Performance Baseline

From staging validation (IDXCLEAN-3004):

| Metric | Baseline | Alert Threshold |
|--------|----------|-----------------|
| Execution time | 14ms | > 2000ms |
| Detection query | < 10ms | > 500ms |
| Deletion per worktree | < 5ms | > 100ms |

## Support and Escalation

For issues with stale worktree cleanup:

1. **Check documentation:**
   - [User Guide](user-guide-cleanup.md)
   - [Admin Guide](admin-guide-cleanup.md)
   - [Incident Response](incident-response-cleanup.md)

2. **Review logs:**
   - `RUST_LOG=debug crewchief-maproom db cleanup-stale --verbose`

3. **Escalate if needed:**
   - See incident response playbook for escalation paths

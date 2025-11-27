# Incident Response Playbook - Stale Worktree Cleanup

This playbook covers incident response procedures for the stale worktree cleanup feature, including data recovery, escalation paths, and runbooks for common scenarios.

## Severity Levels

| Level | Description | Response Time | Examples |
|-------|-------------|---------------|----------|
| **P0 Critical** | Data deleted incorrectly, production impact | Immediate (< 15 min) | Valid worktrees deleted, search returning wrong results |
| **P1 High** | Cleanup errors, repeated failures | < 1 hour | Database connection failures, crash during cleanup |
| **P2 Medium** | Performance degradation | < 4 hours | Slow cleanup, high memory usage |
| **P3 Low** | Minor issues, monitoring alerts | < 24 hours | Warning logs, single failed cleanup |

## Initial Response Checklist

When an incident is detected:

- [ ] Identify severity level
- [ ] Stop any active cleanup operations (if P0/P1)
- [ ] Notify relevant team members
- [ ] Begin incident log (timestamp, symptoms, actions)
- [ ] Collect relevant logs and metrics
- [ ] Identify affected scope (users, repositories, data)

## Runbook: Incorrect Deletion (P0)

**Symptoms:**
- Valid worktrees reported as deleted in logs
- Search results missing expected content
- User reports data loss

**Immediate Actions:**

1. **STOP all cleanup operations:**
   ```bash
   export MAPROOM_AUTO_CLEANUP=false
   pkill -f "crewchief-maproom.*cleanup"
   ```

2. **Identify deleted records:**
   ```bash
   # Find what was deleted from logs
   grep "Deleted:" /var/log/maproom/*.log | tail -50

   # Extract worktree paths
   grep "Deleted:" cleanup-execution.log | awk '{print $2}'
   ```

3. **Assess impact:**
   - How many worktrees deleted?
   - Which repositories affected?
   - How many chunks lost?

4. **Initiate data recovery** (see Data Recovery Procedures below)

5. **Root cause analysis:**
   - Why was valid worktree detected as stale?
   - Was path existence check working?
   - Were there permission issues?

6. **Fix before re-enabling:**
   - Do NOT re-enable cleanup until fix is deployed
   - Test fix in development environment
   - Get team approval before production deployment

## Runbook: Cleanup Failures (P1)

**Symptoms:**
- Cleanup returns exit code 1
- Error messages in logs
- Cleanup doesn't complete

**Diagnostic Steps:**

1. **Check error logs:**
   ```bash
   RUST_LOG=debug crewchief-maproom db cleanup-stale --verbose 2>&1
   ```

2. **Verify database connectivity:**
   ```bash
   # Check database exists
   ls -la ~/.maproom/maproom.db

   # Check database integrity
   sqlite3 ~/.maproom/maproom.db "PRAGMA integrity_check;"

   # Check worktrees table
   sqlite3 ~/.maproom/maproom.db "SELECT COUNT(*) FROM worktrees;"
   ```

3. **Verify filesystem access:**
   ```bash
   # Check permissions on database
   ls -la ~/.maproom/

   # Check if worktree paths are accessible
   # (from dry-run output)
   ls -la /path/to/worktree
   ```

4. **Test in dry-run mode:**
   ```bash
   crewchief-maproom db cleanup-stale --verbose
   # If dry-run works, issue is with deletion logic
   # If dry-run fails, issue is with detection
   ```

**Resolution Steps:**

- **Database locked:** Wait for other processes, or restart database
- **Permission denied:** Check file ownership and permissions
- **Connection refused:** Check database path configuration
- **Corruption detected:** Restore from backup

## Runbook: Performance Issues (P2)

**Symptoms:**
- Cleanup takes > 10 seconds
- High memory usage (> 100MB)
- Database queries slow

**Diagnostic Steps:**

1. **Measure execution time:**
   ```bash
   time crewchief-maproom db cleanup-stale --verbose
   ```

2. **Check database size:**
   ```bash
   ls -lh ~/.maproom/maproom.db
   sqlite3 ~/.maproom/maproom.db "SELECT COUNT(*) FROM chunks;"
   sqlite3 ~/.maproom/maproom.db "SELECT COUNT(*) FROM worktrees;"
   ```

3. **Monitor memory:**
   ```bash
   # Run cleanup with memory monitoring
   /usr/bin/time -v crewchief-maproom db cleanup-stale --verbose
   ```

**Resolution Steps:**

- **Large database:** Consider more frequent cleanup schedules
- **Many stale worktrees:** Normal - performance scales with deletions
- **Memory growth:** Report as bug if unbounded growth

## Data Recovery Procedures

### Option 1: Full Database Restore

**When to use:** Complete data loss, multiple worktrees affected, no time pressure.

```bash
# 1. Stop any maproom processes
pkill -f crewchief-maproom

# 2. Locate backup
ls -la ~/.maproom/maproom.db.backup.*

# 3. Restore database
cp ~/.maproom/maproom.db.backup.20251127 ~/.maproom/maproom.db

# 4. Verify restore
crewchief-maproom status --repo myrepo

# 5. Re-run cleanup with correct configuration (if needed)
```

### Option 2: Selective Re-indexing

**When to use:** Specific worktrees affected, backup unavailable, quick recovery needed.

```bash
# 1. Identify affected worktrees from logs
grep "Deleted:" cleanup-execution.log

# 2. Re-index each affected worktree
crewchief-maproom scan --path /path/to/worktree --repo myrepo --worktree branch-name

# 3. Regenerate embeddings (if using semantic search)
crewchief-maproom generate-embeddings --repo myrepo

# 4. Verify recovery
crewchief-maproom search --query "test" --repo myrepo
```

### Option 3: Incremental Upsert

**When to use:** Files changed since backup, partial recovery needed.

```bash
# Re-index changed files only
crewchief-maproom upsert \
  --paths /path/to/file1.rs /path/to/file2.rs \
  --repo myrepo \
  --worktree main \
  --root /path/to/repo \
  --commit HEAD
```

## Escalation Path

### On-Call Response

| Time | Action |
|------|--------|
| 0-5 min | On-call engineer assesses severity |
| 5-15 min | Begin initial response checklist |
| 15 min | Notify team lead (P0/P1 incidents) |
| 30 min | Status update to team |
| 1 hour | Escalate if not resolved (P0) |

### Communication Template

**Initial Report:**
```
INCIDENT: Stale Worktree Cleanup Issue
Severity: P[X]
Time Detected: [timestamp]
Symptoms: [brief description]
Impact: [affected users/repos]
Status: Investigating
Next Update: [time]
```

**Resolution Report:**
```
RESOLVED: Stale Worktree Cleanup Issue
Duration: [total time]
Root Cause: [description]
Resolution: [what was done]
Follow-up: [any pending actions]
```

## Post-Incident Review

After resolving a P0 or P1 incident:

1. **Document timeline:**
   - When detected
   - When each action taken
   - When resolved

2. **Root cause analysis:**
   - What failed?
   - Why wasn't it caught earlier?
   - How did it reach production?

3. **Action items:**
   - Code fixes
   - Test improvements
   - Monitoring enhancements
   - Documentation updates

4. **Review meeting:**
   - Schedule within 48 hours
   - Include all involved parties
   - Focus on prevention, not blame

## Prevention Measures

### Before Each Cleanup

- [ ] Verify backup exists and is recent
- [ ] Run dry-run first
- [ ] Review dry-run output
- [ ] Get team approval for large deletions

### Regular Maintenance

- Weekly: Review cleanup logs for patterns
- Monthly: Verify backup/restore procedure works
- Quarterly: Review and update this playbook

### Monitoring Alerts

Set up alerts for:
- Cleanup exit code != 0 or 2
- Cleanup duration > 10 seconds
- Unusual deletion counts (> 2x historical average)
- Memory usage spikes during cleanup

## Contact Information

Update with your team's actual contacts:

| Role | Contact |
|------|---------|
| On-Call Engineer | [rotation schedule link] |
| Team Lead | [name/contact] |
| Database Admin | [name/contact] |

## Related Documentation

- [User Guide](user-guide-cleanup.md)
- [Admin Guide](admin-guide-cleanup.md)
- [Deployment Guide](deployment-cleanup.md)

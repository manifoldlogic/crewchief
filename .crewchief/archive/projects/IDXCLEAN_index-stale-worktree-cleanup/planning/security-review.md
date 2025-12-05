# Security Review: Index Stale Worktree Cleanup

## Security Overview

**Primary Security Concern:** Data integrity and prevention of accidental data loss.

This project involves **irreversible deletion** of database records. The security review focuses on preventing unauthorized deletions, accidental data loss, and ensuring audit trail for recovery.

**Risk Profile:**
- **Severity:** High (data loss is permanent)
- **Likelihood:** Medium (user-initiated, requires confirmation)
- **Overall Risk:** Medium-High

**Security Principles:**
1. **Defense in depth:** Multiple safety layers prevent accidental deletion
2. **Least privilege:** Cleanup requires explicit permission
3. **Auditability:** All actions logged for accountability
4. **Fail-safe defaults:** Dry-run mode is default behavior

---

## Threat Model

### Threat 1: Accidental Deletion of Valid Worktree

**Attack Scenario:**
- User runs cleanup command
- Validation incorrectly identifies valid worktree as stale
- Valid worktree and all its chunks are deleted
- Search results no longer show code from that worktree

**Impact:** High
- Permanent data loss
- Requires re-indexing to recover
- User frustration and loss of trust

**Threat Actor:**
- Legitimate user (accidental)
- Automated system (bug in validation logic)

**Attack Vector:**
- Bug in disk existence check
- Race condition (worktree deleted during validation)
- Network mount temporarily unavailable
- File system error (permission denied treated as non-existent)

**Mitigations:**

1. **Validation accuracy:**
   ```rust
   // Robust existence check with error handling
   async fn validate_path_exists(path: &str) -> Result<bool> {
       match tokio::fs::try_exists(path).await {
           Ok(exists) => Ok(exists),
           Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
               // Permission denied ≠ doesn't exist
               tracing::warn!(path = %path, "Permission denied when checking path");
               Ok(true) // Assume exists to be safe
           }
           Err(e) => {
               // Other errors: log but don't treat as non-existent
               tracing::error!(path = %path, error = %e, "Error checking path existence");
               Err(e.into())
           }
       }
   }
   ```

2. **Dry-run default:**
   ```rust
   // CLI requires explicit --confirm flag
   #[derive(Parser)]
   pub struct CleanupStaleCommand {
       #[arg(long, help = "Actually delete (default is dry-run)")]
       confirm: bool, // Defaults to false
   }
   ```

3. **User review before deletion:**
   ```bash
   # Dry-run shows what WOULD be deleted
   $ maproom db cleanup-stale
   📊 Found 95 stale worktrees:
     • experiment-1 (worktree_id=42, chunks=5230)
     • experiment-2 (worktree_id=43, chunks=4821)
     ...
   ⚠️  This was a dry-run. Use --confirm to actually delete.

   # User reviews list, decides to proceed
   $ maproom db cleanup-stale --confirm
   ```

4. **Audit logging:**
   ```rust
   // Every deletion logged with full context
   tracing::info!(
       worktree_id = wt.id,
       name = %wt.name,
       abs_path = %wt.abs_path,
       chunk_count = wt.chunk_count,
       validated_nonexistent = !wt.exists,
       "Deleting stale worktree"
   );
   ```

**Residual Risk:** Low
- Multiple safety layers in place
- Dry-run catches issues before damage
- Audit trail allows recovery

### Threat 2: Database Corruption During Cleanup

**Attack Scenario:**
- Cleanup begins transaction
- Some worktrees deleted successfully
- Database error occurs mid-transaction
- Transaction not properly rolled back
- Database left in inconsistent state

**Impact:** High
- Partial data loss
- Orphaned chunks without worktrees
- Search results inconsistent

**Threat Actor:**
- Database malfunction (hardware failure, OOM)
- Network interruption (remote database)
- Concurrent write conflict

**Attack Vector:**
- Transaction not properly handled
- Network partition during commit
- Database constraint violation
- Out-of-memory during bulk delete

**Mitigations:**

1. **Transaction safety:**
   ```rust
   pub async fn cleanup_stale_worktrees(&self, stale: Vec<StaleWorktree>) -> Result<CleanupReport> {
       // Begin transaction
       let mut tx = self.db.begin_transaction().await?;

       // All deletions within transaction
       for wt in stale {
           self.delete_worktree_tx(&mut tx, wt.id).await?;
       }

       // Commit atomically (all-or-nothing)
       tx.commit().await?;

       Ok(report)
   }
   ```

2. **Rollback on error:**
   ```rust
   // If any operation fails, transaction auto-rolls back
   match tx.commit().await {
       Ok(_) => {
           tracing::info!("Transaction committed successfully");
           Ok(report)
       }
       Err(e) => {
           // Transaction rolled back automatically
           tracing::error!(error = %e, "Transaction failed, rolled back");
           Err(e)
       }
   }
   ```

3. **Foreign key constraints:**
   ```sql
   -- Database ensures referential integrity
   CREATE TABLE chunks (
       id SERIAL PRIMARY KEY,
       worktree_id INTEGER REFERENCES worktrees(id) ON DELETE CASCADE,
       -- ...
   );

   -- CASCADE ensures chunks deleted when worktree deleted
   -- No orphaned chunks possible
   ```

4. **Connection pooling:**
   ```rust
   // Use connection pool to handle transient failures
   let pool = sqlx::postgres::PgPoolOptions::new()
       .max_connections(5)
       .connect(&database_url)
       .await?;
   ```

**Residual Risk:** Very Low
- PostgreSQL transactions are ACID-compliant
- Automatic rollback on failure
- Foreign key constraints prevent orphans

### Threat 3: Unauthorized Deletion

**Attack Scenario:**
- Attacker gains access to user's terminal
- Runs `maproom db cleanup-stale --confirm`
- Deletes all worktrees from database
- Search functionality completely broken

**Impact:** Critical
- Total data loss
- Requires full re-index to recover
- Service outage until recovery

**Threat Actor:**
- Malicious user with terminal access
- Compromised account
- Insider threat

**Attack Vector:**
- Physical access to unlocked machine
- Stolen credentials (SSH, sudo)
- Social engineering
- Malware running commands

**Mitigations:**

1. **Operating system authentication:**
   - Relies on OS-level access control
   - User must have permission to run `maproom` binary
   - Database credentials required (from config file)

2. **Database authentication:**
   ```toml
   # Config file requires valid database credentials
   [database]
   url = "postgresql://maproom:password@localhost:5432/maproom"
   ```

3. **Confirmation requirement:**
   ```bash
   # Explicit --confirm flag required
   # Prevents accidental copy-paste attacks
   $ maproom db cleanup-stale --confirm
   ```

4. **Audit logging:**
   ```rust
   // All cleanup operations logged
   // Security team can review logs for suspicious activity
   tracing::info!(
       user = std::env::var("USER").unwrap_or_default(),
       operation = "cleanup_stale_worktrees",
       deleted_count = report.deleted_count,
       "Cleanup operation completed"
   );
   ```

5. **No remote execution:**
   - CLI is local-only (not exposed via MCP)
   - Watch integration respects same safety checks
   - No API endpoint for cleanup (reduces attack surface)

**Residual Risk:** Medium
- Relies on OS-level security
- No additional authentication layer
- Acceptable for internal tool

### Threat 4: Denial of Service via Excessive Cleanup

**Attack Scenario:**
- Attacker triggers watch cleanup repeatedly
- Cleanup runs continuously, consuming resources
- Database overwhelmed with DELETE queries
- Indexing operations blocked or delayed

**Impact:** Medium
- Service degradation (indexing slow)
- Resource exhaustion (CPU, I/O)
- User experience degraded

**Threat Actor:**
- Malicious user with configuration access
- Buggy code causing infinite loop

**Attack Vector:**
- Modified cleanup configuration (interval = 0)
- Bug in rate limiting logic
- Watch command restarted repeatedly
- Concurrent cleanup processes

**Mitigations:**

1. **Rate limiting:**
   ```rust
   async fn should_run_cleanup(&self) -> bool {
       let last = self.last_cleanup.read().await;
       match *last {
           None => true,
           Some(instant) => {
               // Minimum 15 minutes between cleanups
               instant.elapsed() > Duration::from_secs(900)
           }
       }
   }
   ```

2. **Busy detection:**
   ```rust
   async fn run_cleanup_if_safe(&self) -> Result<()> {
       // Don't run if indexer is busy
       if self.indexer.active_operations() > 0 {
           tracing::debug!("Deferring cleanup: indexer busy");
           return Ok(());
       }

       // Don't run if database under load
       if self.db.query_queue_depth() > 10 {
           tracing::debug!("Deferring cleanup: database busy");
           return Ok(());
       }

       self.run_cleanup().await
   }
   ```

3. **Configuration limits:**
   ```toml
   # Enforced minimum intervals
   [cleanup]
   cleanup_interval = 1800  # Minimum 30 minutes (enforced in code)
   cleanup_cooldown = 900   # Minimum 15 minutes (enforced in code)
   ```

4. **Single instance:**
   ```rust
   // Watch manager ensures only one cleanup task running
   pub struct WatchManager {
       cleanup_handle: Option<tokio::task::JoinHandle<()>>,
   }

   impl WatchManager {
       pub async fn start_watch(&self) {
           // Abort previous cleanup if exists
           if let Some(handle) = &self.cleanup_handle {
               handle.abort();
           }

           // Start new cleanup task
           self.cleanup_handle = Some(tokio::spawn(/* ... */));
       }
   }
   ```

**Residual Risk:** Low
- Rate limiting prevents abuse
- Busy detection prevents interference
- Configuration enforced by code

---

## Architecture Security Analysis

### Component: Stale Detection Module

**Security Properties:**
- Read-only database access (SELECT queries only)
- No write operations (safe to run repeatedly)
- Fails gracefully on disk errors
- No sensitive data leaked in errors

**Vulnerabilities:**
- None identified (read-only operations)

**Recommendations:**
- ✅ Current design is secure
- Consider: Cache validation results to reduce I/O

### Component: Safe Deletion Module

**Security Properties:**
- Transactional writes (ACID guarantees)
- Explicit dry-run mode (default)
- Audit logging (every deletion)
- Error handling (graceful failure)

**Vulnerabilities:**
- Risk: Transaction not properly rolled back
  - Mitigation: Use sqlx transaction API (auto-rollback)
- Risk: Partial commit (some deletions succeed)
  - Mitigation: Single transaction for all deletions

**Recommendations:**
- ✅ Current design is secure
- ✅ Transaction safety properly implemented
- Consider: Add database backup recommendation to docs

### Component: CLI Command Interface

**Security Properties:**
- Requires explicit confirmation (--confirm flag)
- Shows preview before deletion (dry-run default)
- Clear user feedback (deleted count, errors)
- No password prompts (uses config file)

**Vulnerabilities:**
- Risk: User doesn't review dry-run output
  - Mitigation: Prominent warning message
- Risk: --confirm flag added via alias/script
  - Mitigation: User education, documentation

**Recommendations:**
- ✅ Current design is secure
- Consider: Add "Are you sure?" prompt even with --confirm
- Consider: Require typing "yes" for large deletions (>50 worktrees)

### Component: Watch Integration

**Security Properties:**
- Respects same safety checks as manual cleanup
- Rate limited (can't run excessively)
- Background execution (doesn't block)
- Configuration-based (user can disable)

**Vulnerabilities:**
- Risk: Runs without user awareness
  - Mitigation: Log all automatic cleanups
- Risk: Configuration tampered with
  - Mitigation: Config file has OS-level permissions

**Recommendations:**
- ✅ Current design is secure
- Consider: Add "last cleanup" timestamp to status output
- Consider: Notification when automatic cleanup runs

---

## Known Gaps and Risk Evaluation

### Gap 1: No Authentication Beyond OS

**Description:**
Cleanup operations rely solely on operating system authentication. Any user who can run `maproom` binary can run cleanup.

**Risk Level:** Medium
- Internal tool with trusted users
- Database credentials required (stored in config)
- Audit trail available for review

**MVP Decision:** Accept this gap
- Adding authentication is out of scope
- OS-level security is standard for CLI tools
- Enterprise deployment can use RBAC at database level

**Future Mitigation:**
- Add role-based access control (DBA only)
- Require sudo for cleanup operations
- Integrate with identity management system

### Gap 2: No Soft Delete or Undo

**Description:**
Deleted worktrees are permanently removed. No "trash" or "undo" mechanism exists.

**Risk Level:** Medium
- Re-indexing can recover data (but slow)
- Audit logs allow identifying what was deleted
- Dry-run prevents most accidents

**MVP Decision:** Accept this gap
- Soft delete adds complexity (extra state)
- Database backups provide recovery mechanism
- Explicit confirmation is sufficient safeguard

**Future Mitigation:**
- Add soft delete with `deleted_at` timestamp
- Implement "undelete" command (restore from soft delete)
- Automatic purge of soft-deleted after 30 days

### Gap 3: No Backup Integration

**Description:**
Cleanup doesn't automatically create backup before deletion. Recovery relies on external backup system.

**Risk Level:** Low
- Database backups should already exist (operations best practice)
- Adding backup logic complicates deployment
- Dry-run allows user to backup manually if desired

**MVP Decision:** Accept this gap
- Document backup recommendation in README
- Rely on existing database backup procedures
- Consider for future enhancement

**Future Mitigation:**
- Add `--backup` flag to create snapshot before cleanup
- Integrate with pg_dump for automatic backups
- Store backup metadata in database

### Gap 4: Limited Recovery Assistance

**Description:**
If cleanup accidentally deletes valid worktree, no built-in recovery assistance. User must re-index or restore from backup.

**Risk Level:** Low
- Dry-run and confirmation prevent most accidents
- Re-indexing is straightforward (maproom scan)
- Database backups available for worst case

**MVP Decision:** Accept this gap
- Focus on prevention over recovery
- Document recovery procedures in README
- Manual re-index is acceptable for rare case

**Future Mitigation:**
- Add "recovery" command to re-index specific worktree
- Store deleted worktree metadata for reference
- Provide rollback script for recent cleanups

---

## Security Best Practices

### Defense in Depth

**Layer 1: Validation**
- Disk existence check with robust error handling
- Multiple validation passes (detection phase, deletion phase)
- Permission errors treated as "exists" (safe)

**Layer 2: User Confirmation**
- Dry-run is default behavior
- Explicit --confirm flag required
- Preview shows exactly what will be deleted

**Layer 3: Transaction Safety**
- All deletions in single transaction
- Automatic rollback on error
- Foreign key constraints prevent orphans

**Layer 4: Audit Trail**
- Every deletion logged with full context
- Structured logging (easy to search/analyze)
- Logs retained per system logging policy

**Layer 5: Rate Limiting**
- Minimum 15 minutes between automatic cleanups
- Busy detection prevents interference
- Configuration enforced by code

### Least Privilege

**Database permissions:**
```sql
-- Cleanup requires DELETE permission
GRANT SELECT, DELETE ON worktrees TO maproom_user;
GRANT SELECT ON chunks TO maproom_user; -- CASCADE handles deletion

-- Read-only users cannot run cleanup
GRANT SELECT ON worktrees TO maproom_readonly;
```

**Configuration access:**
```bash
# Config file readable only by owner
$ chmod 600 ~/.maproom-mcp/config.toml
```

**Execution permission:**
```bash
# Binary executable only by owner/group
$ chmod 750 /usr/local/bin/maproom
```

### Auditability

**Structured logging:**
```rust
// All cleanup operations logged
tracing::info!(
    operation = "cleanup_stale_worktrees",
    user = std::env::var("USER").unwrap_or_default(),
    hostname = hostname::get().unwrap_or_default(),
    deleted_count = report.deleted_count,
    failed_count = report.failed_count,
    deleted_ids = ?report.deleted_ids,
    timestamp = chrono::Utc::now().to_rfc3339(),
    "Cleanup completed"
);
```

**Log aggregation:**
- Integrate with syslog (if available)
- Export to log management system (Splunk, ELK)
- Retain logs per compliance requirements

**Incident response:**
- Logs allow reconstructing what was deleted
- Audit trail assists with recovery
- Timestamps enable correlation with backups

### Fail-Safe Defaults

**Dry-run default:**
```rust
// Default to safe behavior
impl Default for CleanupStaleCommand {
    fn default() -> Self {
        Self {
            confirm: false, // Dry-run unless explicitly confirmed
            verbose: false,
        }
    }
}
```

**Conservative validation:**
```rust
// When in doubt, assume path exists (safe)
match tokio::fs::try_exists(path).await {
    Ok(exists) => exists,
    Err(e) if e.kind() == ErrorKind::PermissionDenied => true, // Safe
    Err(e) => {
        tracing::warn!("Error checking path, assuming exists: {}", e);
        true // Safe
    }
}
```

**Rate limiting:**
```rust
// Default to conservative limits
impl Default for CleanupConfig {
    fn default() -> Self {
        Self {
            auto_cleanup: true,
            cleanup_interval: Duration::from_secs(1800), // 30 min
            cleanup_cooldown: Duration::from_secs(900),  // 15 min
            batch_size: 50, // Conservative
        }
    }
}
```

---

## Compliance Considerations

### Data Retention

**Policy Adherence:**
- Cleanup aligns with data retention policies
- Stale data (non-existent worktrees) has no retention value
- Active data (valid worktrees) preserved

**Audit Requirements:**
- All deletions logged (audit trail)
- Logs retained per retention policy
- Recovery procedures documented

### Access Control

**Current Implementation:**
- OS-level authentication (user must have maproom access)
- Database authentication (credentials in config)
- No additional access control layer

**Recommendations:**
- Document minimum required permissions
- Provide RBAC guidelines for enterprise deployments
- Consider role separation (read-only vs. admin)

### Change Management

**Deployment Process:**
- Test on staging database first
- Manual validation before production
- Staged rollout (manual → automatic)
- Rollback plan documented

**Documentation:**
- User guide for cleanup command
- Administrator guide for watch integration
- Recovery procedures for accidents
- Incident response playbook

---

## Production Deployment Security

### Pre-Deployment Checklist

**Security Review:**
- [ ] Code review completed (focus on deletion logic)
- [ ] Security testing completed (try to break it)
- [ ] Audit logging verified (all deletions logged)
- [ ] Transaction safety verified (rollback works)
- [ ] Rate limiting verified (can't abuse)

**Infrastructure:**
- [ ] Database backups confirmed (daily backups exist)
- [ ] Log aggregation configured (logs sent to SIEM)
- [ ] Monitoring configured (alert on errors)
- [ ] Access control reviewed (who can run cleanup)
- [ ] Documentation updated (security procedures)

**Validation:**
- [ ] Dry-run tested on staging (review output)
- [ ] Actual deletion tested on staging (verify success)
- [ ] Recovery tested (restore from backup works)
- [ ] Performance tested (no degradation)
- [ ] Error handling tested (failures handled gracefully)

### Deployment Phases

**Phase 1: Manual cleanup only (Week 1)**
- Deploy CLI command
- Enable in production (--confirm required)
- Monitor logs for issues
- Gather user feedback
- **Risk:** Low (user-initiated, explicit confirmation)

**Phase 2: Watch integration (Week 2-3)**
- Enable startup cleanup (conservative rate limiting)
- Monitor performance impact
- Watch for errors in logs
- Verify no interference with indexing
- **Risk:** Medium (automatic, less user oversight)

**Phase 3: Periodic cleanup (Week 4+)**
- Enable background cleanup (30-minute interval)
- Monitor resource usage
- Tune configuration based on metrics
- Gradually reduce cooldown if stable
- **Risk:** Medium (more frequent, automatic)

### Incident Response

**Scenario: Accidental deletion of valid worktree**

1. **Detection:**
   - User reports missing search results
   - Check audit logs for recent cleanup operations
   - Identify deleted worktree from logs

2. **Assessment:**
   - Determine impact (how many chunks lost)
   - Check if data available on disk (worktree still exists)
   - Identify backup to restore from

3. **Recovery:**
   - Option A: Re-index worktree (if still on disk)
     ```bash
     maproom scan /path/to/worktree
     ```
   - Option B: Restore from database backup (if worktree deleted)
     ```bash
     pg_restore -d maproom backup.dump
     ```

4. **Prevention:**
   - Review validation logic (why false positive?)
   - Add test case to prevent regression
   - Update documentation with lessons learned

**Scenario: Database corruption during cleanup**

1. **Detection:**
   - Cleanup command reports transaction error
   - Audit logs show failed commit
   - Database queries return unexpected results

2. **Assessment:**
   - Check transaction status (committed or rolled back?)
   - Verify referential integrity (orphaned chunks?)
   - Identify extent of corruption

3. **Recovery:**
   - If transaction rolled back: No action needed (ACID guarantees)
   - If partial commit: Restore from backup
   - Run database integrity checks (VACUUM, ANALYZE)

4. **Prevention:**
   - Review transaction handling code
   - Add stress tests for concurrent operations
   - Monitor database health metrics

---

## Security Summary

### Risk Assessment

| Risk | Likelihood | Impact | Mitigation | Residual Risk |
|------|-----------|--------|------------|---------------|
| Accidental deletion | Medium | High | Dry-run + confirmation | Low |
| Database corruption | Low | High | Transactions + CASCADE | Very Low |
| Unauthorized access | Low | Critical | OS + DB auth | Medium |
| Denial of service | Very Low | Medium | Rate limiting | Low |

### Security Posture

**Strengths:**
- ✅ Multiple layers of defense (validation, confirmation, transactions)
- ✅ Audit trail for accountability and recovery
- ✅ Fail-safe defaults (dry-run, conservative limits)
- ✅ Transaction safety prevents corruption

**Weaknesses:**
- ⚠️ No authentication beyond OS level
- ⚠️ No soft delete or undo mechanism
- ⚠️ No automatic backup integration
- ⚠️ Recovery relies on external systems

**Overall Assessment:** **Acceptable for MVP**
- Core safety mechanisms are robust
- Identified gaps are low-risk for internal tool
- Future enhancements can address remaining gaps

### Recommendations for Production

**Immediate (MVP):**
1. Document backup procedures in README
2. Add prominent warning about data deletion
3. Test extensively on staging database
4. Monitor audit logs in production

**Short-term (Post-MVP):**
1. Add "last cleanup" timestamp to status output
2. Implement soft delete with configurable purge delay
3. Add backup integration (optional --backup flag)
4. Create incident response playbook

**Long-term (Future):**
1. Add role-based access control (RBAC)
2. Implement undelete command (restore soft-deleted)
3. Create recovery assistant tool
4. Integrate with enterprise monitoring systems

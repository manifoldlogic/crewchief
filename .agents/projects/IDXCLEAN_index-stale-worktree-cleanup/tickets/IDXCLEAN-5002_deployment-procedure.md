# Ticket: IDXCLEAN-5002: Create Deployment Procedure and Monitoring Setup

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation-only ticket)
- [x] **Verified** - by the verify-ticket agent

## Agents
- docs-writer
- verify-ticket
- commit-ticket

## Summary
Create comprehensive deployment procedures, rollback plans, monitoring configuration, and incident response playbook for the stale worktree cleanup feature. This ensures safe production deployment with proper safeguards and recovery mechanisms.

## Background
The stale worktree cleanup feature involves automated data deletion in production. This requires careful deployment procedures to minimize risk, robust monitoring to detect issues early, and clear incident response plans for data recovery.

This ticket implements Phase 5 - Production Deployment (IDXCLEAN-5002) from plan.md (lines 728-754), specifically the deployment checklist, rollback procedures, and monitoring setup requirements.

## Acceptance Criteria
- [x] Deployment checklist created with phased rollout steps (`docs/deployment-cleanup.md`)
- [x] Rollback procedure documented with data recovery steps (in deployment guide)
- [x] Monitoring configuration specified (error alerts, performance metrics) (in deployment guide)
- [x] Log aggregation configured for audit trail (in deployment guide)
- [x] Performance baseline established for cleanup operations (from IDXCLEAN-3004: 14ms)
- [x] Incident response playbook created with escalation paths (`docs/incident-response-cleanup.md`)

## Technical Requirements
- Phased rollout plan (staging → production → watch integration)
- Database backup verification required before cleanup execution
- Monitoring must alert on cleanup errors and unexpected deletions
- Log queries for audit trail of all cleanup operations
- Performance metrics baseline (execution time, records deleted)
- Incident response must include data recovery from backups
- Dry-run requirement before production execution

## Implementation Notes

### Deployment Document Structure (`docs/deployment-cleanup.md`)

```markdown
# Stale Worktree Cleanup - Deployment Guide

## Pre-Deployment Checklist
- [ ] All tests passing (unit + integration)
- [ ] IDXCLEAN-3004 staging validation complete
- [ ] Database backup verified and tested
- [ ] Team briefed on feature and rollback procedures
- [ ] Monitoring configured and tested
- [ ] Incident response team identified

## Phase 1: Staging Deployment
1. Deploy CLI command to staging
2. Run dry-run: `maproom db cleanup-stale`
3. Review results with team (expected deletions)
4. Execute: `maproom db cleanup-stale --confirm`
5. Verify cleanup results match dry-run
6. Monitor for 24 hours (error logs, performance)

## Phase 2: Production CLI Deployment
1. Deploy CLI command only (no watch integration)
2. Create database backup
3. Run dry-run on production
4. Get team approval for deletion scope
5. Execute with confirmation during low-traffic window
6. Verify results immediately
7. Monitor for 48 hours

## Phase 3: Watch Integration Rollout
1. Enable MAPROOM_AUTO_CLEANUP=true in staging
2. Monitor startup cleanup behavior
3. Monitor periodic cleanup (tune intervals)
4. Enable in production after 1 week of stability
5. Monitor production for 1 week

## Rollback Procedures
- CLI-only deployment: Simply don't execute command
- Watch integration: Set MAPROOM_AUTO_CLEANUP=false
- Data recovery: Restore from backup (see incident response)

## Configuration
- Environment variables
- Timing intervals (default: 24h)
- Dry-run vs. confirmation mode
```

### Monitoring Configuration

```markdown
## Error Alerts
- Cleanup execution failures
- Unexpected deletion counts (>X% variance)
- Database connection errors during cleanup
- Filesystem access errors

## Performance Metrics
- Cleanup execution time
- Records deleted per execution
- Database query performance
- Memory usage during cleanup

## Log Aggregation
- All cleanup operations (timestamp, worktree, records deleted)
- All errors with stack traces
- Dry-run results for audit
- Configuration changes
```

### Incident Response Playbook (`docs/incident-response-cleanup.md`)

```markdown
# Incident Response - Stale Worktree Cleanup

## Severity Levels
- **P0 Critical**: Data deleted incorrectly, production down
- **P1 High**: Cleanup errors, repeated failures
- **P2 Medium**: Performance degradation
- **P3 Low**: Monitoring alerts, minor issues

## Data Recovery Procedures
1. Identify affected worktrees from logs
2. Locate backup prior to cleanup
3. Extract affected repositories/chunks from backup
4. Restore to database (upsert operation)
5. Verify restoration with search queries
6. Document incident and root cause

## Escalation Path
1. On-call engineer investigates
2. Team lead notified within 15 minutes (P0/P1)
3. Database backup team engaged for recovery
4. Post-incident review scheduled

## Runbook: Incorrect Deletion
1. IMMEDIATELY disable cleanup (MAPROOM_AUTO_CLEANUP=false)
2. Identify deleted records from logs
3. Assess impact (how many chunks/repos lost)
4. Initiate data recovery procedure
5. Root cause analysis
6. Fix and test before re-enabling

## Runbook: Cleanup Failures
1. Check error logs for root cause
2. Verify database connectivity
3. Verify filesystem access
4. Test cleanup in dry-run mode
5. Fix underlying issue
6. Resume cleanup with monitoring
```

### Performance Baseline

Document baseline metrics from staging validation (IDXCLEAN-3004):
- Average cleanup execution time
- Typical number of stale worktrees detected
- Database query performance (detection + deletion)
- Memory usage profile

## Dependencies
- IDXCLEAN-5001 (user documentation must exist)
- IDXCLEAN-3004 (staging validation provides baseline metrics)

## Risk Assessment
- **Risk**: Incomplete procedures lead to incorrect deployment
  - **Mitigation**: Peer review of all procedures, dry-run in staging first

- **Risk**: Insufficient monitoring misses production issues
  - **Mitigation**: Test all monitoring alerts in staging, establish clear thresholds

- **Risk**: Data loss due to incorrect cleanup
  - **Mitigation**: Mandatory dry-run, backup verification, phased rollout

- **Risk**: Inadequate incident response delays recovery
  - **Mitigation**: Practice data recovery in staging, document all steps clearly

## Files/Packages Affected
- `docs/deployment-cleanup.md` (new file) - Complete deployment guide with phased rollout
- `docs/incident-response-cleanup.md` (new file) - Incident response playbook with runbooks

## Implementation Notes

### Created Documentation

**docs/deployment-cleanup.md** covers:
- Pre-deployment checklist (tests, backups, team briefing)
- Phase 1: Development validation
- Phase 2: Staging deployment with 24h monitoring
- Phase 3: Production deployment with 48h monitoring
- Phase 4: Watch integration (future - blocked pending watch reimplementation)
- Rollback procedures for CLI and watch integration
- Data recovery options (full restore, selective re-index, upsert)
- Configuration reference (env vars, CLI flags, exit codes)
- Monitoring configuration (alerts, log queries, performance baseline)

**docs/incident-response-cleanup.md** covers:
- Severity levels (P0-P3) with response times
- Initial response checklist
- Runbook: Incorrect Deletion (P0) - immediate stop, identify, recover
- Runbook: Cleanup Failures (P1) - diagnostics and resolution
- Runbook: Performance Issues (P2) - monitoring and optimization
- Data recovery procedures (3 options)
- Escalation path with communication templates
- Post-incident review process
- Prevention measures and maintenance schedule

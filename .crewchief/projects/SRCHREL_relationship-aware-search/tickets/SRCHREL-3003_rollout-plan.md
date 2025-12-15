# Ticket: SRCHREL-3003 - Rollout Plan and Procedures

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- ops-engineer
- verify-ticket
- commit-ticket

## Summary

Create detailed rollout plan for production deployment of quality-weighted graph scoring, including staged rollout, monitoring checkpoints, and rollback procedures.

## Acceptance Criteria

- [ ] Document 4-stage rollout plan (deploy disabled, internal testing, production enable, stabilization)
- [ ] Define success criteria for each stage
- [ ] Document monitoring checkpoints (6h, 24h, 48h after enable)
- [ ] Create rollback procedure with <15 minute target
- [ ] Define rollback trigger conditions (latency, errors, user complaints)
- [ ] Identify rollback authority (who can execute)
- [ ] Test rollback procedure in staging
- [ ] Document post-rollout validation steps

## Technical Requirements

**Rollout Stages:**

```markdown
## Stage 1: Deploy with Flag Disabled (Week 5-6)
- Deploy enhanced executor code to production
- Feature flag: `enable_quality_scoring: false`
- Duration: 2-3 days
- Success: No errors, no performance regressions, flag toggle works

## Stage 2: Internal Testing (Week 6)
- Enable flag in staging environment
- Run test queries against staging database
- Monitor metrics (latency, score distributions)
- Duration: 2-3 days
- Success: Latency <35ms p95, rankings improved on test queries

## Stage 3: Production Enable (Week 6-7)
- Set flag: `enable_quality_scoring: true` in production
- Monitoring period: 24-48 hours intensive
- Checkpoints: 6h, 24h, 48h after enable
- Success: Latency <35ms p95, error rate <0.5%, no user complaints

## Stage 4: Stabilization (Week 7+)
- Continue monitoring for 1 week
- Gather user feedback
- Tune weights if needed based on metrics
- Plan: Remove feature flag after 1 month of stability
```

**Rollback Procedure:**

```markdown
## Trigger Conditions

**Automatic Rollback** (if implemented):
- Graph executor p95 latency >50ms for 10+ minutes
- Error rate >5% for 5+ minutes

**Manual Rollback** (engineer decision):
- Graph executor p95 latency >40ms sustained
- Error rate >1% sustained
- User complaints about ranking quality
- Unexpected behavior (extreme scores)

## Rollback Steps (< 15 minutes)

1. **Immediate Mitigation** (< 5 min)
   - Edit config: Set `enable_quality_scoring: false`
   - OR set env var: `MAPROOM_ENABLE_QUALITY_SCORING=false`
   - Restart maproom service (or hot reload if implemented)

2. **Verification** (< 5 min)
   - Check logs: Confirm flag is false
   - Run test query: Verify old behavior restored
   - Check metrics: Latency returns to baseline

3. **Communication** (< 5 min)
   - Notify team: Rollback completed
   - Document issue in incident log
   - Preserve metrics/logs for investigation

## Rollback Authority

Who can rollback:
- Any engineer with production config access
- On-call engineer (24/7)
- Engineering manager

No approval needed for rollback (safety mechanism).
```

## Dependencies

**Prerequisites:**
- SRCHREL-3002 (monitoring setup complete)
- SRCHREL-3001 (documentation ready)

**Blocks:**
- None (rollout is final phase)

## Planning References

- Plan: `.crewchief/projects/SRCHREL_relationship-aware-search/planning/plan.md` (Rollout strategy, lines 419-521)

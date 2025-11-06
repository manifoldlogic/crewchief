# Ticket: AGENTOPT-3002: Phase 3 Testing and Deployment (OPTIONAL)

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
**OPTIONAL PHASE 3**: Test and deploy LLM fallback with cost monitoring and gradual rollout.

## Background
Validates Phase 3 LLM fallback (AGENTOPT-3001) through testing and controlled production deployment with strict cost monitoring.

## Acceptance Criteria
- [ ] Unit tests for rewriter (mocked API responses)
- [ ] Integration tests with real Haiku API
- [ ] Fallback success rate >50% (rewrites help find results)
- [ ] Cost monitoring dashboard deployed
- [ ] Alerts configured (>$10/day triggers email)
- [ ] Monthly cost <$150 at 100 users
- [ ] Feature flag deployment (opt-in initially)

## Technical Requirements
- Test cases:
  - 0 results → fallback triggers → rewrites → retry → success
  - Low confidence → fallback → improved results
  - Timeout handling (2s limit)
  - API errors (graceful degradation)
- Cost monitoring:
  - Track fallback_count, cost_per_call, monthly_total
  - Alert if daily cost >$10
  - Dashboard showing cost trends
- Gradual rollout:
  - Week 1: Internal testing only
  - Week 2: 10% of users (opt-in)
  - Week 3: 25% if costs acceptable
  - Week 4+: Evaluate full rollout vs keep opt-in

## Implementation Notes
1. Unit tests with mocked Haiku responses
2. Integration tests with real API (limited to 10 calls)
3. Deploy with flag OFF
4. Enable for internal testing (1 week)
5. Gradual rollout based on cost/success metrics

## Dependencies
- AGENTOPT-3001 (LLM fallback implementation)

## Risk Assessment
- **Risk**: Cost spiral
  - **Mitigation**: Hard rate limits, immediate kill switch
- **Risk**: Fallback not helpful (success rate <50%)
  - **Mitigation**: Roll back, refine prompts, try different model

## Files/Packages Affected
- crates/maproom/tests/ (unit and integration tests)
- Cost monitoring dashboard

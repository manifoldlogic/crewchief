# Ticket: LANG_PARSE-4004: Production Deployment and Rollout

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- monitoring-observability-engineer
- database-engineer
- test-runner
- verify-ticket
- commit-ticket

## Summary
Create and execute a staged production rollout plan for the new tree-sitter parser implementation across Python, Rust, and Go languages. Implement comprehensive monitoring dashboards and alerting to ensure safe deployment with immediate visibility into parser performance and error rates.

## Background
After completing testing and validation in LANG_PARSE-4003, the new parser implementation is ready for production deployment. This ticket covers the final phase of the language parsing enhancement project - safely rolling out the new parsers to production with proper observability and operational procedures.

A staged rollout approach (Python → Rust → Go) minimizes risk by deploying the most stable parser first and monitoring for issues before proceeding. Comprehensive monitoring and alerting ensure the team can quickly detect and respond to any performance degradation or parsing errors.

This is Phase 4, Week 7, Task 4 from the planning document, representing the culmination of the entire language parsing enhancement project.

## Acceptance Criteria
- [ ] Staged rollout plan document created with clear phases and rollback procedures
- [ ] Grafana dashboard operational showing parser metrics (parse time, success rate, memory usage)
- [ ] Alert rules configured for parse errors, performance degradation, and memory issues
- [ ] Operational runbook documented with troubleshooting procedures
- [ ] Python parser rolled out to production with 48-hour monitoring period
- [ ] Rust parser rolled out to production with 48-hour monitoring period
- [ ] Go parser rolled out to production with 48-hour monitoring period
- [ ] Post-rollout validation completed with no critical issues
- [ ] All monitoring dashboards showing healthy metrics across all three languages

## Technical Requirements
- Grafana dashboard configuration for parser metrics:
  - Parse time by language (p50, p95, p99)
  - Parse success/error rates
  - Memory usage per parser
  - Chunks created per file
  - Files processed per minute
- Alert configuration in Prometheus/Alertmanager:
  - Parse error rate > 5% (warning), > 10% (critical)
  - Parse time p95 > 2x baseline (warning), > 3x baseline (critical)
  - Memory usage > 500MB per parser (warning), > 1GB (critical)
  - Parser crash/restart events (critical)
- Rollout plan with:
  - Stage 1: Python (most stable, highest test coverage)
  - Stage 2: Rust (after 48h Python monitoring)
  - Stage 3: Go (after 48h Rust monitoring)
  - Rollback procedures for each stage
  - Success criteria for proceeding to next stage
- Operational runbook covering:
  - How to check parser health
  - Common error patterns and solutions
  - Rollback procedures
  - Escalation paths
  - Performance tuning guidelines

## Implementation Notes

### Monitoring Dashboard Design
Create a comprehensive Grafana dashboard (`crates/maproom/config/grafana/parser_dashboard.json`) with panels for:
1. **Overview Panel**: High-level health indicators for all three parsers
2. **Performance Panels**: Parse time histograms and trends by language
3. **Error Panels**: Error rates, error types, and recent error samples
4. **Resource Panels**: Memory usage, CPU usage, and file processing throughput
5. **Comparison Panel**: Before/after metrics comparing old vs new parser performance

### Alert Configuration
Configure alerts in `crates/maproom/config/alerts/parser_alerts.yml`:
- Use Prometheus alert rules format
- Include alert severity levels (info, warning, critical)
- Set appropriate thresholds based on baseline metrics from LANG_PARSE-4001
- Configure notification channels (Slack, PagerDuty, email)
- Include runbook links in alert annotations

### Staged Rollout Strategy
Document in `crates/maproom/docs/rollout_plan.md`:
- **Stage 1 - Python (Day 1)**:
  - Enable Python parser in production
  - Monitor for 48 hours
  - Success criteria: error rate < 2%, p95 parse time < baseline + 20%
- **Stage 2 - Rust (Day 3)**:
  - Enable Rust parser after Python validation
  - Monitor for 48 hours
  - Success criteria: error rate < 2%, p95 parse time < baseline + 20%
- **Stage 3 - Go (Day 5)**:
  - Enable Go parser after Rust validation
  - Monitor for 48 hours
  - Success criteria: error rate < 2%, p95 parse time < baseline + 20%
- **Rollback Procedure**: Feature flag toggle to revert to old parser, test in staging first

### Operational Runbook
Create comprehensive runbook in `crates/maproom/docs/operational_runbook.md`:
1. **Health Checks**: How to verify parser is functioning correctly
2. **Common Issues**: Known error patterns and their solutions
3. **Performance Tuning**: Memory limits, concurrent workers, batch sizes
4. **Rollback Process**: Step-by-step rollback instructions
5. **Escalation**: When and how to escalate issues
6. **Troubleshooting Flowcharts**: Decision trees for common problems

### Configuration Management
- Use feature flags to control parser rollout per language
- Store rollout state in database for coordination
- Implement gradual rollout percentage (0% → 25% → 50% → 100% for each stage)
- Ensure rollback can be triggered via config update without code deployment

## Dependencies
- **LANG_PARSE-4003**: Migration and cutover testing completed - REQUIRED
- External: Grafana instance available and configured
- External: Prometheus/Alertmanager configured for Maproom
- External: Production deployment pipeline ready

## Risk Assessment
- **Risk**: Parser regression causes production indexing failures
  - **Mitigation**: Staged rollout allows catching issues early; feature flags enable instant rollback; comprehensive monitoring provides early warning

- **Risk**: Alert fatigue from overly sensitive thresholds
  - **Mitigation**: Baseline metrics from LANG_PARSE-4001 inform appropriate thresholds; alerts have clear severity levels; test alerts in staging first

- **Risk**: Monitoring overhead impacts parser performance
  - **Mitigation**: Use sampling for high-frequency metrics; optimize metric collection; benchmark monitoring impact before rollout

- **Risk**: Rollback procedure fails or is unclear
  - **Mitigation**: Test rollback in staging environment; document step-by-step procedures; include rollback drills in runbook

- **Risk**: Insufficient monitoring visibility during rollout
  - **Mitigation**: Deploy monitoring and alerts before parser rollout; validate dashboards show expected data; set up real-time monitoring rotation

## Files/Packages Affected
- `crates/maproom/config/grafana/parser_dashboard.json` (NEW)
- `crates/maproom/config/alerts/parser_alerts.yml` (NEW)
- `crates/maproom/docs/rollout_plan.md` (NEW)
- `crates/maproom/docs/operational_runbook.md` (NEW)
- `crates/maproom/src/config.rs` (feature flag configuration)
- `crates/maproom/src/metrics.rs` (metrics export for monitoring)
- `crates/maproom/src/parser/mod.rs` (feature flag integration)

# Quality-Weighted Graph Scoring Rollout Plan

**Ticket:** SRCHREL-3003
**Feature:** Relationship-Aware Search Ranking (SRCHREL)

---

## Executive Summary

This document outlines a 4-stage rollout plan for quality-weighted graph scoring with checkpoints, success criteria, and rollback procedures.

---

## Rollout Stages

### Stage 1: Development (COMPLETE)

**Duration:** Completed
**Scope:** Local development and CI

**Checkpoints:**
- [x] All unit tests passing
- [x] Integration tests passing
- [x] Benchmark performance <35ms p95
- [x] Configuration documentation complete
- [x] Feature flag defaults to disabled

**Status:** ✅ Complete - Ready for Stage 2

---

### Stage 2: Internal Testing

**Duration:** 1 week
**Scope:** Enable for CrewChief codebase development

**Actions:**
1. Enable feature flag: `MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_QUALITY_WEIGHTED_GRAPH=true`
2. Monitor search latency and error rates
3. Collect feedback from internal users
4. Review search relevance for common queries

**Success Criteria:**
- [ ] No latency regressions >10%
- [ ] No new errors or crashes
- [ ] Subjective improvement in search relevance
- [ ] No reported degradations in common queries

**Exit Criteria:**
- 1 week with all success criteria met
- OR explicit approval from tech lead

**Rollback Trigger:**
- Latency regression >20%
- Error rate increase >1%
- Multiple user complaints about relevance

**Rollback Procedure:**
```bash
export MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_QUALITY_WEIGHTED_GRAPH=false
# Restart any running daemons
```

---

### Stage 3: Limited Production

**Duration:** 2 weeks
**Scope:** Enable for select external users/repositories

**Actions:**
1. Enable via config file for selected repositories
2. Monitor per-repository metrics
3. A/B test with fusion_weight_override variations
4. Collect structured user feedback

**Success Criteria:**
- [ ] p95 latency <35ms maintained across repos
- [ ] No critical bugs reported
- [ ] Positive user feedback trend
- [ ] Ranking improvements validated (per SRCHREL-2005)

**Exit Criteria:**
- 2 weeks stable operation
- ≥64% of test queries show improvement
- ≤4% of test queries show degradation

**Rollback Trigger:**
- p95 latency exceeds 40ms
- Any data corruption or crash
- >10% of users report degradation

**Rollback Procedure:**
```bash
# Option 1: Environment variable (immediate)
export MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_QUALITY_WEIGHTED_GRAPH=false

# Option 2: Config file
# Edit maproom-search.yml:
# feature_flags:
#   enable_quality_weighted_graph: false
```

---

### Stage 4: Full Production

**Duration:** Permanent (with monitoring)
**Scope:** Default enabled for all users

**Actions:**
1. Update default configuration to `enable_quality_weighted_graph: true`
2. Document in release notes
3. Maintain monitoring dashboards
4. Gradual enablement over 1 week

**Success Criteria:**
- [ ] 1 month stable operation
- [ ] Search quality metrics improved
- [ ] No rollback required
- [ ] Documentation and runbooks complete

**Maintenance:**
- Monthly review of metrics
- Quarterly tuning of edge weights if needed
- Continuous monitoring for regressions

---

## Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Latency regression | Low | Medium | Feature flag rollback in <1 min |
| Score distribution shift | Medium | Low | Expected behavior, monitor for issues |
| Test code ranking higher | Low | Medium | Verify test detection patterns |
| Configuration errors | Low | Low | Validation prevents invalid config |
| Database performance | Low | High | Index optimization complete |

---

## Rollback Procedures

### Immediate Rollback (Seconds)

Environment variable override (no restart needed for new searches):

```bash
export MAPROOM_SEARCH_FEATURE_FLAGS_ENABLE_QUALITY_WEIGHTED_GRAPH=false
```

### Configuration Rollback (Minutes)

Edit config file and restart daemon:

```yaml
# maproom-search.yml
feature_flags:
  enable_quality_weighted_graph: false
```

### Emergency Rollback (If Both Fail)

1. Deploy previous version without quality-weighted code
2. Contact on-call engineer
3. File incident report

---

## Communication Plan

### Stage 2 (Internal)
- Announce in #maproom-dev channel
- Share monitoring dashboard link
- Request feedback via issue tracker

### Stage 3 (Limited)
- Email selected users with opt-in instructions
- Create feedback form
- Weekly status updates

### Stage 4 (Full)
- Release notes with feature description
- Documentation links in changelog
- Blog post for major improvement

---

## Success Metrics

| Metric | Baseline | Target | Measurement |
|--------|----------|--------|-------------|
| Search relevance | Legacy ranking | ≥64% improved | Manual evaluation |
| p95 latency | Current p95 | <35ms | Prometheus metrics |
| Error rate | ~0% | <1% | Error monitoring |
| User satisfaction | Current | Improved | Feedback surveys |

---

## Contacts

| Role | Responsibility |
|------|----------------|
| Feature Owner | Rollout decisions, go/no-go |
| On-Call Engineer | Monitoring, immediate rollback |
| Tech Lead | Stage transitions, escalation |

---

**Document Version:** 1.0
**Author:** docs-engineer agent
**Last Updated:** 2025-12-15

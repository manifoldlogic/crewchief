# Semantic Entry Point Ranking - Deployment Runbook

**Version**: 1.0
**Last Updated**: 2025-11-19
**Project**: SEMRANK (Semantic Entry Point Ranking)
**Risk Level**: Low (SQL logic changes only, no schema changes)

## Overview

This runbook guides the deployment of semantic entry point ranking enhancements to the Maproom search system. The changes implement kind-based multipliers and exact match bonuses to prioritize code implementations over tests and documentation in search results.

**What's Changing:**
- TypeScript: `packages/maproom-mcp/src/tools/search.ts` (query normalization)
- Rust: `crates/maproom/src/search/fts.rs` (SQL scoring logic)
- No database schema changes
- No data migration required
- Fully reversible

## Pre-Deployment Checklist

Before deploying, verify all conditions are met:

### Code Quality
- [ ] All Phase 0-3 tickets completed (SEMRANK-0001 through SEMRANK-3006)
- [ ] All integration tests passing (`pnpm test` in maproom-mcp package)
- [ ] All regression tests passing (SEMRANK-3006 suite: 11/11 tests)
- [ ] Code reviewed and approved
- [ ] No outstanding security issues

### Performance Validation
- [ ] Performance benchmarks completed (SEMRANK-3005)
- [ ] p95 latency within acceptable range (<200ms)
- [ ] Benchmark comparison shows no >10% regression on any query
- [ ] Performance improvement validated (target: -17% average latency)

### Documentation
- [ ] Search ranking documentation complete (`docs/search-ranking.md`)
- [ ] Architecture documentation updated (`docs/architecture/MAPROOM_ARCHITECTURE.md`)
- [ ] README updated with semantic ranking feature
- [ ] This runbook reviewed and approved

### Environment Readiness
- [ ] Staging environment available for pre-production testing
- [ ] Monitoring dashboards configured
- [ ] Rollback plan tested in staging
- [ ] Team notified of deployment window

## Deployment Steps

### Step 1: Pre-Deployment Validation

**Duration**: 15 minutes

Run final validation checks before deployment:

```bash
# Navigate to workspace root
cd /workspace

# Run all tests
pnpm test

# Run specific SEMRANK integration tests
cd packages/maproom-mcp
pnpm exec vitest run tests/integration/regression.test.ts
pnpm exec vitest run tests/integration/search-quality.test.ts
pnpm exec vitest run tests/integration/edge-cases.test.ts

# Verify build succeeds
pnpm build

# Check Rust binary compiles
cd /workspace/crates/maproom
cargo build --release

# Verify binary runs
/workspace/target/release/crewchief-maproom --version
```

**Success Criteria:**
- All tests pass (100% pass rate)
- Build completes without errors
- Binary executable and reports version

### Step 2: Record Baseline Metrics

**Duration**: 10 minutes

Before deploying, capture current baseline metrics for comparison:

```bash
# Run baseline query set
cd /workspace/packages/maproom-mcp

# Execute golden queries and record results
# (Requires database running with test-corpus or crewchief indexed)

# Example queries to record:
# - "authenticate" → record top 3 kinds, latency, scores
# - "validate_token" → record top 3 kinds, latency, scores
# - "user authentication" → record top 3 kinds, latency, scores
# - "sendMessage" → record top 3 kinds, latency, scores
# - "HTTPServer" → record top 3 kinds, latency, scores

# Manual baseline capture (adapt to your monitoring tools):
# 1. Run each query via MCP tool or direct search function
# 2. Record: top_3_kinds, p95_latency, top_1_score
# 3. Store in baseline-metrics.csv for comparison
```

**Baseline Metrics to Record:**
- Query latency (p50, p95, p99)
- Top-1 result kind distribution
- Top-1 implementation accuracy for exact matches
- Error rate

### Step 3: Deploy Code Changes

**Duration**: 10 minutes

Deploy updated TypeScript and Rust code:

```bash
# Ensure you're on main branch with latest SEMRANK changes
cd /workspace
git status
git log --oneline -5  # Verify SEMRANK commits present

# Build TypeScript
cd /workspace/packages/maproom-mcp
pnpm build

# Build Rust binary
cd /workspace/crates/maproom
cargo build --release

# Verify binary in correct location
ls -lh /workspace/target/release/crewchief-maproom

# If using Docker/MCP server, rebuild container
cd /workspace/packages/maproom-mcp
docker compose down
docker compose build
docker compose up -d

# Verify MCP server started successfully
docker compose ps
docker compose logs -f --tail=50 maproom-mcp
```

**Success Criteria:**
- Build completes without errors
- Binary created in `/workspace/target/release/`
- MCP server running (if using Docker)
- No error logs in server startup

### Step 4: Warm Up Caches

**Duration**: 5 minutes

Run test queries to warm up database caches and verify system operational:

```bash
# Execute golden query set to warm caches
# Use MCP tool or direct search function

# Example warm-up queries:
# 1. authenticate
# 2. validate_token
# 3. user authentication
# 4. sendMessage
# 5. HTTPServer
# 6. parse json
# 7. database connection
# 8. XMLParser
# 9. useAuth
# 10. create session
```

**Success Criteria:**
- All queries return results
- No errors or timeouts
- Response times reasonable (<200ms p95)

### Step 5: Post-Deployment Validation

**Duration**: 20 minutes

Run comprehensive validation suite:

#### 5.1: Smoke Tests (Golden Queries)

Execute 10 representative queries and verify improved ranking:

| Query | Expected Top-1 Kind | Expected Behavior |
|-------|---------------------|-------------------|
| `authenticate` | `func` or `async_func` | Implementation ranks #1, not docs |
| `validate_token` | `func` | Exact match with snake_case normalization |
| `user authentication` | `func` or `class` | Implementation beats documentation |
| `sendMessage` | `func` or `method` | camelCase query works |
| `HTTPServer` | `class` or `func` | Acronym handling works |
| `parse json` | `func` | Multi-word query works |
| `database connection` | `func` or `class` | snake_case match via normalization |
| `XMLParser` | `class` or `func` | Acronym normalization works |
| `useAuth` | `func` or `hook` | Hook type prioritized |
| `create_session` | `func` | Exact match works |

**Test Command:**
```bash
# Run smoke test suite (if automated)
cd /workspace/packages/maproom-mcp
pnpm exec vitest run tests/integration/regression.test.ts

# Manual validation:
# For each query, verify:
# 1. Top-1 result is implementation (func, class, method), not test or doc
# 2. Latency <200ms
# 3. Score >1.0 (indicates multipliers applied)
```

#### 5.2: Latency Validation

Verify performance meets SLOs:

```bash
# Run performance benchmark suite
pnpm exec vitest run tests/integration/performance.test.ts

# Verify:
# - p95 latency <200ms
# - No query >10% slower than baseline
# - Target: 17% improvement on average (from benchmarks)
```

**Success Criteria:**
- p50 latency <100ms
- p95 latency <200ms
- p99 latency <500ms
- No query regressed >10% vs baseline

#### 5.3: Debug Mode Validation

Verify debug mode returns score breakdown:

```bash
# Test debug mode (manual or automated)
# Query with debug=true should return score_breakdown object

# Expected structure:
# {
#   chunk_id: number,
#   symbol_name: string,
#   kind: string,
#   score: number,
#   score_breakdown: {
#     base_score: number,
#     kind_multiplier: number,
#     exact_match_multiplier: number,
#     final_score: number,
#     explanation: string
#   }
# }
```

#### 5.4: Integration Test Suite

Run complete integration test suite:

```bash
cd /workspace/packages/maproom-mcp

# Run all SEMRANK integration tests
pnpm exec vitest run tests/integration/regression.test.ts
pnpm exec vitest run tests/integration/search-quality.test.ts
pnpm exec vitest run tests/integration/edge-cases.test.ts

# Verify 100% pass rate
```

**Success Criteria:**
- All tests pass (11/11 regression, all quality, all edge cases)
- No new failures introduced
- Test execution time reasonable

### Step 6: Deployment Sign-Off

**Duration**: 5 minutes

Document deployment completion:

- [ ] All smoke tests passed
- [ ] Latency within SLOs
- [ ] Debug mode functional
- [ ] Integration tests passing
- [ ] No errors in logs
- [ ] Monitoring dashboards show healthy metrics

**Record Deployment:**
- Deployment time: `<timestamp>`
- Git commit SHA: `<sha>`
- Deployed by: `<name>`
- Environment: `<production|staging>`

## Post-Deployment Monitoring

### Monitoring Plan (4-Week Window)

Track the following metrics for 4 weeks post-deployment:

#### Week 1: Close Monitoring (Daily Checks)

**Performance Metrics:**
- Query latency (p50, p95, p99)
- Query volume
- Error rate
- Timeout rate

**Quality Metrics:**
- Top-1 result kind distribution
  - Target: >70% func/class/method for exact matches
  - Baseline: <50% (before SEMRANK)
- Average rank of first implementation
  - Target: <3 (top 3 results)
- Top-1 accuracy for exact symbol matches
  - Target: >90%

**User Experience:**
- User feedback (if available)
- Support tickets related to search
- Complaint patterns

#### Weeks 2-4: Regular Monitoring (Weekly Checks)

Continue monitoring same metrics, reviewing weekly:
- Identify any degradation trends
- Look for query patterns that don't benefit from semantic ranking
- Collect data for potential tuning

### Monitoring Tools

**Manual Monitoring:**
```bash
# Run golden query set weekly
# Compare results to baseline
# Record: latency, top_1_kind, top_3_kinds

# Example monitoring script:
cd /workspace/packages/maproom-mcp
# Execute queries and log results
# Parse logs for metrics
```

**Automated Monitoring (if available):**
- Grafana dashboards for latency tracking
- PostgreSQL slow query log analysis
- MCP server metrics (if instrumented)

### Tuning Triggers

Adjust multipliers if any of these conditions occur:

| Metric | Threshold | Action |
|--------|-----------|--------|
| Top-1 implementation rate | <70% | Increase func/class multipliers by +0.2 |
| Avg implementation rank | >5 | Investigate query patterns, increase multipliers |
| p95 latency | >220ms (10% regression) | Profile queries, optimize CASE logic |
| Error rate | >1% | Investigate errors, check edge cases |
| User complaints | >5 reports/week | Review specific queries, adjust multipliers |

**Tuning Process:**
1. Identify problematic query patterns
2. Propose multiplier adjustment (±0.2 increments)
3. Test adjustment in staging with benchmarks
4. Deploy adjustment if improvement confirmed
5. Document tuning rationale in `docs/architecture/MAPROOM_ARCHITECTURE.md`

## Rollback Procedure

**When to Rollback:**
- p95 latency >10% above baseline for >24 hours
- Top-1 implementation rate <50% (worse than baseline)
- Critical bugs affecting search functionality
- Error rate >5%

### Rollback Steps

**Duration**: 15 minutes

#### Step 1: Identify Rollback Target

```bash
# Identify commit SHA before SEMRANK changes
cd /workspace
git log --oneline --all | grep -i "semrank" | tail -1

# Find commit BEFORE first SEMRANK commit
# Record SHA: <pre-semrank-sha>
```

#### Step 2: Revert Code Changes

```bash
# Checkout previous versions of modified files
cd /workspace

# Revert TypeScript search tool
git checkout <pre-semrank-sha> packages/maproom-mcp/src/tools/search.ts

# Revert Rust FTS implementation
git checkout <pre-semrank-sha> crates/maproom/src/search/fts.rs

# Verify changes reverted
git status
git diff HEAD
```

#### Step 3: Rebuild and Restart

```bash
# Rebuild TypeScript
cd /workspace/packages/maproom-mcp
pnpm build

# Rebuild Rust binary
cd /workspace/crates/maproom
cargo build --release

# Restart MCP server (if using Docker)
cd /workspace/packages/maproom-mcp
docker compose down
docker compose up -d

# Verify server started
docker compose ps
docker compose logs -f --tail=50 maproom-mcp
```

#### Step 4: Validate Rollback

```bash
# Run integration tests to verify rollback successful
cd /workspace/packages/maproom-mcp
pnpm test

# Run golden queries
# Verify results match pre-SEMRANK baseline

# Check latency returned to baseline
# Verify no errors in logs
```

#### Step 5: Document Rollback

Record rollback event:
- Rollback time: `<timestamp>`
- Rolled back to SHA: `<pre-semrank-sha>`
- Reason for rollback: `<description>`
- Performed by: `<name>`
- Next steps: `<investigation plan>`

### Post-Rollback Investigation

1. **Collect Data:**
   - Logs from deployment period
   - Latency metrics showing regression
   - Query examples demonstrating issues
   - User feedback/complaints

2. **Root Cause Analysis:**
   - Identify specific queries causing problems
   - Analyze why multipliers performed poorly
   - Check for edge cases not covered in testing

3. **Fix and Re-Deploy:**
   - Adjust multipliers based on findings
   - Add test cases for problematic queries
   - Re-run benchmarks in staging
   - Schedule new deployment when ready

## Tuning Criteria and Process

### Multiplier Values (Current)

From `docs/search-ranking.md`:

| Kind | Multiplier | Rationale |
|------|-----------|-----------|
| `func`, `async_func` | 2.5× | Primary implementations |
| `class`, `struct`, `enum`, `interface` | 2.0× | Type definitions |
| `method` | 1.5× | Class methods |
| `test`, `test_function` | 0.6× | Demote tests |
| `heading_1`, `heading_2` | 0.6×, 0.5× | Demote docs |
| `heading_3` | 0.3× | Lower-level headings |
| `comment`, `doc_comment` | 0.3× | Lowest priority |
| Exact match bonus | 3.0× | Symbol name exact match |

### When to Tune

Monitor for 2-4 weeks, then consider tuning if:
- Top-1 implementation rate <70%
- Average implementation rank >5
- Specific languages underperforming
- User feedback indicates poor ranking

### Tuning Process

1. **Identify Issue:**
   - Collect problematic query examples
   - Analyze current ranking vs desired ranking
   - Identify specific kinds or patterns affected

2. **Propose Adjustment:**
   - Adjust multipliers in ±0.2 increments
   - Example: func 2.5× → 2.7× if implementations not ranking high enough
   - Document rationale

3. **Test in Staging:**
   - Update multipliers in staging environment
   - Re-run benchmark suite
   - Verify improvement on problematic queries
   - Ensure no regression on other queries

4. **A/B Test (if possible):**
   - Run old and new multipliers in parallel
   - Compare metrics over 1 week
   - Use statistical significance testing

5. **Deploy Tuning:**
   - Follow this runbook's deployment process
   - Monitor closely for 1 week
   - Document adjustment in architecture.md

6. **Document Results:**
   - Update `docs/architecture/MAPROOM_ARCHITECTURE.md` with new multiplier values
   - Add tuning rationale and date
   - Update `docs/search-ranking.md` if user-facing changes

### Example Tuning Scenarios

**Scenario 1: Tests Still Rank Too High**

*Symptom:* `test_authenticate()` ranks #1 instead of `authenticate()`

*Analysis:* Test multiplier 0.6× not low enough

*Solution:*
- Reduce test multiplier: 0.6× → 0.4×
- Test in staging
- Verify implementations now rank first
- Deploy if successful

**Scenario 2: Implementations Not Standing Out**

*Symptom:* Average implementation rank is 4-5, target is <3

*Analysis:* Implementation multipliers need boost

*Solution:*
- Increase func multiplier: 2.5× → 2.7×
- Increase class multiplier: 2.0× → 2.2×
- Test in staging
- Verify average rank improves to <3
- Deploy if successful

**Scenario 3: Documentation Still Too High**

*Symptom:* Documentation ranks above code for concept queries

*Analysis:* Doc multipliers not low enough

*Solution:*
- Reduce heading_1: 0.6× → 0.4×
- Reduce heading_2: 0.5× → 0.3×
- Test in staging
- Verify code ranks first
- Deploy if successful

## Golden Query Set

Use these 10 queries for smoke testing and monitoring:

| # | Query | Expected Top-1 Kind | Test Category |
|---|-------|---------------------|---------------|
| 1 | `authenticate` | `func` | Exact match |
| 2 | `validate_token` | `func` | snake_case normalization |
| 3 | `user authentication` | `func` or `class` | Concept query |
| 4 | `sendMessage` | `func` or `method` | camelCase |
| 5 | `HTTPServer` | `class` or `func` | Acronym |
| 6 | `parse json` | `func` | Multi-word |
| 7 | `database connection` | `func` or `class` | snake_case via normalization |
| 8 | `XMLParser` | `class` or `func` | Acronym normalization |
| 9 | `useAuth` | `func` or `hook` | Hook type |
| 10 | `create_session` | `func` | snake_case exact match |

**How to Use:**
1. Run all 10 queries before deployment (baseline)
2. Run all 10 queries after deployment (validation)
3. Run weekly during monitoring period
4. Compare results to baseline
5. Track any ranking changes

## Risk Assessment and Mitigation

| Risk | Likelihood | Impact | Mitigation | Contingency |
|------|-----------|--------|------------|-------------|
| Latency regression >10% | Low | High | Benchmarks must pass before deploy | Rollback immediately |
| Search quality degrades | Low | Medium | Golden queries cover diverse patterns | Rollback, adjust multipliers |
| Rollback procedure fails | Very Low | High | Test rollback in staging first | Manual file revert, rebuild |
| Monitoring not working | Medium | Medium | Set up dashboards before deploy | Manual query monitoring |
| Unknown edge cases | Medium | Low | Comprehensive test suite | Quick patch release |
| User complaints spike | Low | Medium | Monitor support channels closely | Investigate specific queries, tune |

## Success Criteria

Deployment considered successful if after 1 week:

**Performance:**
- [x] p95 latency <200ms
- [x] No query >10% slower than baseline
- [x] Average latency improved (target: -17% from benchmarks)

**Quality:**
- [x] Top-1 implementation rate >70% for exact matches
- [x] Average implementation rank <3
- [x] Top-1 accuracy >90% for exact symbol matches

**Stability:**
- [x] Error rate <1%
- [x] No critical bugs reported
- [x] Integration tests 100% passing

**User Experience:**
- [x] Positive feedback or neutral (no negative spike)
- [x] Support ticket volume unchanged or decreased
- [x] No complaints about search quality

## Appendix A: File Locations

**Modified Files:**
- `/workspace/packages/maproom-mcp/src/tools/search.ts`
- `/workspace/crates/maproom/src/search/fts.rs`
- `/workspace/crates/maproom/src/search/normalize.rs`

**Documentation:**
- `/workspace/packages/maproom-mcp/docs/search-ranking.md`
- `/workspace/docs/architecture/MAPROOM_ARCHITECTURE.md`
- `/workspace/packages/maproom-mcp/README.md`

**Tests:**
- `/workspace/packages/maproom-mcp/tests/integration/regression.test.ts`
- `/workspace/packages/maproom-mcp/tests/integration/search-quality.test.ts`
- `/workspace/packages/maproom-mcp/tests/integration/edge-cases.test.ts`
- `/workspace/packages/maproom-mcp/tests/integration/performance.test.ts`

**Results:**
- `/workspace/packages/maproom-mcp/tests/results/regression-validation.md`
- `/workspace/packages/maproom-mcp/benchmarks/performance-comparison.md`

## Appendix B: Contact Information

**Deployment Team:**
- Primary contact: `<name>`
- Backup contact: `<name>`
- Escalation: `<name>`

**Incident Response:**
- Rollback decision maker: `<name>`
- On-call engineer: `<name>`

## Appendix C: References

- **SEMRANK Project Plan:** `.crewchief/projects/SEMRANK_semantic-entry-point-ranking/planning/plan.md`
- **Architecture Documentation:** `docs/architecture/MAPROOM_ARCHITECTURE.md`
- **Search Ranking Guide:** `packages/maproom-mcp/docs/search-ranking.md`
- **Quality Strategy:** `.crewchief/projects/SEMRANK_semantic-entry-point-ranking/planning/quality-strategy.md`

---

**Document Version History:**
- v1.0 (2025-11-19): Initial runbook creation
- Document owner: SEMRANK Project Team
- Review schedule: Quarterly or after each deployment

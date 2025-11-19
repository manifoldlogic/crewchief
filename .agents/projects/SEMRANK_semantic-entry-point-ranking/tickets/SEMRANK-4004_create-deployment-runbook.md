# Ticket: SEMRANK-4004: Create Deployment Runbook

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (documentation-only ticket, but runbook procedures should be tested)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Create deployment runbook documenting step-by-step deployment process, rollback procedures, monitoring plan, and post-deployment validation for the semantic ranking system.

## Background
The SEMRANK project implements semantic entry point ranking with no schema changes—only SQL logic modifications. While this is low-risk, we need a comprehensive deployment runbook to ensure safe rollout, quick rollback if needed, and ongoing monitoring to validate search quality improvements.

This ticket implements the deployment preparation requirements from Phase 4 (Documentation & Deployment) of the SEMRANK project plan.

## Acceptance Criteria
- [ ] Create deployment runbook with sections:
  - [ ] Pre-Deployment Checklist:
    - [ ] All Phase 3 tests passing
    - [ ] Performance benchmarks acceptable
    - [ ] Documentation complete
    - [ ] Rollback plan tested
  - [ ] Deployment Steps:
    - [ ] Deploy updated search.ts to MCP server
    - [ ] Deploy updated fts.rs (Rust binary)
    - [ ] Restart MCP server
    - [ ] Warm up caches (run test queries)
  - [ ] Validation Steps:
    - [ ] Run smoke tests (10 golden queries)
    - [ ] Check p95 latency < 200ms
    - [ ] Verify top-1 accuracy >90% for exact matches
    - [ ] Test debug mode returns score_breakdown
  - [ ] Monitoring Plan:
    - [ ] Track search latency (p50, p95, p99)
    - [ ] Track top-1 result kind distribution
    - [ ] Monitor error rates
    - [ ] Collect user feedback (if available)
    - [ ] 4-week monitoring window
  - [ ] Rollback Procedure:
    - [ ] Revert to previous search.ts version
    - [ ] Revert to previous fts.rs version
    - [ ] Restart MCP server
    - [ ] Validate rollback (run tests)
  - [ ] Tuning Criteria (from architecture.md):
    - [ ] Adjust if top-1 implementation rate <70%
    - [ ] Adjust if average implementation rank >5
    - [ ] Adjustment process: ±0.2 increments, A/B test
- [ ] Runbook tested in staging/dev environment
- [ ] Rollback procedure verified (can revert cleanly)

## Technical Requirements
- Create `/packages/maproom-mcp/docs/deployment/semantic-ranking-rollout.md`
- Include step-by-step procedures for deployment and rollback
- Define clear validation criteria and success metrics
- Document monitoring strategy with specific metrics to track
- Include tuning criteria from architecture.md
- Make runbook executable (can be followed without ambiguity)

## Implementation Notes

### Deployment Characteristics
- **No schema changes**: Only SQL logic updates
- **Low risk**: No data migration, reversible changes
- **Components affected**:
  - TypeScript: `packages/maproom-mcp/src/tools/search.ts`
  - Rust: `crates/maproom/src/search/fts.rs`
  - No database schema changes

### Golden Queries for Smoke Testing
Define 10 representative queries that cover:
1. Exact match function names
2. Partial matches
3. camelCase queries
4. Acronyms (HTTP, SQL, etc.)
5. Multi-word queries
6. Edge cases (single character, special chars)

Examples:
- `sendMessage` (exact match, camelCase)
- `authenticate` (partial match)
- `HTTPServer` (acronym)
- `parse json` (multi-word)

### Validation Metrics
- **Latency**: p95 < 200ms (from architecture.md performance requirements)
- **Accuracy**: Top-1 accuracy >90% for exact matches
- **Quality**: Top-1 implementation rate >70%
- **Debug mode**: score_breakdown present when debug=true

### Monitoring Plan
4-week window tracking:
1. **Performance**:
   - p50, p95, p99 latency
   - Query volume
   - Error rate
2. **Quality**:
   - Top-1 result kind distribution (should favor functions/classes over tests)
   - Average rank of first implementation
   - User interactions (if click-through data available)
3. **Tuning triggers**:
   - If implementation rate <70%: increase function/class multipliers
   - If avg implementation rank >5: investigate query patterns

### Rollback Procedure
1. Identify commit SHA before SEMRANK changes
2. Revert TypeScript files: `git checkout <SHA> packages/maproom-mcp/src/tools/search.ts`
3. Revert Rust files: `git checkout <SHA> crates/maproom/src/search/fts.rs`
4. Rebuild: `pnpm build`
5. Restart MCP server
6. Validate: Run integration tests from SEMRANK-3003
7. Monitor: Check latency returns to baseline

### Tuning Process
From architecture.md:
- Adjust multipliers in ±0.2 increments
- A/B test changes with production traffic (if possible)
- Re-run benchmarks after each adjustment
- Document tuning rationale in architecture.md

## Dependencies
- SEMRANK-3003 (integration tests for validation)
- SEMRANK-3005 (performance benchmarks for validation)
- SEMRANK-4003 (documentation for reference during deployment)

## Risk Assessment
- **Risk**: Deployment causes latency regression
  - **Mitigation**: Performance benchmarks must pass before deployment; rollback immediately if p95 >10% increase
- **Risk**: Search quality degrades for some query patterns
  - **Mitigation**: Golden queries cover diverse patterns; monitor top-1 distribution; rollback if degradation detected
- **Risk**: Rollback procedure untested
  - **Mitigation**: Test rollback in dev/staging before production deployment
- **Risk**: Monitoring not in place before deployment
  - **Mitigation**: Set up monitoring dashboards before deployment; baseline metrics from Phase 1

## Files/Packages Affected
- `/packages/maproom-mcp/docs/deployment/semantic-ranking-rollout.md` (new file)

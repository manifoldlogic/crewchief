# Ticket: AGENTOPT-0005: Deploy Production A/B Testing Infrastructure

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
Build production A/B testing infrastructure for live traffic variant comparison, including consistent user assignment, metrics collection, and analysis dashboard.

## Background
This ticket implements Phase 0, Step 5 from the AGENTOPT project plan (planning/plan.md lines 453-462). After offline testing identifies promising variants, production A/B testing validates performance with real user queries. This infrastructure enables continuous optimization through multi-armed bandit algorithms.

## Acceptance Criteria
- [ ] Variant assignment using consistent hashing (user_id → variant)
- [ ] Metrics collection in MCP server (query logs with variant_id)
- [ ] SQL analysis queries for success rate comparison
- [ ] Simple dashboard or JSON endpoint for monitoring
- [ ] Documentation for deployment process

## Technical Requirements
- Variant assigner (architecture.md lines 1063-1075):
  - Hash user_id to deterministic bucket (0-99)
  - Support 50/50 split initially, configurable ratios
  - Persistent assignment (same user → same variant)
- Metrics collector (architecture.md lines 1077-1095):
  - Log: timestamp, user_id, variant, query, result_count, success
  - Store in PostgreSQL or JSON logs
  - Session tracking for multi-query analysis
- Analysis layer:
  - SQL queries for success rate by variant
  - t-test implementation for statistical comparison
  - Automatic winner detection after n≥1000 per variant
- Optional: Multi-armed bandit (Thompson Sampling) for continuous optimization

## Implementation Notes
Modify `packages/maproom-mcp/src/index.ts`:
1. Add variant assignment logic before returning tool description
2. Add metrics logging after search execution
3. Store variant_id in search context

Create `packages/maproom-mcp/test/tool-description-optimization/ab-test/`:
- assigner.ts (variant assignment)
- collector.ts (metrics collection)
- analyzer.ts (statistical analysis)
- dashboard.ts (simple monitoring)

Database schema:
```sql
CREATE TABLE ab_test_metrics (
  timestamp BIGINT,
  user_id TEXT,
  variant TEXT,
  query_original TEXT,
  result_count INT,
  success BOOLEAN,
  session_id TEXT
);
```

## Dependencies
- AGENTOPT-0002 (variants to A/B test)
- AGENTOPT-0004 (statistical analysis tools)

## Risk Assessment
- **Risk**: User experience degraded by poor variant
  - **Mitigation**: Start with 10% traffic, monitor closely, instant rollback
- **Risk**: Variant assignment inconsistent
  - **Mitigation**: Use cryptographic hash, test assignment stability

## Files/Packages Affected
- packages/maproom-mcp/src/index.ts (variant assignment integration)
- packages/maproom-mcp/test/tool-description-optimization/ab-test/ (infrastructure)
- config/init.sql (add ab_test_metrics table)

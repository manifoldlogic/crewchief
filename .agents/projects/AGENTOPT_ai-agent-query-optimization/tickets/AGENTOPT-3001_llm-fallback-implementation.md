# Ticket: AGENTOPT-3001: Implement LLM Fallback for Edge Cases (Phase 3 - OPTIONAL)

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- rust-indexer-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
**OPTIONAL PHASE 3**: Implement Haiku-based query rewriting as fallback for edge cases when standard approaches fail.

## Background
This ticket implements Phase 3 LLM fallback from planning/plan.md lines 571-591. Uses Claude Haiku to rewrite complex/failed queries into better search terms. Only triggers on: 0 results AND query >5 words, or low confidence (<0.3 top score).

**Trigger Condition**: Phase 1+2 deployed but edge cases remain (complex multi-word queries still failing).

**Cost**: ~$0.0003 per rewrite, estimated $1-2/month for 100 users.

## Acceptance Criteria
- [ ] Haiku integration implemented in Rust
- [ ] Fallback trigger logic (0 results OR low confidence)
- [ ] Query rewriting generates 1-3 alternatives
- [ ] Automatic retry with alternatives
- [ ] Cost monitoring and alerts
- [ ] Feature flag (MAPROOM_ENABLE_LLM_FALLBACK=false by default)

## Technical Requirements
- Location: `crates/maproom/src/query/rewriter.rs`
- Dependencies: `reqwest`, `serde_json` (HTTP client)
- Environment: `ANTHROPIC_API_KEY` required
- Trigger conditions:
  - Result count = 0 AND query length >5 words
  - OR top score <0.3
- Haiku prompt: "Transform this search query into 1-3 optimal code search terms."
- Timeout: 2 seconds (fall back to original query if timeout)
- Cost tracking: Log each API call with cost

## Implementation Notes
See architecture.md lines 305-341 for implementation example.

## Dependencies
- AGENTOPT-2003 (Phase 2 deployed)
- Anthropic API key
- Cost budget approval

## Risk Assessment
- **Risk**: Cost overrun
  - **Mitigation**: Rate limit (max 10 fallbacks/day/user), cost alerts
- **Risk**: Latency degradation
  - **Mitigation**: 2s timeout, graceful fallback to original

## Files/Packages Affected
- crates/maproom/src/query/rewriter.rs (create)
- Cargo.toml (add reqwest, serde_json)

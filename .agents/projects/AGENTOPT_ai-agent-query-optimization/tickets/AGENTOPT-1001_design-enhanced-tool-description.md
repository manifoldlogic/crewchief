# Ticket: AGENTOPT-1001 - Design Enhanced Tool Description

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - related tests pass
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Design the enhanced MCP tool description with query transformation patterns, examples, and multi-query retry strategy based on empirical results from Phase 0 experiments. This formalizes the winning variant into production-ready documentation that teaches Claude Code how to transform natural language queries into optimal search terms.

## Background
This ticket implements Phase 1, Step 1 from the AGENTOPT project plan (planning/plan.md lines 36-50). After Phase 0 completes data-driven testing of multiple variants, this step takes the optimal variant and formalizes it into production-ready documentation. The enhanced description serves as an MCP tool description that guides Claude Code agent behavior during semantic search query formulation.

The enhanced description will teach AI agents clear patterns for:
- Extracting core technical terms from natural language queries
- Removing stop words and filler language
- Using code-like terminology preferred by the search engine
- Implementing multi-query retry strategies when initial searches don't yield results

Reference: AGENTOPT project planning documents
- Plan: planning/plan.md lines 36-50 (Phase 1 overview)
- Architecture: planning/architecture.md lines 119-189 (Enhanced description specification)

## Acceptance Criteria
- [ ] Enhanced description draft created based on Phase 0 winning variant
- [ ] Transformation patterns documented with clear examples
- [ ] Good/bad query examples included (10-15 pairs minimum)
- [ ] Multi-query retry strategy documented with concrete retry approaches
- [ ] Token count validated (<600 tokens total)
- [ ] Internal review completed with feedback incorporated

## Technical Requirements
- Start with Phase 0 winning variant identified in AGENTOPT-0006 experiment results
- Include all sections matching planning/architecture.md lines 132-177:
  - AI AGENT QUERY FORMULATION section (high-level overview)
  - TRANSFORMATION PATTERNS (extract terms, remove stop words, prefer code-like)
  - Examples (before/after transformations, minimum 10-15 pairs)
  - QUERY BEST PRACTICES (good vs avoid patterns)
  - MULTI-QUERY STRATEGY (retry with variations, fallback approaches)
- Token budget: <600 tokens total (strict limit)
- Maintain MCP schema compatibility (tool description format)
- Clear, actionable language written for AI agents (not humans)
- Use markdown formatting suitable for MCP tool descriptions

## Implementation Notes
1. Review Phase 0 experiment results (AGENTOPT-0006) to identify the winning variant and its performance metrics
2. If no clear single winner exists, analyze top 2 variants and extract best-performing patterns from each
3. Draft enhanced description in markdown format suitable for MCP tool descriptions
4. Include systematic transformation patterns:
   - Extract 2-3 core technical terms from natural language query
   - Remove common stop words: how, what, where, when, why, does, is, are, the, a, an
   - Identify code-like terminology and preserve/emphasize it
   - Handle negations and boolean operators
5. Add 10-15 before/after transformation examples from the test query set, covering:
   - Basic terminology queries
   - Multi-part queries
   - Queries with negations
   - Architecture/design queries
   - Implementation-specific queries
6. Document multi-query retry strategy including:
   - Primary query attempt and expected behavior
   - Fallback strategies for zero-result queries
   - Progressive simplification approach
   - When to stop retrying
7. Conduct internal review with project leads and incorporate feedback
8. Refine description iteratively based on feedback until approved

## Dependencies
- AGENTOPT-0006 (Phase 0 experiment results and winning variant identification)

## Risk Assessment
- **Risk**: Winning variant from Phase 0 not suitable for production use or scaling
  - **Mitigation**: Fall back to control variant + best-performing patterns from top 2 competing variants; document trade-offs
- **Risk**: Description too verbose (exceeds 600 token budget)
  - **Mitigation**: Prioritize most impactful transformation patterns; remove redundant or low-value examples; focus on clarity over comprehensiveness
- **Risk**: Patterns unclear or ambiguous to AI agents following the guidance
  - **Mitigation**: Use concrete examples for every pattern; test descriptions with sample queries before finalizing

## Files/Packages Affected
- .agents/projects/AGENTOPT_ai-agent-query-optimization/phase1/enhanced-description-draft.md (create)

## Planning References
- **Plan**: .agents/projects/AGENTOPT_ai-agent-query-optimization/planning/plan.md (lines 36-50)
- **Architecture**: .agents/projects/AGENTOPT_ai-agent-query-optimization/planning/architecture.md (lines 119-189)
- **Phase 0 Results**: AGENTOPT-0006 experiment output and analysis

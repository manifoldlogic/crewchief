# Ticket: SEMRANK-4003: Update Search Documentation

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (documentation-only ticket)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Update maproom MCP search documentation to explain semantic ranking, multipliers, debug mode, and usage examples. Ensure users understand how to leverage the new ranking for better entry points.

## Background
The SEMRANK project has implemented semantic entry point ranking that enhances maproom's FTS to return implementations over tests/docs by applying kind-based and exact-match multipliers to search results. Users need comprehensive documentation to understand how this new ranking system works, how to use it effectively, and how to troubleshoot results.

This ticket implements the documentation requirements from Phase 4 (Documentation & Deployment) of the SEMRANK project plan.

## Acceptance Criteria
- [ ] Create `/packages/maproom-mcp/docs/search-ranking.md` with sections:
  - [ ] Overview: What is semantic entry point ranking
  - [ ] How It Works: FTS × kind_multiplier × exact_match_multiplier
  - [ ] Multiplier Values: Table of kind multipliers (func: 2.5×, test: 0.6×, etc.)
  - [ ] Exact Match Bonus: 3.0× when normalized query matches symbol_name
  - [ ] Query Normalization: Explain camelCase → snake_case, acronym handling
  - [ ] Debug Mode: How to enable, what score_breakdown shows
  - [ ] Examples: Before/after comparisons with queries
  - [ ] Migration Notes: Replacing old +0.2 exact bonus
- [ ] Update `/packages/maproom-mcp/README.md`:
  - [ ] Add "Semantic Ranking" section
  - [ ] Link to detailed docs
  - [ ] Show example search with debug mode
- [ ] Update `/docs/architecture/SEARCH_ARCHITECTURE.md`:
  - [ ] Add semantic ranking section
  - [ ] Document SQL CASE statements
  - [ ] Document integration with RRF fusion
- [ ] All documentation reviewed for accuracy and clarity
- [ ] Code examples tested and verified

## Technical Requirements
- Create comprehensive search-ranking.md guide in maproom-mcp docs
- Update existing README.md with overview and links
- Update architecture documentation with implementation details
- Include practical examples with actual queries
- Document debug mode usage and output format
- Explain migration from old exact bonus (+0.2) to new multipliers

## Implementation Notes

### Documentation Structure
The new `search-ranking.md` should be the authoritative guide for semantic ranking, covering:
1. **Concept**: Why semantic ranking improves entry point discovery
2. **Mechanics**: The formula and how multipliers combine
3. **Multipliers**: Complete table with rationale for each value
4. **Exact Match**: How query normalization works (camelCase → snake_case)
5. **Debug Mode**: Enabling `debug: true` and interpreting score_breakdown
6. **Examples**: Real-world queries showing improved results
7. **Migration**: How the new system replaces the old +0.2 bonus

### Key Multiplier Values to Document
From architecture.md:
- `function`: 2.5× (primary implementations)
- `class`/`struct`/`enum`/`interface`: 2.0× (definitions)
- `method`: 1.5× (class methods)
- `test_function`: 0.6× (demote tests)
- `comment`/`doc_comment`: 0.3× (demote docs)
- Default: 1.0× (neutral)

### Debug Mode Output Format
Example score_breakdown JSON:
```json
{
  "base_score": 0.85,
  "kind_multiplier": 2.5,
  "exact_match_multiplier": 3.0,
  "final_score": 6.375
}
```

### README Update
Add a concise "Semantic Ranking" section that:
- Explains the feature in 2-3 sentences
- Links to detailed docs
- Shows a simple example with debug mode

### Architecture Documentation
Update SEARCH_ARCHITECTURE.md with:
- SQL CASE statements for kind multipliers
- Exact match detection logic
- How multipliers integrate with RRF fusion scoring
- Performance characteristics

## Dependencies
- SEMRANK-2003 (kind multiplier implementation)
- SEMRANK-2004a (exact match SQL)
- SEMRANK-2004b (query normalization)
- SEMRANK-2006 (debug mode)

## Risk Assessment
- **Risk**: Documentation becomes outdated if multipliers are tuned
  - **Mitigation**: Reference architecture.md for authoritative multiplier values; add note that values may be tuned based on monitoring
- **Risk**: Examples may not work if test corpus changes
  - **Mitigation**: Use stable examples from core codebase; verify all examples before finalizing documentation

## Files/Packages Affected
- `/packages/maproom-mcp/docs/search-ranking.md` (new file)
- `/packages/maproom-mcp/README.md`
- `/docs/architecture/SEARCH_ARCHITECTURE.md`

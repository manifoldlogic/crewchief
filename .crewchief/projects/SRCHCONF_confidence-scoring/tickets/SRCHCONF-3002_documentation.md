# Ticket: [SRCHCONF-3002]: Confidence Scoring Documentation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation-only ticket)
- [x] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- documentation-writer
- verify-ticket
- commit-ticket

## Summary
Create comprehensive documentation explaining confidence signals, their interpretation, usage patterns, and example scenarios showing high vs low confidence results.

## Background
Users need clear documentation to understand confidence signals and make informed decisions based on them. This documentation explains what each signal means, how to interpret them together, and when to trust or investigate results further.

This completes Phase 3 by providing user-facing guidance for the confidence scoring feature.

## Acceptance Criteria
- [ ] New documentation file created: `packages/maproom-mcp/docs/confidence-scoring.md`
- [ ] Documentation explains all 3 core confidence signals (source_count, score_gap, is_exact_match)
- [ ] Includes interpretation guidance for each signal
- [ ] Provides at least 3 example scenarios (high confidence, low confidence, mixed signals)
- [ ] Explains how to enable confidence scoring (include_confidence parameter)
- [ ] Documents backward compatibility (parameter optional, defaults to false)
- [ ] Mentions performance characteristics (<5ms overhead)
- [ ] Links to related documentation (search.md, query understanding)
- [ ] Markdown formatting is correct (headers, code blocks, lists)
- [ ] No spelling or grammar errors

## Technical Requirements
**Documentation Structure**:

1. **Overview**
   - What is confidence scoring
   - Why it's useful
   - When to enable it

2. **Confidence Signals** (3 core signals for MVP)
   - `source_count`: Number of search sources (1-4)
   - `score_gap`: Difference from next result
   - `is_exact_match`: Exact symbol name match

3. **Interpreting Signals**
   - High confidence indicators
   - Low confidence indicators
   - How to combine signals

4. **Usage Examples**
   ```typescript
   // Enable confidence scoring
   const results = await search({
     query: 'authenticate',
     repo: 'my-repo',
     include_confidence: true
   });

   // Check confidence signals
   results.hits.forEach(hit => {
     if (hit.confidence) {
       console.log(`Sources: ${hit.confidence.source_count}`);
       console.log(`Score gap: ${hit.confidence.score_gap}`);
       console.log(`Exact match: ${hit.confidence.is_exact_match}`);
     }
   });
   ```

5. **Example Scenarios**
   - **High Confidence**: 3+ sources, exact match, large gap
   - **Low Confidence**: 1 source, no exact match, small gap
   - **Mixed Signals**: Interpretation guidance

6. **Performance**
   - <5ms overhead
   - Opt-in via parameter (default false)
   - No database queries

7. **Backward Compatibility**
   - Parameter optional
   - Existing calls work unchanged
   - Response structure unchanged when disabled

## Implementation Notes
Documentation should follow maproom-mcp documentation style:
- Use clear, concise language
- Provide code examples
- Explain concepts before technical details
- Include visual indicators (emojis for high/low confidence examples)

Example scenario template:
```markdown
### High Confidence Example

**Query**: `authenticate_user`

**Result**:
- `source_count: 3` - Appears in FTS, vector, and graph
- `score_gap: 2.5` - Large separation from next result
- `is_exact_match: true` - Exact function name match

**Interpretation**: Very high confidence. This is likely the exact function you're looking for. Multiple search methods agree, it's far ahead of alternatives, and the name matches exactly.
```

Reference quality-strategy.md for signal interpretation:
- source_count: 3-4 = high, 1-2 = lower
- score_gap: >1.0 = large separation, <0.1 = ambiguous
- is_exact_match: true = high confidence indicator

## Dependencies
- **Prerequisite**: SRCHCONF-3001 (MCP tool must expose parameter before documenting it)
- **Prerequisite**: Phase 1-2 complete (feature must be implemented before documenting)

## Risk Assessment
- **Risk**: Documentation becomes outdated as feature evolves
  - **Mitigation**: Link to code (TYPE_SYNC comments), date documentation, plan to update with Phase 2 additions.
- **Risk**: Users misinterpret signals
  - **Mitigation**: Clear examples, explicit interpretation guidance, avoid ambiguous language.

## Files/Packages Affected
- `packages/maproom-mcp/docs/confidence-scoring.md` - NEW documentation file
- `packages/maproom-mcp/README.md` - Add link to confidence-scoring.md

## Verification Notes
The verify-ticket agent should check:
- All 3 confidence signals explained clearly
- At least 3 example scenarios provided
- Code examples are syntactically correct
- Interpretation guidance helps users make decisions
- Performance characteristics mentioned (<5ms overhead)
- Backward compatibility documented
- No broken links, spelling errors, or formatting issues
- Documentation is accessible (linked from README)

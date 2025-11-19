# Ticket: SEMRANK-2006: Add Debug Mode Score Breakdown

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - tests executed and passing (or N/A if no tests)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- database-engineer
- unit-test-runner
- verify-ticket
- commit-ticket

## Summary
Return score_breakdown object when debug=true with base_fts, kind_multiplier, exact_match_multiplier, final scores; update MCP tool response schema; add permission check.

## Background
During development and tuning of semantic ranking, visibility into scoring components is essential. Why did "authenticate" rank above "validateProvider"? Was it the kind multiplier, exact match, or both?

Debug mode returns a score_breakdown object showing all intermediate values: base FTS score, kind multiplier, exact match multiplier, and final combined score. This enables tuning multiplier values and understanding ranking decisions.

As a best practice, debug mode should be restricted to admin/operator users. However, if no authentication system exists, we'll log a warning and allow it (the metadata is not sensitive for MVP).

References SEMRANK plan Section 3.3 (Debug Mode Implementation).

## Acceptance Criteria
- [ ] Debug mode parameter accepted in search tool (debug: boolean, default false)
- [ ] When debug=true, return score_breakdown object in results
- [ ] Score breakdown includes: `{ base_fts, kind_multiplier, exact_match_multiplier, final }`
- [ ] MCP tool response schema updated to include optional score_breakdown field
- [ ] Permission check implemented OR documented as future enhancement if no auth system exists
- [ ] Debug mode tested: Returns expected breakdown for sample queries
- [ ] All score values in breakdown match SQL query output

## Technical Requirements
- Location: `/packages/maproom-mcp/src/tools/search.ts`
- Add debug parameter to Zod schema: `debug: z.boolean().optional().default(false)`
- Modify TypeScript to return breakdown when debug=true:
  ```typescript
  if (params.debug) {
    // Check if auth system exists
    if (typeof user !== 'undefined' && user.hasPermission) {
      if (!user.hasPermission('debug_mode')) {
        throw new Error('Debug mode requires admin permissions');
      }
    } else {
      console.warn('Debug mode enabled without permission check');
    }

    return results.map(r => ({
      ...r,
      score_breakdown: {
        base_fts: r.base_score,
        kind_multiplier: r.kind_mult,
        exact_match_multiplier: r.exact_mult,
        final: r.final_score
      }
    }));
  }
  ```
- SQL query already returns base_score, kind_mult, exact_mult, final_score (from SEMRANK-2003, 2004a, 2005)
- TypeScript maps these to score_breakdown format
- If no auth system exists: Log warning, allow debug (metadata not sensitive)

## Implementation Notes
**Step 1: Add Debug Parameter**
- Update Zod schema for search tool parameters
- Add debug: boolean field with default false
- Update TypeScript types to match

**Step 2: Implement Permission Check**
- Check if auth system exists (user object available)
- If exists, verify user.hasPermission('debug_mode')
- If not exists, log warning and proceed (acceptable for MVP)

**Step 3: Return Score Breakdown**
- Map SQL result columns to score_breakdown object
- Only include breakdown when debug=true
- Verify all fields present and values correct

**Step 4: Testing**
- Test with debug=true: Verify all fields present
- Test with debug=false: Verify breakdown not included
- Test values: base_fts × kind_multiplier × exact_match_multiplier = final
- Document permission check as future enhancement if auth missing

## Dependencies
- SEMRANK-2005 (final_score computed, all columns available)

## Risk Assessment
- **Risk**: No auth system exists, debug mode unrestricted
  - **Mitigation**: Allow with warning (acceptable for MVP, metadata only, not sensitive)
- **Risk**: Schema changes break MCP protocol
  - **Mitigation**: Follow existing patterns, make field optional
- **Risk**: Performance impact from returning extra data
  - **Mitigation**: Debug mode opt-in only, minimal data overhead

## Files/Packages Affected
- `/packages/maproom-mcp/src/tools/search.ts`

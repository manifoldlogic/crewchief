# Ticket: TOOLOPT-3002: Create variant-e-task-mapping with task-to-query section

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (JSON file creation, validation in next ticket)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- Tests pass - N/A: This is JSON file creation work, validation occurs in TOOLOPT-3003

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Clone variant-a-detailed, integrate task-to-query mapping section, and save as new variant-e-task-mapping.json with proper metadata.

## Background
Combine proven winner (variant-a-detailed) with new task-to-query mapping section to create enhanced variant targeting >20% performance. This addresses the critical gap identified in genetic optimization analysis where agents receive tasks but lack systematic guidance for converting task types into effective search strategies.

This ticket implements the variant creation phase of Phase 3 from the TOOLOPT project plan.

## Acceptance Criteria
- [ ] New file created: `/workspace/packages/maproom-mcp/test/tool-description-optimization/variants/variant-e-task-mapping.json`
- [ ] Variant successfully cloned from variant-a-detailed as base
- [ ] Task-to-query section inserted after transformation patterns (before SEARCH MODES)
- [ ] Metadata updated with all required fields:
  - id: "variant-e-task-mapping"
  - name: "Task Mapping Enhanced"
  - generation: 11 (next after current genetic runs)
  - parent_ids: ["variant-a-detailed"]
  - mutations: ["enhancement"]
- [ ] Token count calculated and documented in metadata
- [ ] JSON validates correctly (proper escaping, structure, no syntax errors)

## Technical Requirements
- Source file: `/workspace/packages/cli/.crewchief/genetic-iterations/ultra-run-1763154816350/variants/variant-a-detailed.json`
- Insert task-to-query section strategically after transformation workflow, before SEARCH MODES
- Update all metadata fields to reflect new variant identity
- Calculate new token count using same tokenizer as genetic optimizer (tiktoken)
- Verify JSON schema compliance with variant format
- Properly escape all JSON strings (newlines, quotes, etc.)
- Maintain variant-a-detailed's proven structure while adding enhancement

## Implementation Notes
Section placement strategy (variant structure):
1. Opening (BEST FOR / USE WHEN / NOT FOR)
2. Transformation workflow ✅
3. **[NEW] Task-to-query mapping** ← Insert here
4. SEARCH MODES
5. Examples and filters

Rationale for placement: Task mapping builds on transformation workflow conceptually, should come before mode selection to provide foundational guidance.

Metadata fields to update:
```json
{
  "id": "variant-e-task-mapping",
  "name": "Task Mapping Enhanced",
  "generation": 11,
  "parent_ids": ["variant-a-detailed"],
  "mutations": ["enhancement"],
  "description": "Enhanced variant with task-to-query mapping addressing critical gap in task→strategy conversion",
  "token_count": [calculated value]
}
```

JSON string escaping:
- Newlines: `\n`
- Quotes: `\"`
- Backslashes: `\\`
- Ensure multi-line content from markdown is properly escaped

Token counting:
- Use same approach as genetic optimizer
- Target: <600 tokens total
- Document final count in metadata

## Dependencies
- TOOLOPT-3001 (task-to-query section designed and drafted)

## Risk Assessment
- **Risk**: JSON escaping errors in multi-line content
  - **Mitigation**: Use proper JSON encoding, validate with JSON parser before saving
- **Risk**: Token count may exceed 600 token budget
  - **Mitigation**: Review section length, trim examples if needed to stay within budget
- **Risk**: Section placement may disrupt variant-a-detailed's flow
  - **Mitigation**: Test logical flow after insertion, ensure sections connect smoothly
- **Risk**: Metadata fields may not match genetic optimizer expectations
  - **Mitigation**: Follow exact schema from existing variants, verify all required fields present

## Files/Packages Affected
- `/workspace/packages/maproom-mcp/test/tool-description-optimization/variants/variant-e-task-mapping.json` (created)
- Source reference: `/workspace/packages/cli/.crewchief/genetic-iterations/ultra-run-1763154816350/variants/variant-a-detailed.json`

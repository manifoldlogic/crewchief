# Ticket: TOOLOPT-3003: Validate enhanced variant format and quality

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - validation checks executed and passing
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- Validation checks must be executed and shown passing (JSON validation, token count, schema compliance)

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Load variant-e-task-mapping in test framework and verify schema compliance, token budget, section ordering, and example quality.

## Background
Before using variant-e-task-mapping in future genetic runs, need to confirm it meets all format requirements and quality standards. This validation ensures the variant will work correctly with the genetic optimization framework and provides value when tested.

This ticket implements the validation phase of Phase 3 from the TOOLOPT project plan.

## Acceptance Criteria
- [ ] Variant loads successfully in test framework without errors
- [ ] Schema validation passes (all required metadata fields present and correctly formatted)
- [ ] Token count within budget (<600 tokens total)
- [ ] Section ordering is logical and consistent with design
- [ ] Examples are concrete and actionable (not abstract)
- [ ] No JSON syntax errors (validated with parser)
- [ ] Metadata fields properly set with correct values
- [ ] Ready for genetic optimization testing (compatible with framework)

## Technical Requirements
- Use existing variant validation utilities from genetic optimization framework
- Token count check: must be <600 tokens (document actual count)
- JSON schema validation against variant schema
- Load in comparison test framework to verify compatibility
- Check string escaping and formatting (newlines, quotes properly escaped)
- Verify all required metadata fields:
  - id, name, generation, parent_ids, mutations, description, token_count
- Confirm section ordering matches design specification

## Implementation Notes
Validation checklist (all must pass):
- ✓ JSON syntax valid (parses without errors)
- ✓ All metadata fields present and correct type
- ✓ Token count <600 tokens
- ✓ Section ordering matches design (task-to-query after transformation, before modes)
- ✓ Task-to-query examples are concrete and domain-specific
- ✓ No duplicate content from variant-a-detailed (only intended sections present)
- ✓ Transformation workflow intact from parent variant
- ✓ Compatible with test framework (loads and can be used in comparisons)

Can use validation approaches:
```bash
# JSON syntax validation
node -e "JSON.parse(require('fs').readFileSync('variant-e-task-mapping.json'))"

# Token counting (if utility exists)
npx tsx src/search-optimization/count-tokens.ts variant-e-task-mapping

# Schema validation (if utility exists)
npx tsx src/search-optimization/validate-variant.ts variant-e-task-mapping
```

If no dedicated validation utilities exist, perform manual checks:
1. Load JSON and parse successfully
2. Count tokens using tiktoken
3. Verify metadata structure
4. Check section ordering
5. Review example quality

Document all validation results in ticket completion report.

## Dependencies
- TOOLOPT-3002 (variant-e-task-mapping.json created)

## Risk Assessment
- **Risk**: Validation utilities may not exist in codebase
  - **Mitigation**: Perform manual validation using JSON parser, tiktoken, and visual inspection
- **Risk**: Token count may be borderline (near 600 limit)
  - **Mitigation**: If over budget, trim examples or restructure section (requires returning to TOOLOPT-3002)
- **Risk**: Schema may have changed since variant-a-detailed was created
  - **Mitigation**: Compare metadata fields with most recent variants in genetic runs
- **Risk**: Test framework compatibility issues may be subtle
  - **Mitigation**: Attempt to load variant in actual framework code, don't rely solely on schema validation

## Files/Packages Affected
- `/workspace/packages/maproom-mcp/test/tool-description-optimization/variants/variant-e-task-mapping.json` (validated)
- Potentially validation utility scripts if they exist in codebase

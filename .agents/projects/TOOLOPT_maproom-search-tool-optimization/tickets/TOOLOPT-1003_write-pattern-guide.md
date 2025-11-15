# Ticket: TOOLOPT-1003: Create reusable tool description patterns guide

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation only)
- [x] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Extract winning patterns from genetic optimization as actionable templates and create how-to guide for writing effective AI agent tool descriptions.

## Background
The genetic optimization experiment revealed specific patterns that improve tool description effectiveness. While TOOLOPT-1002 documents the raw results, this ticket creates actionable guidance that developers can use when writing tool descriptions for other tools in the system. This transforms research findings into practical, reusable patterns.

This implements the documentation phase from TOOLOPT project plan - making optimization insights actionable.

## Acceptance Criteria
- [ ] `docs/optimization/tool-description-patterns.md` completed with:
  - [ ] Winning patterns documented as reusable templates
  - [ ] Transformation workflow pattern (numbered steps, before→after examples)
  - [ ] How-to guide for writing tool descriptions
  - [ ] Good vs bad pattern comparisons with concrete examples
  - [ ] Decision tree or guidelines for choosing patterns
- [ ] Actionable guidance (not just theoretical analysis)
- [ ] Examples that can be adapted to other tools
- [ ] Clear before/after examples showing pattern application

## Technical Requirements
- Extract patterns from variant-a-detailed (winner at 19.6%)
- Document imperative command structure (vs descriptive advice)
- Include emoji usage patterns (🤖 for AI agent sections)
- Cover complete query lifecycle: transformation → execution → recovery → boundaries
- Markdown formatting with code blocks for examples
- Template sections that can be copied and adapted

## Implementation Notes
Winning patterns to document:
1. **Transformation workflow** - how to convert natural language → queries
   - Numbered rules (3-4 steps)
   - Before→After examples with arrows
   - Complete query lifecycle coverage
   - Imperative commands ("Extract 2-3 terms") vs descriptive ("Best for...")

2. **Structure patterns**:
   - AI Agent Query Transformation section (with 🤖)
   - Systematic step-by-step guidance
   - Concrete examples with annotations
   - Error recovery guidance

3. **Content patterns**:
   - Focus on "how to transform" not "when to use"
   - Include boundary cases and limitations
   - Show query refinement strategies

Anti-patterns to document (what to avoid):
- Static examples without transformation guidance
- Over-documenting alternative tools (Grep/Glob comparisons)
- Excessive brevity (<300 tokens)
- Missing systematic transformation guidance
- Purely descriptive language vs imperative commands

Document structure suggestion:
1. Overview - what makes tool descriptions effective
2. Winning patterns (with templates)
3. Anti-patterns to avoid
4. How-to guide (step-by-step for writing new descriptions)
5. Examples (before/after transformations)
6. Decision framework (when to use which patterns)

## Dependencies
- TOOLOPT-1002 (optimization results document must exist for context)

## Risk Assessment
- **Risk**: Patterns too specific to search tool, not generalizable
  - **Mitigation**: Extract principles, not just specific wording; test patterns against other tool types
- **Risk**: Guide becomes too prescriptive, stifles creativity
  - **Mitigation**: Present as patterns and guidelines, not rigid rules; include decision framework

## Files/Packages Affected
- `/workspace/docs/optimization/tool-description-patterns.md` (create/populate)
- Reference: `/workspace/docs/optimization/genetic-optimization-results.md` (read for context)
- Source variants: `/workspace/packages/cli/.crewchief/genetic-iterations/ultra-run-1763154816350/variants/` (read)

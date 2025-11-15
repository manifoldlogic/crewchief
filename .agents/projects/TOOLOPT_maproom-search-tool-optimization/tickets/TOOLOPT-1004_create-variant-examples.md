# Ticket: TOOLOPT-1004: Export variant examples with annotations

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (documentation only)
- [ ] **Verified** - by the verify-ticket agent

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Create annotated markdown examples of key genetic optimization variants (winner, baseline, enhancement) for reference and comparison.

## Background
Having concrete examples of actual variants tested during optimization provides valuable reference material. While the patterns guide (TOOLOPT-1003) extracts general principles, these annotated examples show complete real-world implementations that developers can study and adapt. Preserving the winner, baseline, and enhancement variants allows direct comparison and learning.

This implements the documentation phase from TOOLOPT project plan - preserving concrete examples.

## Acceptance Criteria
- [ ] `docs/optimization/examples/variant-a-detailed.md` created (winner, 19.6%)
- [ ] `docs/optimization/examples/variant-control.md` created (baseline, 17.7%)
- [ ] `docs/optimization/examples/variant-e-task-mapping.md` created (future enhancement, if exists)
- [ ] Each example includes:
  - [ ] Performance score and generation number
  - [ ] Annotated sections explaining key features
  - [ ] Highlighted differences from other variants
  - [ ] Token count
  - [ ] Key patterns used (reference to patterns guide)
- [ ] Original variant content preserved accurately
- [ ] Annotations clearly distinguished from original content

## Technical Requirements
- Source directory: `/workspace/packages/cli/.crewchief/genetic-iterations/ultra-run-1763154816350/variants/`
- Convert JSON variant descriptions to readable markdown
- Add comparison annotations (using blockquotes or callouts)
- Maintain original content integrity (don't modify the actual variant text)
- Include metadata (generation, score, token count)
- Cross-reference to patterns documented in TOOLOPT-1003

## Implementation Notes
Variants to document:
1. **variant-a-detailed**: Winner with transformation workflow
   - Highlight the transformation workflow section
   - Note emoji usage (🤖) and structural patterns
   - Annotate the numbered steps and before→after examples
   - Compare token count and performance

2. **variant-control**: Baseline for comparison
   - Show what was original/standard approach
   - Highlight differences from winner
   - Note what's missing vs variant-a-detailed

3. **variant-e-task-mapping**: Enhancement proposal (if created in optimization)
   - Show proposed improvements
   - Note experimental nature

Format suggestion for each file:
```markdown
# Variant: [name]

## Metadata
- **Generation**: X
- **Performance Score**: X.X%
- **Token Count**: XXX
- **Status**: Winner/Baseline/Experimental

## Key Features
- [Feature 1]
- [Feature 2]

## Original Variant Content
[Full variant description here]

## Annotations
### Section 1: [Name]
> [Annotation explaining what makes this effective/ineffective]

### Comparison to Other Variants
[Differences and insights]
```

Preserve JSON structure if variants are in JSON format, or extract the description field if that's the primary content.

## Dependencies
- TOOLOPT-1002 (optimization results for context and metadata)

## Risk Assessment
- **Risk**: Variant files not in expected format
  - **Mitigation**: Inspect source files first; adapt extraction approach to actual format
- **Risk**: Annotations mislead or misinterpret variant intent
  - **Mitigation**: Base annotations on performance data and documented patterns

## Files/Packages Affected
- `/workspace/docs/optimization/examples/variant-a-detailed.md` (create)
- `/workspace/docs/optimization/examples/variant-control.md` (create)
- `/workspace/docs/optimization/examples/variant-e-task-mapping.md` (create, if source exists)
- Source (read-only): `/workspace/packages/cli/.crewchief/genetic-iterations/ultra-run-1763154816350/variants/`

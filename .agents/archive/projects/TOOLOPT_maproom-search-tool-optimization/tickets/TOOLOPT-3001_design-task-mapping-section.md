# Ticket: TOOLOPT-3001: Design task-to-query mapping section for enhanced variant

## Status
- [ ] **Task completed** - acceptance criteria met
- [ ] **Tests pass** - N/A (design work)
- [ ] **Verified** - by the verify-ticket agent

**Note on "Tests pass"**:
- Tests pass - N/A: This is design and documentation work with no code to test

## Agents
- general-purpose
- verify-ticket
- commit-ticket

## Summary
Design and draft the task-to-query mapping section that addresses the critical gap identified in genetic optimization analysis - teaching agents how to derive search strategies from high-level task goals.

## Background
Analysis revealed that current tool descriptions teach "question→query transformation" but lack "task→query mapping". Agents receive tasks like "Find where X is implemented" but don't have systematic guidance for converting task types into effective search strategies. This gap likely explains the 19-20% plateau observed in genetic optimization runs.

This ticket implements the enhancement design phase of Phase 3 from the TOOLOPT project plan, creating the foundation for variant-e-task-mapping.

## Acceptance Criteria
- [ ] Task-to-query section designed with clear structure following the pattern:
  ```markdown
  🎯 TASK-TO-QUERY MAPPING:

  FINDING IMPLEMENTATION:
  Task: "Find where X is implemented"
  Query: "[component] [action]"
  Examples: ...

  UNDERSTANDING ARCHITECTURE:
  Task: "Understand how X works"
  Query: "[system] [flow/architecture]"
  Examples: ...

  [Additional task categories...]
  ```
- [ ] 4 task categories defined with complete structure:
  1. Finding Implementation
  2. Understanding Architecture
  3. Debugging/Tracing
  4. Exploring Dependencies
- [ ] Each category includes all required elements:
  - Task description pattern (what agents typically receive)
  - Query strategy (how to transform task into search query)
  - 2-3 concrete examples showing transformation
- [ ] Section draft completed and ready for integration into variant JSON
- [ ] Token budget considered and section stays within <100 tokens

## Technical Requirements
- Follow variant-a-detailed structural patterns for consistency
- Use consistent emoji and formatting (🎯 for task mapping)
- Maintain imperative command tone matching existing sections
- Keep examples concrete and actionable (avoid abstract guidance)
- Total addition should not exceed 100 tokens to stay within budget
- Examples should use CrewChief/maproom domain when possible

## Implementation Notes
Task categories rationale:
- **Finding Implementation**: Most common agent task, needs direct mapping from "find X" to component queries
- **Understanding Architecture**: Requires broader system queries, flow-based searches
- **Debugging/Tracing**: Flow-based search strategies, execution path queries
- **Exploring Dependencies**: Relationship and import patterns, connection queries

Example structure for each category:
```
FINDING IMPLEMENTATION:
Task: "Find where X is implemented"
→ Query: "[component] [action/noun]"
Examples:
- "Find user auth" → "user authenticate"
- "Find DB connection" → "database connect"
```

The section should be self-contained and insertable after the transformation workflow section in variant-a-detailed, before the SEARCH MODES section.

Design considerations:
- Keep examples domain-specific (use maproom, worktree, indexing concepts)
- Show clear before/after transformation
- Use → arrow to indicate transformation
- Match tone and style of existing variant-a-detailed sections

## Dependencies
None (design work can proceed independently)

## Risk Assessment
- **Risk**: Section design may not align with variant-a-detailed structure
  - **Mitigation**: Review variant-a-detailed format before designing, maintain consistent emoji/formatting patterns
- **Risk**: Token budget may be exceeded with 4 categories and examples
  - **Mitigation**: Keep examples concise, use 2 examples per category instead of 3 if needed
- **Risk**: Task categories may not cover common agent use cases
  - **Mitigation**: Base categories on actual agent behavior patterns observed in optimization runs

## Files/Packages Affected
- New draft document to be created in planning or work-in-progress area
- No code files affected (design phase only)

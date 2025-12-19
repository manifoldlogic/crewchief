# Task: [PLUGIN-001.2002]: Create Search Best Practices Reference

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation task, no executable tests)
- [x] **Verified** - by the verify-task agent

## Agents
- general-implementation
- verify-task
- commit-task

## Summary
Create the search-best-practices.md reference document with 10+ query transformation examples, search strategy patterns, and anti-patterns.

## Background
The search-best-practices.md reference provides detailed examples that supplement the core SKILL.md documentation. It teaches query transformation, SearchMode detection patterns, and task-based search strategies through concrete examples.

This task completes the "Skill Implementation" phase from plan.md by providing the progressive disclosure reference that Claude can consult for advanced guidance.

## Acceptance Criteria
- [x] File `search-best-practices.md` created in `skills/maproom-search/references/` directory
- [x] Document contains 10+ query transformation examples
- [x] Examples show transformation from natural language to effective queries
- [x] Each example includes SearchMode detection pattern (Code/Text/Auto)
- [x] Document includes search strategy patterns section
- [x] Strategy patterns cover at least 3 task types (e.g., architecture exploration, debugging, feature discovery)
- [x] Document includes anti-patterns section
- [x] Anti-patterns list includes at least 5 common mistakes
- [x] Examples are concrete and actionable (no abstract placeholders)
- [x] Examples demonstrate both FTS and vector search use cases
- [x] All query examples are 2-3 words (demonstrating best practice)
- [x] Formatting is consistent (tables for examples, lists for strategies)
- [x] No placeholder content remains

## Technical Requirements
- Markdown formatting with tables for examples
- Consistent structure: example tables, strategy sections, anti-pattern lists
- Examples must demonstrate SearchMode auto-detection intelligence
- Query transformations show concept extraction technique
- Strategy patterns are task-oriented (what user wants to accomplish)
- Anti-patterns explain why approach fails and what to do instead

## Implementation Notes

### Query Transformation Examples Table
Create table with columns:
- Natural Language Query
- Transformed Query (2-3 words)
- SearchMode Detection (Code/Text/Auto)
- Rationale

Example entries:
| Natural Language | Transformed Query | SearchMode | Rationale |
|------------------|-------------------|------------|-----------|
| "How does authentication work in this codebase?" | "authentication" | Code | Single word, likely code identifier |
| "Find the user profile API endpoint" | "user profile api" | Auto | 3 words, mixed concepts |
| "Explain how to handle database connections" | "database connection" | Auto | 2 words, conceptual |
| "Where is UserAuth::login() implemented?" | "UserAuth::login()" | Code | Code pattern detected |
| "What are the best practices for error handling?" | "error handling" | Auto | 2 words, conceptual |

Provide 10+ such examples covering various query types.

### Search Strategy Patterns
Organize by task type:

**Architecture Exploration**:
- Start broad ("authentication"), narrow with context
- Use vector search for conceptual understanding
- Follow with context expansion (callers/callees)

**Debugging**:
- Search for error/exception handling
- Find similar code patterns
- Check test coverage

**Feature Discovery**:
- Search by feature name or concept
- Use FTS for known identifiers
- Use vector search for similar implementations

**Code Navigation**:
- Start with status to check embeddings
- Use search for quick identifier lookup
- Use context to explore relationships

### Anti-Patterns to Document
1. **Full sentence queries**: "How do I authenticate users in this application?" → Too verbose, extract "authentication"
2. **Over-specific queries**: "UserAuthenticationServiceImplV2" → Start broader, narrow with context
3. **Multiple unrelated concepts**: "authentication database logging" → Search each separately
4. **No status check**: Trying vector search without checking embeddings available
5. **Ignoring SearchMode signals**: Fighting auto-detection instead of leveraging it
6. **Using maproom for exact matches**: "TODO: fix this" → Use Grep instead
7. **Using maproom for file patterns**: "*.test.ts" → Use Glob instead

### SearchMode Detection Pattern Examples
Show how the system detects mode:
- Single word, camelCase/snake_case → Code mode
- 2-3 words, mixed → Auto mode
- Natural language question → Text mode
- Code patterns (::, ->, .) → Code mode

Emphasize that auto-detection is intelligent and rarely needs override.

### Content Structure
1. Introduction (purpose of reference)
2. Query Transformation Examples (table with 10+ entries)
3. Search Strategy Patterns (by task type)
4. SearchMode Detection Patterns
5. Anti-Patterns to Avoid
6. Advanced Techniques (optional)

## Dependencies
- PLUGIN-001.1001 (directory structure must exist)
- PLUGIN-001.2001 (SKILL.md references this file)

## Risk Assessment
- **Risk**: Examples too abstract or not actionable
  - **Mitigation**: Use concrete code concepts, actual query patterns
- **Risk**: Insufficient example count (<10)
  - **Mitigation**: Create diverse examples covering different query types
- **Risk**: Examples don't demonstrate SearchMode detection
  - **Mitigation**: Explicitly show how system detects Code/Text/Auto for each example

## Files/Packages Affected
- `.crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search/references/search-best-practices.md` (new)

## Deliverables Produced

Documents created in references directory:

- search-best-practices.md - Comprehensive query examples, strategy patterns, and anti-patterns with SearchMode detection guidance

## Verification Notes

The verify-task agent should:
1. Count query transformation examples (must be 10+)
2. Verify each example has all columns: Natural Language, Transformed Query, SearchMode, Rationale
3. Confirm transformed queries are 2-3 words (demonstrating best practice)
4. Check SearchMode detection patterns are explained
5. Verify strategy patterns cover at least 3 task types
6. Count anti-patterns (minimum 5)
7. Confirm examples are concrete (no "[TODO]" or abstract placeholders)
8. Verify table formatting is consistent
9. Check that examples demonstrate both FTS and vector use cases
10. Confirm no placeholder content remains

Example validation command:
```bash
# Count table rows (should be 10+ excluding header)
grep -c "^|" search-best-practices.md
```

## Verification Audit
<!-- Audit log maintained by verify-task agent for enterprise compliance -->
| Date | Agent | Decision | Notes |
|------|-------|----------|-------|
| 2025-12-17 | verify-task | PASS | All 13 acceptance criteria met, 15 query examples, 6 strategy patterns, 10 anti-patterns, deliverable complete |

# Task: [PLUGIN-001.2001]: Create maproom-search Skill Documentation

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation task, no executable tests)
- [x] **Verified** - by the verify-task agent

## Agents
- general-implementation
- verify-task
- commit-task

## Summary
Create the maproom-search skill documentation (SKILL.md) with YAML frontmatter, decision tree, query formulation patterns, and CLI command reference.

## Background
The maproom-search skill teaches Claude when and how to use semantic code search instead of native tools like Grep. It provides query formulation guidance, command selection criteria, and error handling workflows.

This task implements the core skill documentation from the "Skill Implementation" phase in plan.md. The skill must activate on conceptual queries while avoiding activation for simple text/file pattern searches.

## Acceptance Criteria
- [ ] File `SKILL.md` created in `skills/maproom-search/` directory
- [ ] YAML frontmatter is valid and parseable
- [ ] Frontmatter field `name` is "maproom-search" (lowercase, hyphens only)
- [ ] Frontmatter field `description` is under 1024 characters
- [ ] Description mentions semantic code search and conceptual queries
- [ ] Description states when to use maproom vs grep/glob
- [ ] Body includes decision tree section (when maproom vs grep vs glob)
- [ ] Body includes query formulation patterns section with 2-3 examples
- [ ] Body includes command selection guidance (search vs vector-search)
- [ ] Body includes CLI command reference with correct syntax
- [ ] CLI commands documented: search, vector-search, status, context
- [ ] NO `--mode` flags in command examples (uses separate commands)
- [ ] Body includes SearchMode awareness section (Code/Text/Auto auto-detection)
- [ ] SearchMode section explains auto-detection, not manual override
- [ ] Body includes error handling section
- [ ] Error handling covers: no results, database not indexed, embeddings missing
- [ ] Body references search-best-practices.md for detailed examples
- [ ] All instructions use imperative form (verb-first)
- [ ] Examples are copy-paste ready with actual command syntax
- [ ] No placeholder content remains

## Technical Requirements
- YAML frontmatter delimiter: `---` on separate lines
- Frontmatter fields: `name` (string), `description` (string)
- Description optimized for skill activation matching
- Command syntax verified against crewchief-maproom CLI source
- Markdown formatting: consistent headers, code blocks with bash syntax
- Reference links use relative paths
- SearchMode auto-detection is presented as system intelligence, not user choice

## Implementation Notes

### Frontmatter Template
```yaml
---
name: maproom-search
description: This skill should be used for semantic code search when exploring unfamiliar codebases, finding implementations by concept (e.g., "authentication", "error handling"), or understanding code architecture. Uses the crewchief-maproom CLI for FTS and vector search. Prefer native Grep for exact text matches and Glob for file patterns.
---
```

### Decision Tree Content
The skill must clearly differentiate:
- **Use maproom when**: Conceptual queries, understanding architecture, finding implementations by concept
- **Use Grep when**: Exact text search, known identifiers, literal strings
- **Use Glob when**: File pattern matching, finding files by extension/name

### Query Formulation Guidance
Teach transformation of natural language to effective queries:
- "How does authentication work?" → "authentication"
- "Find the database connection logic" → "database connection"
- "Where is error handling implemented?" → "error handling"
Emphasize 2-3 word queries, concept extraction, avoiding full sentences.

### Command Selection Guidance
Explain when to choose each command:
- **search (FTS)**: Keyword matching, always works, faster, good for identifiers
- **vector-search**: Semantic similarity, requires embeddings, better for concepts
- **status**: Check repo availability and embedding status FIRST
- **context**: Expand from search results (callers, callees, tests)

### SearchMode Awareness
Explain the auto-detection system:
- **Code mode**: Single words, code patterns (e.g., "authentication", "UserAuth::login()")
- **Text mode**: Natural language questions (e.g., "how to authenticate users")
- **Auto mode**: Mixed queries (e.g., "user authentication")
The system detects mode automatically based on query structure - no manual override needed.

### CLI Command Reference
Document verified commands from crates/maproom/CLAUDE.md:
```bash
# Check status (run this first)
crewchief-maproom status --repo <repo>

# FTS search
crewchief-maproom search --query "<query>" --repo <repo> [--k N]

# Vector search
crewchief-maproom vector-search --query "<query>" --repo <repo> [--k N] [--threshold 0.7]

# Context expansion
crewchief-maproom context --chunk-id <id> [--callers] [--callees] [--tests] [--json]
```

### Error Handling Workflows
- **No results**: Try broader query, check if repo is indexed
- **Database not indexed**: Run `crewchief-maproom scan --repo <repo>`
- **Embeddings missing**: Vector search unavailable, use FTS search instead
- **Repository not found**: Use `status` without `--repo` to list available repos

### Content Structure
1. YAML frontmatter
2. Overview section
3. Decision tree (when to use maproom vs grep vs glob)
4. Query formulation patterns
5. Command selection guidance
6. SearchMode awareness
7. CLI command reference
8. Error handling
9. Reference to search-best-practices.md

## Dependencies
- PLUGIN-001.1001 (directory structure must exist)

## Risk Assessment
- **Risk**: Description too long (>1024 chars)
  - **Mitigation**: Focus on trigger conditions, keep concise
- **Risk**: Skill doesn't activate on conceptual queries
  - **Mitigation**: Test description with sample queries, refine wording
- **Risk**: CLI commands incorrect or outdated
  - **Mitigation**: Reference crates/maproom/CLAUDE.md as source of truth
- **Risk**: Skill defaults to one mode instead of teaching informed choice
  - **Mitigation**: Present both FTS and vector as valuable tools with different use cases

## Files/Packages Affected
- `.crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search/SKILL.md` (new)

## Deliverables Produced

Documents created in skills directory:

- SKILL.md - Core skill documentation with decision tree, query patterns, CLI commands, and error handling

## Verification Notes

The verify-task agent should:
1. Parse YAML frontmatter (should not error)
2. Verify `name` field is lowercase with hyphens only
3. Measure `description` character count (<1024)
4. Confirm decision tree clearly states when to use maproom vs grep vs glob
5. Count query formulation examples (minimum 2-3)
6. Verify command selection guidance explains search vs vector-search
7. Check CLI commands match verified syntax (no `--mode` flags)
8. Confirm SearchMode section explains auto-detection (not manual override)
9. Verify error handling covers all three scenarios
10. Check all instructions use imperative form
11. Test example commands are syntactically correct
12. Confirm no placeholder content remains

## Verification Audit
<!-- Audit log maintained by verify-task agent for enterprise compliance -->
| Date | Agent | Decision | Notes |
|------|-------|----------|-------|
| 2025-12-17 | verify-task | PASS | All 21 acceptance criteria met, SKILL.md deliverable complete (6131 bytes, 8 sections, 5 query examples, correct CLI syntax, no --mode flags) |
<!-- Entries added automatically during verification -->

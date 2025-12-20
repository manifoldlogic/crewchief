# Task: [MPRSKL.2001]: Create brief SKILL.md

## Status
- [x] **Task completed** - acceptance criteria met
- [x] **Tests pass** - N/A (documentation-only task)
- [x] **Verified** - by the verify-task agent

**Note on "Tests pass"**:
- If tests were created/modified, you MUST run them and show output
- "Tests pass" means tests were EXECUTED and all passed
- "Tests pass - N/A" is only valid for documentation-only tickets
- Test file existence alone does NOT satisfy this requirement

## Agents
- general
- verify-task
- commit-task

## Summary
Rewrite the maproom skill's SKILL.md to under 50 lines using progressive disclosure pattern, making it optimized for AI agent consumption with clear capability summary, when-to-use guidance, and links to detailed references.

**Note:** This task REPLACES the existing 196-line SKILL.md file with a brief version, not creates a new file alongside it.

## Background
Current SKILL.md is 196 lines and comprehensive but not optimized for agent scanning. Agents need to quickly understand what maproom does, when to use it vs other tools, and where to find detailed information.

Progressive disclosure pattern allows agents to get essential information immediately and dive deeper only when needed, improving both token efficiency and decision-making speed.

**References:** plan.md Phase 2, Task 3; architecture.md Decision 2

## Acceptance Criteria
- [x] New SKILL.md is under 50 lines (excluding YAML frontmatter)
- [x] Contains YAML frontmatter with name and description
- [x] Includes capability summary (2-3 sentences explaining what maproom does)
- [x] Contains when-to-use table comparing maproom vs grep vs glob
- [x] Lists 4-5 most common commands with brief descriptions
- [x] Links to all reference documents (search-best-practices.md, cli-reference.md, troubleshooting.md)
- [x] All links resolve to existing or planned files
- [x] Content is accurate for current crewchief-maproom CLI
- [x] Follows plugin conventions (YAML frontmatter format)

## Technical Requirements
- Modify `.crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search/SKILL.md`
- Keep existing YAML frontmatter structure: `name` and `description` fields
- Use markdown tables for when-to-use comparison
- Use code blocks for command examples
- Link format: `[Display Text](./references/filename.md)`
- Total line count (excluding frontmatter) must be under 50

## Implementation Notes
**Structure to follow (from architecture.md):**

```markdown
---
name: maproom-search
description: Semantic code search for exploring unfamiliar codebases and finding implementations by concept.
---

# Maproom Search

Semantic code search using SQLite FTS and optional vector embeddings.

## When to Use

| Tool | Use Case |
|------|----------|
| maproom | Find code by concept ("authentication", "error handling") |
| Grep | Exact text/regex matches |
| Glob | File path patterns |

## Quick Reference

```bash
# Check indexed repositories
crewchief-maproom status

# Full-text search
crewchief-maproom search --repo <repo> --query "<query>"

# Vector search (requires embeddings)
crewchief-maproom vector-search --repo <repo> --query "<query>"

# Get context for a chunk
crewchief-maproom context --chunk-id <id>
```

## Learn More

- [Search Best Practices](./references/search-best-practices.md) - Query patterns and strategies
- [CLI Reference](./references/cli-reference.md) - Complete command documentation
- [Troubleshooting](./references/troubleshooting.md) - Common errors and solutions
```

**Key content decisions:**
- Focus on semantic search capability (differentiates from grep/glob)
- When-to-use table helps agents make tool selection decisions
- 4 commands cover 80% of use cases: status, search, vector-search, context
- Links enable discovery of detailed info without cluttering main page

**Line count target:** 35-45 lines (excluding frontmatter) to leave buffer below 50

## Dependencies
- None (can be done independently, but references MPRSKL.2002 and MPRSKL.2003 files)

## Risk Assessment
- **Risk**: Breaking plugin loading due to YAML frontmatter format
  - **Mitigation**: Preserve existing frontmatter structure exactly; verify plugin loads after change
- **Risk**: Content inaccuracy for current CLI version
  - **Mitigation**: Test all example commands against current crewchief-maproom binary
- **Risk**: Line count exceeds 50
  - **Mitigation**: Be ruthless about brevity; move ALL detailed content to references

## Files/Packages Affected
- .crewchief/claude-code-plugins/plugins/maproom/skills/maproom-search/SKILL.md

## Deliverables Produced

Documents created in `deliverables/` directory:

- None

## Verification Notes
The verify-task agent should specifically check:

- [ ] Line count is under 50 (use `wc -l` excluding frontmatter section)
- [ ] YAML frontmatter is valid and follows plugin conventions
- [ ] All command examples are accurate for current CLI (`crewchief-maproom --help` to verify)
- [ ] When-to-use table clearly differentiates maproom from grep and glob
- [ ] All reference links are present (search-best-practices.md, cli-reference.md, troubleshooting.md)
- [ ] File paths in links are correct relative to SKILL.md location
- [ ] Content is clear, concise, and actionable
- [ ] No technical jargon without explanation
- [ ] Examples use placeholder values like `<repo>` and `<query>` appropriately

**Line count verification:**
```bash
# Exclude YAML frontmatter (between --- markers)
tail -n +$(grep -n "^---$" SKILL.md | tail -1 | cut -d: -f1) SKILL.md | wc -l
# Should be < 50
```

**Command accuracy verification:**
```bash
crewchief-maproom --help
crewchief-maproom status --help
crewchief-maproom search --help
crewchief-maproom vector-search --help
crewchief-maproom context --help
# Verify all commands and flags in SKILL.md match help output
```

## Verification Audit
<!-- Audit log maintained by verify-task agent for enterprise compliance -->
| Date | Agent | Decision | Notes |
|------|-------|----------|-------|
| 2025-12-20 | verify-task | PASS | All 9 acceptance criteria met, 35 lines (under 50 limit), commands verified against CLI |

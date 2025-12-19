# Ticket: Maproom Plugin

**Ticket ID:** PLUGIN-001
**Status:** Planning Complete
**Created:** 2025-12-15

## Summary

Create the `maproom` plugin for the crewchief marketplace at `.crewchief/claude-code-plugins/plugins/maproom/`. The plugin provides semantic code search capabilities via the crewchief-maproom CLI, teaching Claude when and how to use maproom search instead of grep/glob for conceptual code queries.

## Problem Statement

Claude Code's native tools (Grep, Glob) are optimized for exact text matching, but struggle with conceptual queries like "How does authentication work?" The crewchief-maproom CLI provides semantic code search via FTS and vector embeddings, but Claude has no way to discover this capability. This plugin bridges the gap by teaching Claude when to use semantic search and how to formulate effective queries.

## Proposed Solution

Create a Claude Code plugin following the established marketplace pattern:

```
.crewchief/claude-code-plugins/plugins/maproom/
├── .claude-plugin/
│   └── plugin.json           # Plugin metadata
├── README.md                 # User documentation
└── skills/
    └── maproom-search/
        ├── SKILL.md          # Skill instructions (decision tree, query patterns, CLI commands)
        └── references/
            └── search-best-practices.md  # 10+ query transformation examples
```

**Key Design Decisions:**
- Use CLI directly via Bash tool (not MCP protocol) for simplicity
- Default to FTS mode (always works without embeddings)
- Include clear decision tree for when to use maproom vs grep/glob
- Progressive disclosure: essentials in SKILL.md, details in references

## Relevant Agents

- ticket-planner (planning phase) - Complete
- task-creator (ticket generation) - Next
- General implementation agent (file creation)
- verify-task (verification)
- commit-task (commit)

## Deliverables

### Plugin Files (to be created)

| File | Purpose |
|------|---------|
| `.claude-plugin/plugin.json` | Plugin metadata (name, version, description, author, keywords) |
| `README.md` | User documentation (features, prerequisites, installation, usage) |
| `skills/maproom-search/SKILL.md` | Skill instructions with decision tree and CLI commands |
| `skills/maproom-search/references/search-best-practices.md` | 10+ query transformation examples |

### Acceptance Criteria

- [ ] Plugin directory structure matches specification
- [ ] plugin.json has valid name, version, description, author, repository, keywords
- [ ] README.md documents installation, features, prerequisites
- [ ] SKILL.md frontmatter has name (maproom-search) and description (<1024 chars)
- [ ] SKILL.md includes decision tree (maproom vs grep vs glob)
- [ ] SKILL.md includes query formulation patterns with examples
- [ ] SKILL.md documents CLI commands (search, status, context)
- [ ] SKILL.md includes error handling guidance
- [ ] search-best-practices.md contains 10+ query examples

## Planning Documents

- [analysis.md](planning/analysis.md) - Problem analysis and research findings
- [architecture.md](planning/architecture.md) - Solution design and component structure
- [plan.md](planning/plan.md) - Two-phase execution plan
- [quality-strategy.md](planning/quality-strategy.md) - Validation approach and checklists
- [security-review.md](planning/security-review.md) - Security assessment (low risk)

## Key Technical References

- **CLI Documentation:** `/crates/maproom/CLAUDE.md` - Authoritative CLI command reference
- **MCP Tools:** `/packages/maproom-mcp/CLAUDE.md` - Query formulation patterns
- **Epic Overview:** `/.sdd/epics/2025-12-15_crewchief-cli-skills/overview.md`
- **Research Synthesis:** `/.sdd/epics/2025-12-15_crewchief-cli-skills/analysis/research-synthesis.md`

## Tasks

See [tasks/](tasks/) for all ticket tasks (to be created by task-creator).

## Next Steps

**Recommended:** Run `/sdd:review PLUGIN-001` before creating tasks to validate planning documents.

Then: Run `/sdd:create-tasks PLUGIN-001` to generate implementation tasks.

## Notes

- This is the first ticket in the CrewChief CLI Skills epic (PLUGIN-001, PLUGIN-002, PLUGIN-003)
- Plugin will be registered in marketplace.json by PLUGIN-003 ticket
- No custom agents recommended - general implementation skills sufficient for documentation-only plugin

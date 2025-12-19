# Ticket: Worktree Plugin

**Ticket ID:** PLUGIN-002
**Status:** Planning Complete
**Created:** 2025-12-17

## Summary

Create the `worktree` plugin for the crewchief marketplace at `.crewchief/claude-code-plugins/plugins/worktree/`. The plugin provides git worktree management capabilities via the crewchief CLI, documenting the worktree lifecycle (create -> use -> work -> merge -> clean) with safety guidance.

## Problem Statement

Claude Code users have no way to discover crewchief's git worktree management capabilities. The worktree lifecycle has specific sequencing requirements (cannot merge from inside worktree, cannot delete current worktree) and safety considerations that are easy to get wrong. This plugin bridges the gap by teaching Claude when and how to manage worktrees safely.

## Proposed Solution

Create a Claude Code plugin following the established marketplace pattern:

```
.crewchief/claude-code-plugins/plugins/worktree/
├── .claude-plugin/
│   └── plugin.json           # Plugin metadata
├── README.md                 # User documentation
└── skills/
    └── worktree-management/
        └── SKILL.md          # Skill instructions (lifecycle, commands, safety)
```

**Key Design Decisions:**
- Single skill covering complete worktree lifecycle (not split by command)
- Safety-first documentation structure (warnings before command reference)
- Workflow-centric organization (what users want to do, then how)
- No separate references file (workflows are linear, not query patterns)

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
| `README.md` | User documentation (features, prerequisites, installation, usage, troubleshooting) |
| `skills/worktree-management/SKILL.md` | Skill instructions with lifecycle, safety, CLI commands, and workflows |

### Acceptance Criteria

- [ ] Plugin directory structure matches specification
- [ ] plugin.json has valid name ("worktree"), version ("0.1.0"), description, author, repository, keywords
- [ ] README.md documents installation, features, prerequisites, troubleshooting
- [ ] SKILL.md frontmatter has name ("worktree-management") and description (<1024 chars)
- [ ] SKILL.md description clearly states when to use this skill
- [ ] Worktree lifecycle documented: create -> use -> work -> merge -> clean
- [ ] All 6 CLI commands documented with correct syntax and examples:
  - `crewchief worktree create`
  - `crewchief worktree list`
  - `crewchief worktree use`
  - `crewchief worktree clean`
  - `crewchief worktree merge`
  - `crewchief worktree copy-ignored`
- [ ] Safety considerations documented (current worktree protection, merge safety, branch deletion)
- [ ] Common workflows have step-by-step examples (feature dev, experiment, cleanup)

## Planning Documents

- [analysis.md](planning/analysis.md) - Problem analysis with CLI research
- [architecture.md](planning/architecture.md) - Solution design and component structure
- [plan.md](planning/plan.md) - Two-phase execution plan
- [quality-strategy.md](planning/quality-strategy.md) - Validation approach and checklists
- [security-review.md](planning/security-review.md) - Security assessment (low risk, documentation only)

## Key Technical References

- **CLI Implementation:** `/packages/cli/src/cli/worktree.ts` - Command implementations (875 lines)
- **Worktree Service:** `/packages/cli/src/git/worktrees.ts` - Core worktree operations
- **CLI CLAUDE.md:** `/packages/cli/CLAUDE.md` - CLI package overview
- **Maproom Plugin:** `.crewchief/claude-code-plugins/plugins/maproom/` - Pattern to follow
- **Epic Summary:** `/.sdd/epics/2025-12-15_crewchief-cli-skills/decomposition/ticket-summaries/PLUGIN-002_worktree-plugin.md`

## Tasks

See [tasks/](tasks/) for all ticket tasks (to be created by task-creator).

## Next Steps

**Recommended:** Run `/sdd:review PLUGIN-002` before creating tasks to validate planning documents.

Then: Run `/sdd:create-tasks PLUGIN-002` to generate implementation tasks.

## Notes

- This is the second ticket in the CrewChief CLI Skills epic (PLUGIN-001, PLUGIN-002, PLUGIN-003)
- Plugin will be registered in marketplace.json by PLUGIN-003 ticket
- No custom agents recommended - general implementation skills sufficient for documentation-only plugin
- Follows maproom plugin pattern exactly for consistency
